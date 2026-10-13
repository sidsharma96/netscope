//! Bounded observations of one live event stream, independent of the imported HAR.
use crate::{Outcome, ReplayResult};
use bytes::Bytes;
use http_body_util::BodyExt;
use netscope_core::{ReplayMethod, ReplayReview, SSE_EVENT_LIMIT, SSE_TEXT_LIMIT, parse_sse};
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// One absolute deadline, including connection setup.
pub const STREAM_TIME_LIMIT: Duration = Duration::from_secs(30);
/// The latest cumulative observation replaces the previous publication.
#[derive(Debug, Clone, Default)]
pub struct StreamSnapshot {
    /// Retained raw response bytes; at most 64 KiB.
    pub body: Vec<u8>,
    /// Time since accepted Send when a complete event boundary was observed.
    pub observed: Vec<Duration>,
    /// UTF-8 prefix available for inspection; a split character can remain pending.
    pub valid_bytes: usize,
}
/// Validate the separate live mode without network I/O.
/// # Errors
/// Rejects POST and a user-provided Accept header rather than silently replacing it.
pub fn validate_stream(review: &ReplayReview) -> Result<(), &'static str> {
    crate::validate(review)?;
    if review.method != ReplayMethod::Get {
        return Err("Live SSE requires GET");
    }
    if review
        .headers
        .iter()
        .any(|h| h.name.eq_ignore_ascii_case("accept"))
    {
        return Err("Live SSE manages Accept; remove the selected Accept header first");
    }
    Ok(())
}
pub(crate) struct LiveCapture {
    started: Instant,
    snapshot: StreamSnapshot,
    publisher: watch::Sender<Option<StreamSnapshot>>,
}
impl LiveCapture {
    pub(crate) fn new(started: Instant, publisher: watch::Sender<Option<StreamSnapshot>>) -> Self {
        Self {
            started,
            snapshot: StreamSnapshot::default(),
            publisher,
        }
    }
    fn observe(
        &mut self,
        bytes: &[u8],
        elapsed: Duration,
    ) -> Result<Option<Outcome>, &'static str> {
        let remaining = SSE_TEXT_LIMIT - self.snapshot.body.len();
        self.snapshot
            .body
            .extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        let invalid = match std::str::from_utf8(&self.snapshot.body) {
            Ok(text) => {
                self.snapshot.valid_bytes = text.len();
                false
            }
            Err(error) => {
                self.snapshot.valid_bytes = error.valid_up_to();
                error.error_len().is_some()
            }
        };
        let text = std::str::from_utf8(&self.snapshot.body[..self.snapshot.valid_bytes])
            .map_err(|_| "Invalid UTF-8 prefix")?;
        let count = parse_sse(text).events.len();
        self.snapshot.observed.resize(count, elapsed);
        self.publisher.send_replace(Some(self.snapshot.clone()));
        if invalid {
            return Err("Live SSE contained invalid UTF-8; valid prefix retained");
        }
        if count == SSE_EVENT_LIMIT {
            return Ok(Some(Outcome::CaptureLimited("100 events observed")));
        }
        if self.snapshot.body.len() == SSE_TEXT_LIMIT {
            return Ok(Some(Outcome::CaptureLimited("64 KiB capture limit")));
        }
        Ok(None)
    }
    pub(crate) async fn retain(
        &mut self,
        mut body: impl hyper::body::Body<Data = Bytes> + Unpin,
        retained: &mut Vec<u8>,
    ) -> Result<Outcome, &'static str> {
        self.publisher.send_replace(Some(self.snapshot.clone()));
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| "Response body ended with a read error")?;
            if let Ok(data) = frame.into_data() {
                let outcome = self.observe(&data, self.started.elapsed());
                retained.clone_from(&self.snapshot.body);
                if let Some(outcome) = outcome? {
                    return Ok(outcome);
                }
            }
        }
        if self.snapshot.valid_bytes != self.snapshot.body.len() {
            return Err("Live SSE ended inside a UTF-8 character");
        }
        Ok(Outcome::Complete)
    }
}
pub(crate) fn validate_response(result: &ReplayResult) -> Result<(), &'static str> {
    if result.status != Some(200) {
        return Err("Live SSE requires HTTP status 200");
    }
    let types: Vec<_> = result
        .headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case("content-type"))
        .collect();
    if types.len() != 1
        || !std::str::from_utf8(&types[0].1).is_ok_and(|s| {
            s.split(';')
                .next()
                .unwrap_or_default()
                .trim_matches([' ', '\t'])
                .eq_ignore_ascii_case("text/event-stream")
        })
    {
        return Err("Live SSE requires one text/event-stream Content-Type");
    }
    if result.headers.iter().any(|(n, v)| {
        n.eq_ignore_ascii_case("content-encoding") && !v.eq_ignore_ascii_case(b"identity")
    }) {
        return Err("Live SSE does not decode compressed content");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capture() -> (LiveCapture, watch::Receiver<Option<StreamSnapshot>>) {
        let (tx, rx) = watch::channel(None);
        (LiveCapture::new(Instant::now(), tx), rx)
    }
    #[test]
    fn every_utf8_split_preserves_events_and_observation_meaning() -> Result<(), &'static str> {
        let body = "\u{feff}id: book\r\ndata: café\r\n\r\ndata: second\n\n".as_bytes();
        for cut in 0..=body.len() {
            let (mut live, rx) = capture();
            live.observe(&body[..cut], Duration::from_millis(3))?;
            let before = live.snapshot.observed.len();
            live.observe(&body[cut..], Duration::from_millis(7))?;
            let snapshot = rx.borrow().clone().ok_or("Missing publication")?;
            assert_eq!(snapshot.body, body);
            assert_eq!(snapshot.observed.len(), 2);
            assert!(
                snapshot.observed[..before]
                    .iter()
                    .all(|t| *t == Duration::from_millis(3))
            );
            assert!(
                snapshot.observed[before..]
                    .iter()
                    .all(|t| *t == Duration::from_millis(7))
            );
        }
        Ok(())
    }
    #[test]
    fn latest_publication_is_cumulative_and_limits_stop_without_waiting_for_eof()
    -> Result<(), &'static str> {
        let (mut live, rx) = capture();
        for n in 0..100 {
            let outcome = live.observe(b"data: x\n\n", Duration::from_millis(n))?;
            assert_eq!(
                outcome,
                if n == 99 {
                    Some(Outcome::CaptureLimited("100 events observed"))
                } else {
                    None
                }
            );
        }
        assert_eq!(
            rx.borrow()
                .as_ref()
                .ok_or("Missing snapshot")?
                .observed
                .len(),
            100
        );
        let (mut live, _) = capture();
        assert_eq!(
            live.observe(&vec![b'x'; SSE_TEXT_LIMIT], Duration::ZERO)?,
            Some(Outcome::CaptureLimited("64 KiB capture limit"))
        );
        assert!(live.snapshot.observed.is_empty());
        Ok(())
    }
    #[test]
    fn invalid_utf8_and_unfinished_event_do_not_invent_delivery() {
        let (mut live, _) = capture();
        assert!(
            live.observe(b"data: complete\n\ndata: pending\n", Duration::ZERO)
                .is_ok()
        );
        assert_eq!(live.snapshot.observed.len(), 1);
        assert!(live.observe(&[255], Duration::ZERO).is_err());
        assert_eq!(live.snapshot.observed.len(), 1);
    }
}
