//! Data rules shared by importers and views.
//!
//! Captures own their data; views borrow requests through stable source IDs.

use std::fmt;

/// An optional timing as reported by a capture exporter.
///
/// Missing and unavailable values must not become measured zeroes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    /// The exporter omitted the field.
    Missing,
    /// The exporter supplied the HAR sentinel value `-1`.
    Unavailable,
    /// A duration in milliseconds. The constructor accepts finite, non-negative values.
    Milliseconds(f64),
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
            Some(-1.0) => Ok(Self::Unavailable),
            Some(milliseconds) if milliseconds.is_finite() && milliseconds >= 0.0 => {
                Ok(Self::Milliseconds(milliseconds))
            }
            Some(_) => Err(InvalidTiming),
        }
    }
}

/// Beginner examples checked by rustdoc alongside the core API.
#[doc = include_str!("../../../docs/chapters/07-values-and-functions.md")]
pub mod values_lesson {}

/// Collection and ID examples, including an expected compiler rejection.
#[doc = include_str!("../../../docs/chapters/08-structs-vectors-and-ids.md")]
pub mod collections_lesson {}

/// Examples of the implemented optional timing constructor.
#[doc = include_str!("../../../docs/chapters/09-options-results-and-matching.md")]
pub mod errors_lesson {}

mod session;
pub use session::*;

mod hosts;
pub use hosts::{HostSummary, StatusCounts};

/// Beginner loop examples for counting requests and summing durations.
#[doc = include_str!("../../../docs/chapters/12-grouping-requests.md")]
pub mod grouping_lesson {}

mod redirects;
pub use redirects::RedirectEvidence;

/// Preserve the difference between an unavailable search and an empty result.
#[doc = include_str!("../../../docs/chapters/13-following-redirects.md")]
pub mod redirects_lesson {}

mod comparison;
mod retries;
pub use retries::RetryEvidence;

/// Equality and Boolean expressions in a beginner request-comparison example.
#[doc = include_str!("../../../docs/chapters/14-recognizing-repeats.md")]
pub mod retries_lesson {}

mod protocol;
pub use protocol::RecordedProtocol;

/// Borrowed text and exact alternative patterns for recognizing a label.
#[doc = include_str!("../../../docs/chapters/15-reading-protocol-labels.md")]
pub mod protocol_lesson {}

mod replay;
pub use replay::{
    BodyFormat, REQUEST_BODY_LIMIT, ReplayDraft, ReplayMethod, ReplayReview, validate_replay_body,
    validate_replay_header,
};

/// Independent owned text for an editable request draft.
#[doc = include_str!("../../../docs/chapters/16-preparing-a-request.md")]
pub mod draft_lesson {}

/// UTF-8 bytes borrowed from owned request text.
#[doc = include_str!("../../../docs/chapters/19-submitting-text.md")]
pub mod submission_lesson {}

mod saved;
pub use saved::{SAVED_FILE_LIMIT, SavedDraft, sensitive_saved_header};

/// Serialization reconstructs independently owned values.
#[doc = include_str!("../../../docs/chapters/20-remembering-a-request.md")]
pub mod persistence_lesson {}

/// Ownership and synchronization choices, with isolated atomic examples.
#[doc = include_str!("../../../docs/chapters/24-sharing-work.md")]
pub mod sharing_lesson {}

mod sse;
pub use sse::{
    SSE_EVENT_LIMIT, SSE_TEXT_LIMIT, SseEvent, SseInspection, SseLimit, SseStream, inspect_sse,
    parse_sse,
};

/// Captured event boundaries and joined data in a beginner example.
#[doc = include_str!("../../../docs/chapters/25-reading-event-streams.md")]
pub mod sse_lesson {}
