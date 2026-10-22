//! Data rules shared by importers and views.
//!
//! Captures own their data; views borrow requests through stable source IDs.

use std::fmt;

/// An optional timing as reported by a capture exporter.
///
/// Missing and not-applicable values must not become measured zeroes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    /// The exporter omitted the field.
    Missing,
    /// The exporter supplied `-1`. HAR 1.2 defines this for an optional
    /// timing as "does not apply to the current request", for example DNS
    /// and connect on a reused connection. It is not "value unknown".
    NotApplicable,
    /// A duration in milliseconds. The constructor accepts finite, non-negative values.
    Milliseconds(f64),
    /// A retained nonstandard exporter value, not a measured duration or -1 sentinel.
    ExporterNegative(f64),
}

/// The supplied timing was neither unavailable nor a valid duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTiming;

impl fmt::Display for InvalidTiming {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected -1 or a finite non-negative duration")
    }
}

impl std::error::Error for InvalidTiming {}

impl Timing {
    /// Interpret an optional numeric HAR field without losing absence.
    ///
    /// This validates an optional field such as DNS or connect. Required
    /// send, wait, and receive fields need stricter importer validation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidTiming`] for other negative values, NaN, or infinity.
    /// The importer is responsible for rejecting JSON null and wrong types.
    ///
    /// ```
    /// use netscope_core::Timing;
    /// assert_eq!(Timing::from_optional_har(None), Ok(Timing::Missing));
    /// assert_eq!(Timing::from_optional_har(Some(0.0)), Ok(Timing::Milliseconds(0.0)));
    /// ```
    pub fn from_optional_har(value: Option<f64>) -> Result<Self, InvalidTiming> {
        match value {
            None => Ok(Self::Missing),
            Some(-1.0) => Ok(Self::NotApplicable),
            Some(milliseconds) if milliseconds.is_finite() && milliseconds >= 0.0 => {
                Ok(Self::Milliseconds(milliseconds))
            }
            Some(_) => Err(InvalidTiming),
        }
    }
}

mod session;
pub use session::*;

mod hosts;
pub use hosts::{HostSummary, StatusCounts};

mod redirects;
pub use redirects::RedirectEvidence;

mod comparison;
mod retries;
pub use retries::RetryEvidence;

mod protocol;
pub use protocol::RecordedProtocol;

mod replay;
pub use replay::{
    BodyFormat, REQUEST_BODY_LIMIT, ReplayDraft, ReplayMethod, ReplayReview, validate_replay_body,
    validate_replay_header,
};

mod saved;
pub use saved::{SAVED_FILE_LIMIT, SavedDraft, sensitive_saved_header};

mod sse;
pub use sse::{
    SSE_EVENT_LIMIT, SSE_TEXT_LIMIT, SseEvent, SseInspection, SseLimit, SseStream, inspect_sse,
    parse_sse,
};

mod websocket;
pub use websocket::{WebSocketEvidence, websocket_evidence};

/// Recorded packet layers, independent of HAR sessions.
pub mod packets;

/// Bounded TCP sequence-space observations with explicit holes and conflicts.
pub mod reassembly;
/// Plaintext HTTP/1.1 framing candidates from contiguous recorded bytes.
pub mod wire_http;

/// Bounded UDP application metadata recognition.
pub mod datagrams;
