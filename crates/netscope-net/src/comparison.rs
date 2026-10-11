//! Compare retained observations without claiming identical requests or complete captures.
use crate::{Outcome, ReplayResult};
use netscope_core::{Body, Request};
use std::collections::BTreeMap;

/// What can be established about the available body representations.
#[derive(Debug, PartialEq, Eq)]
pub enum BodyComparison {
    /// Equal stored bytes; the capture does not establish its own completeness.
    Equal(usize),
    /// A zero-based byte offset where available representations differ.
    DifferentByte(usize),
    /// Available lengths differ after a matching shared prefix.
    DifferentLengths,
    /// An incomplete replay matches this many available bytes only.
    MatchingPrefix(usize),
    /// An incomplete replay differs within the shared prefix.
    DifferentPrefix(usize),
    /// An incomplete replay has no bytes in common to compare.
    NoOverlap,
    /// Representation or response metadata needed for comparison is unavailable.
    Unavailable(&'static str),
}
impl BodyComparison {
    /// A precise display label, including the limits of the conclusion.
    pub fn label(&self) -> String {
        match self {
            Self::Equal(n) => format!("Stored bytes match ({n}); capture completeness unproven"),
            Self::DifferentByte(n) => {
                format!("Stored bytes differ at byte {n}")
            }
            Self::DifferentLengths => "Stored lengths differ; shared bytes match".into(),
            Self::MatchingPrefix(n) => {
                format!("Partial replay: shared prefix matches ({n} bytes only)")
            }
            Self::DifferentPrefix(n) => {
                format!("Partial replay: prefix differs at byte {n}")
            }
            Self::NoOverlap => "Partial replay: no shared bytes to compare".into(),
            Self::Unavailable(reason) => format!("Body comparison unavailable: {reason}"),
        }
    }
}
/// Borrow reconstructable stored bytes without copying body data.
pub fn captured_bytes(body: &Body) -> Option<&[u8]> {
    match body {
        Body::Text(s) | Body::DecodedText(s) => Some(s.as_bytes()),
        Body::Binary(bytes) => Some(bytes),
        Body::NotRecorded | Body::Undecoded { .. } => None,
    }
}
fn encoded(name: &str, value: &[u8]) -> bool {
    name.eq_ignore_ascii_case("content-encoding")
        && !value.trim_ascii().eq_ignore_ascii_case(b"identity")
}
/// Compare full retained bytes, separately from any shortened display preview.
pub fn compare_body(captured: &Request, replay: &ReplayResult) -> BodyComparison {
    if replay.status.is_none() {
        return BodyComparison::Unavailable("no replay response headers");
    }
    let Some(left) = captured_bytes(&captured.body) else {
        return BodyComparison::Unavailable("capture body missing or undecoded");
    };
    if captured
        .response_headers
        .iter()
        .any(|h| encoded(&h.name, h.value.as_bytes()))
        || replay.headers.iter().any(|(n, v)| encoded(n, v))
    {
        return BodyComparison::Unavailable(
            "content encoding prevents a common byte representation",
        );
    }
    let difference = left.iter().zip(&replay.body).position(|(a, b)| a != b);
    if replay.outcome != Outcome::Complete {
        return match difference {
            Some(offset) => BodyComparison::DifferentPrefix(offset),
            None if left.is_empty() || replay.body.is_empty() => BodyComparison::NoOverlap,
            None => BodyComparison::MatchingPrefix(left.len().min(replay.body.len())),
        };
    }
    match difference {
        Some(offset) => BodyComparison::DifferentByte(offset),
        None if left.len() != replay.body.len() => BodyComparison::DifferentLengths,
        None => BodyComparison::Equal(left.len()),
    }
}
/// A header name paired with ordered values borrowed from each observation.
#[derive(Debug)]
pub struct HeaderComparison<'a> {
    /// ASCII lowercase name used for grouping and deterministic ordering.
    pub name: String,
    /// Values retained in the capture, including duplicates.
    pub captured: Vec<&'a [u8]>,
    /// Values received by replay, including duplicates.
    pub replayed: Vec<&'a [u8]>,
    /// Whether both ordered value lists match, or no replay headers arrived.
    pub equal: Option<bool>,
}
/// Group names without merging, sorting, or decoding their values.
pub fn compare_headers<'a>(
    captured: &'a Request,
    replay: &'a ReplayResult,
) -> Vec<HeaderComparison<'a>> {
    let mut rows: BTreeMap<String, HeaderComparison<'a>> = BTreeMap::new();
    for (name, value, left) in captured
        .response_headers
        .iter()
        .map(|h| (h.name.as_str(), h.value.as_bytes(), true))
        .chain(
            replay
                .headers
                .iter()
                .map(|(n, v)| (n.as_str(), v.as_slice(), false)),
        )
    {
        let name = name.to_ascii_lowercase();
        let row = rows
            .entry(name.clone())
            .or_insert_with(|| HeaderComparison {
                name,
                captured: vec![],
                replayed: vec![],
                equal: None,
            });
        if left {
            row.captured.push(value);
        } else {
            row.replayed.push(value);
        }
    }
    rows.into_values()
        .map(|mut row| {
            row.equal = replay.status.map(|_| row.captured == row.replayed);
            row
        })
        .collect()
}
