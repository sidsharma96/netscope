//! Counts derived from captured entries, without inferring network relationships.
use crate::{HostId, Request, RequestId};

/// Counts of reported HTTP statuses; zero is kept outside the HTTP classes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct StatusCounts {
    /// Recorded 100–199 statuses.
    pub informational: usize,
    /// Recorded 200–299 statuses.
    pub successful: usize,
    /// Recorded 300–399 statuses.
    pub redirection: usize,
    /// Recorded 400–499 statuses.
    pub client_error: usize,
    /// Recorded 500–599 statuses.
    pub server_error: usize,
    /// Entries for which no HTTP status was recorded.
    pub not_recorded: usize,
}

/// Derived counts and request IDs for one normalised URL host.
#[derive(Debug, PartialEq)]
pub struct HostSummary {
    /// First-source-occurrence host identity.
    pub host_id: HostId,
    /// Original request IDs in chronological order, with stable source-order ties.
    pub request_ids: Vec<RequestId>,
    /// Counts of the recorded status classes.
    pub statuses: StatusCounts,
    /// Sum of reported entry durations in milliseconds; None means numeric overflow.
    /// Requests can overlap, so this is not elapsed time.
    pub summed_duration_ms: Option<f64>,
    /// Number of requests with one or more import warnings.
    pub warning_requests: usize,
}
impl HostSummary {
    pub(crate) fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            request_ids: Vec::new(),
            statuses: StatusCounts::default(),
            summed_duration_ms: Some(0.0),
            warning_requests: 0,
        }
    }
    pub(crate) fn record(&mut self, request: &Request) {
        match request.status {
            0 => self.statuses.not_recorded += 1,
            100..=199 => self.statuses.informational += 1,
            200..=299 => self.statuses.successful += 1,
            300..=399 => self.statuses.redirection += 1,
            400..=499 => self.statuses.client_error += 1,
            500..=599 => self.statuses.server_error += 1,
            _ => {}
        }
        self.warning_requests += usize::from(!request.warnings.is_empty());
        self.summed_duration_ms = self.summed_duration_ms.and_then(|sum| {
            let next = sum + request.total_ms;
            next.is_finite().then_some(next)
        });
    }
}
