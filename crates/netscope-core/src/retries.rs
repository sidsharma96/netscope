use crate::comparison::{elapsed_ms, request_url};
use crate::{ReportedSize, Request, RequestBody, RequestId};
use std::collections::BTreeMap;

/// Possible repeated attempts inferred from recorded fields, never client intent.
#[derive(Debug, PartialEq, Eq)]
pub enum RetryEvidence {
    /// The source status is outside this analysis's selected error statuses.
    NotApplicable,
    /// Required evidence is unavailable or outside the supported request profile.
    Unavailable(&'static str),
    /// A completed search, including an empty result.
    Candidates {
        /// Up to twenty source IDs, ordered by timestamp then source position.
        ids: Vec<RequestId>,
        /// At least one additional candidate exists.
        more: bool,
    },
}

fn supported_request(request: &Request) -> bool {
    matches!(request.method.as_str(), "GET" | "HEAD")
        && request.request_body_size == ReportedSize::Bytes(0)
        && matches!(request.request_body, RequestBody::NotRecorded)
}

pub(crate) fn analyze(requests: &[Request], chronological: &[RequestId]) -> Vec<RetryEvidence> {
    let mut index: BTreeMap<(String, &str), Vec<RequestId>> = BTreeMap::new();
    for id in chronological {
        let request = &requests[id.0];
        if supported_request(request)
            && let Some(url) = request_url(&request.url)
        {
            index
                .entry((url.into(), &request.method))
                .or_default()
                .push(*id);
        }
    }
    requests
        .iter()
        .map(|source| {
            if source.status == 0 {
                return RetryEvidence::Unavailable(
                    "No status recorded; a network failure cannot be assumed",
                );
            }
            if !matches!(source.status, 408 | 429 | 500 | 502 | 503 | 504) {
                return RetryEvidence::NotApplicable;
            }
            if !supported_request(source) {
                return RetryEvidence::Unavailable(
                    "Requires GET or HEAD, reported body size 0, and no captured request body",
                );
            }
            if !source.total_ms.is_finite() || source.total_ms < 0.0 {
                return RetryEvidence::Unavailable(
                    "Reported duration cannot define a matching window",
                );
            }
            let Some(url) = request_url(&source.url) else {
                return RetryEvidence::Unavailable("Source URL cannot be compared conservatively");
            };
            let mut ids = Vec::new();
            let mut more = false;
            if let Some(candidates) = index.get(&(url.into(), source.method.as_str())) {
                let start =
                    candidates.partition_point(|id| requests[id.0].instant <= source.instant);
                for id in &candidates[start..] {
                    let candidate = &requests[id.0];
                    let elapsed = elapsed_ms(source, candidate);
                    if elapsed < source.total_ms {
                        continue;
                    }
                    if elapsed - source.total_ms > 30_000.0 {
                        break;
                    }
                    // Equality concerns the recorded lists, not everything sent on the wire.
                    if candidate.headers != source.headers
                        || candidate.cookies != source.cookies
                        || candidate.query != source.query
                    {
                        continue;
                    }
                    if ids.len() == 20 {
                        more = true;
                        break;
                    }
                    ids.push(*id);
                }
            }
            RetryEvidence::Candidates { ids, more }
        })
        .collect()
}
