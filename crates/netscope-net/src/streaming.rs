//! Bounded observations of one live event stream, independent of the imported HAR.
use crate::{Outcome, ReplayResult};
use bytes::Bytes;
use http_body_util::BodyExt;
use netscope_core::{ReplayMethod, ReplayReview, SSE_EVENT_LIMIT, SSE_TEXT_LIMIT};
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
// Count only complete data-bearing blocks; the display parser reads fields later.
#[derive(Default)]
struct Boundaries {
    scanned: usize,
    line_start: usize,
    initialized: bool,
    after_cr: bool,
    has_data: bool,
}
impl Boundaries {
    fn advance(&mut self, bytes: &[u8], observed: &mut Vec<Duration>, elapsed: Duration) {
        if !self.initialized && !bytes.is_empty() {
            self.initialized = true;
            self.scanned = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                3
            } else {
                0
            };
            self.line_start = self.scanned;
        }
        while self.scanned < bytes.len() && observed.len() < SSE_EVENT_LIMIT {
            let at = self.scanned;
            let byte = bytes[at];
            self.scanned += 1;
            if byte == b'\n' && self.after_cr {
                self.after_cr = false;
                self.line_start = self.scanned;
                continue;
            }
            self.after_cr = byte == b'\r';
            if matches!(byte, b'\r' | b'\n') {
                let line = &bytes[self.line_start..at];
                if line.is_empty() {
                    if self.has_data {
                        observed.push(elapsed);
                    }
                    self.has_data = false;
                } else if line == b"data" || line.starts_with(b"data:") {
                    self.has_data = true;
                }
                self.line_start = self.scanned;
            }
        }
    }
}
pub(crate) struct LiveCapture {
    started: Instant,
    snapshot: StreamSnapshot,
    publisher: watch::Sender<Option<StreamSnapshot>>,
    boundaries: Boundaries,
    last_publication: Duration,
    begun: bool,
    published_bytes: usize,
}
impl LiveCapture {
    pub(crate) fn new(started: Instant, publisher: watch::Sender<Option<StreamSnapshot>>) -> Self {
        Self {
            started,
            snapshot: StreamSnapshot::default(),
            publisher,
            boundaries: Boundaries::default(),
            last_publication: Duration::ZERO,
            begun: false,
            published_bytes: 0,
        }
    }
    fn publish(&mut self, elapsed: Duration) {
        self.publisher.send_replace(Some(self.snapshot.clone()));
        self.last_publication = elapsed;
        self.published_bytes = self.snapshot.body.len();
    }
    // Called after the exchange is dropped too, so cancellation retains the last bytes.
    pub(crate) fn finish(&mut self) {
        if self.begun {
            self.publish(self.started.elapsed());
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
        let valid = self.snapshot.valid_bytes;
        // Recheck only new bytes and at most three pending bytes of a split character.
        let invalid = match std::str::from_utf8(&self.snapshot.body[valid..]) {
            Ok(text) => {
                self.snapshot.valid_bytes += text.len();
                false
            }
            Err(error) => {
                self.snapshot.valid_bytes += error.valid_up_to();
                error.error_len().is_some()
            }
        };
        let previous = self.snapshot.observed.len();
        self.boundaries.advance(
            &self.snapshot.body[..self.snapshot.valid_bytes],
            &mut self.snapshot.observed,
            elapsed,
        );
        let count = self.snapshot.observed.len();
        let limited = count == SSE_EVENT_LIMIT || self.snapshot.body.len() == SSE_TEXT_LIMIT;
        // Complete events appear immediately. Pending text is coalesced between publications.
        if invalid
            || limited
            || count != previous
            || elapsed.saturating_sub(self.last_publication) >= Duration::from_millis(50)
        {
            self.publish(elapsed);
        }
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
        self.begun = true;
        self.publish(Duration::ZERO);
        loop {
            let frame = if self.published_bytes != self.snapshot.body.len() {
                tokio::select! {
                    frame = body.frame() => frame,
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        self.publish(self.started.elapsed());
                        continue;
                    }
                }
            } else {
                body.frame().await
            };
            let Some(frame) = frame else { break };
            let frame = frame.map_err(|_| "Response body ended with a read error")?;
            if let Ok(data) = frame.into_data() {
                let before = self.snapshot.body.len();
                let outcome = self.observe(&data, self.started.elapsed());
                retained.extend_from_slice(&self.snapshot.body[before..]);
                if let Some(outcome) = outcome? {
                    return Ok(outcome);
                }
            }
        }
        self.finish();
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
    #[test]
    #[ignore = "local timing probe; no speed threshold"]
    fn measure_fragment_work() -> Result<(), &'static str> {
        let body = vec![b'x'; SSE_TEXT_LIMIT];
        for size in [1, 16, 256, 4096] {
            for sample in 1..=3 {
                let (mut live, _receiver) = capture();
                let start = Instant::now();
                for part in body.chunks(size) {
                    let _ = live.observe(part, Duration::ZERO)?;
                }
                std::hint::black_box(&live.snapshot);
                println!(
                    "fragment,{size},{sample},{:.6}",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }
        Ok(())
    }
    #[test]
    fn boundary_count_matches_saved_parser_for_generated_fragmentations() -> Result<(), &'static str>
    {
        let lines = [
            "data",
            "data:",
            "data: café",
            ": comment",
            "id: x",
            "event: y",
            "retry: 3",
            "unknown",
            "",
        ];
        for seed in 0..128 {
            let mut text = String::from("\u{feff}");
            let mut state = seed + 1usize;
            for _ in 0..24 {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                text.push_str(lines[state % lines.len()]);
                text.push_str(["\r", "\n", "\r\n"][state / 7 % 3]);
            }
            for chunk in 1..=17 {
                let (mut live, _) = capture();
                let mut end = 0;
                for part in text.as_bytes().chunks(chunk) {
                    end += part.len();
                    live.observe(part, Duration::from_millis(end as u64))?;
                    let prefix =
                        std::str::from_utf8(&live.snapshot.body[..live.snapshot.valid_bytes])
                            .map_err(|_| "prefix")?;
                    assert_eq!(
                        live.snapshot.observed.len(),
                        netscope_core::parse_sse(prefix).events.len(),
                        "seed {seed}, chunk {chunk}, end {end}"
                    );
                    assert_eq!(live.boundaries.scanned, live.snapshot.valid_bytes);
                }
            }
        }
        Ok(())
    }
    #[tokio::test(start_paused = true)]
    async fn pending_publication_wakes_without_another_fragment()
    -> Result<(), Box<dyn std::error::Error>> {
        struct OpenBody {
            next: Option<Bytes>,
            ready: Option<tokio::sync::oneshot::Sender<()>>,
        }
        impl hyper::body::Body for OpenBody {
            type Data = Bytes;
            type Error = std::convert::Infallible;
            fn poll_frame(
                mut self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>>
            {
                if let Some(bytes) = self.next.take() {
                    if let Some(ready) = self.ready.take() {
                        let _ = ready.send(());
                    }
                    std::task::Poll::Ready(Some(Ok(hyper::body::Frame::data(bytes))))
                } else {
                    std::task::Poll::Pending
                }
            }
        }
        let (mut live, mut rx) = capture();
        let (ready, received) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            live.retain(
                OpenBody {
                    next: Some(Bytes::from_static(b"data: pending")),
                    ready: Some(ready),
                },
                &mut vec![],
            )
            .await
        });
        received.await?;
        assert!(
            rx.borrow_and_update()
                .as_ref()
                .ok_or("initial publication")?
                .body
                .is_empty()
        );
        tokio::time::advance(Duration::from_millis(49)).await;
        tokio::task::yield_now().await;
        assert!(!rx.has_changed()?);
        tokio::time::advance(Duration::from_millis(1)).await;
        rx.changed().await?;
        assert_eq!(
            rx.borrow().as_ref().ok_or("pending publication")?.body,
            b"data: pending"
        );
        assert!(
            rx.borrow()
                .as_ref()
                .ok_or("pending publication")?
                .observed
                .is_empty()
        );
        task.abort();
        assert!(task.await.is_err_and(|error| error.is_cancelled()));
        Ok(())
    }
}
