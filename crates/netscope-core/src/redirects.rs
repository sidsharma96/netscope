use crate::{Request, RequestId};
use std::collections::BTreeMap;
use url::Url;

/// Evidence for a possible follow-up, never proof that a client followed a redirect.
#[derive(Debug, PartialEq, Eq)]
pub enum RedirectEvidence {
    /// This analysis only considers 301, 302, 303, 307, and 308.
    NotApplicable,
    /// A fixed explanation of why matching could not be attempted.
    Unavailable(&'static str),
    /// A usable Location, with zero or more possible follow-ups.
    Candidates {
        /// Resolved URL, with its fragment removed for request comparison.
        target: String,
        /// At most twenty source IDs, ordered by timestamp then source position.
        ids: Vec<RequestId>,
        /// More candidates exist than the retained twenty.
        more: bool,
    },
}

fn request_url(raw: &str) -> Option<Url> {
    // Do not silently repair whitespace or backslashes in captured addresses.
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) || raw.contains('\\') {
        return None;
    }
    let mut url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return None;
    }
    url.set_fragment(None);
    Some(url)
}

fn method_matches(source: &Request, candidate: &Request) -> bool {
    match source.status {
        303 => matches!(candidate.method.as_str(), "GET" | "HEAD"),
        301 | 302 => {
            candidate.method == source.method
                || (source.method == "POST" && candidate.method == "GET")
        }
        _ => candidate.method == source.method,
    }
}

fn elapsed_ms(source: &Request, candidate: &Request) -> f64 {
    // Subtract integer timestamps before conversion, preserving small intervals.
    let seconds = i128::from(candidate.instant.0) - i128::from(source.instant.0);
    let nanos =
        seconds * 1_000_000_000 + i128::from(candidate.instant.1) - i128::from(source.instant.1);
    nanos as f64 / 1_000_000.0
}

pub(crate) fn analyze(requests: &[Request], chronological: &[RequestId]) -> Vec<RedirectEvidence> {
    let mut by_url: BTreeMap<String, Vec<RequestId>> = BTreeMap::new();
    for id in chronological {
        if let Some(url) = request_url(&requests[id.0].url) {
            by_url.entry(url.into()).or_default().push(*id);
        }
    }
    requests
        .iter()
        .map(|source| {
            if !matches!(source.status, 301 | 302 | 303 | 307 | 308) {
                return RedirectEvidence::NotApplicable;
            }
            let mut locations = source
                .response_headers
                .iter()
                .filter(|header| header.name.eq_ignore_ascii_case("location"));
            let Some(location) = locations.next() else {
                return RedirectEvidence::Unavailable(
                    "No Location header recorded; redirectURL metadata is not used",
                );
            };
            if locations.next().is_some() {
                return RedirectEvidence::Unavailable(
                    "Multiple Location headers; no target chosen",
                );
            }
            let raw = location.value.trim_matches([' ', '\t']);
            if raw.is_empty()
                || raw.chars().any(|c| c.is_whitespace() || c.is_control())
                || raw.contains('\\')
            {
                return RedirectEvidence::Unavailable("Empty or unsupported Location value");
            }
            let Some(base) = request_url(&source.url) else {
                return RedirectEvidence::Unavailable(
                    "Source URL cannot be compared conservatively",
                );
            };
            let Some(target) = base
                .join(raw)
                .ok()
                .and_then(|url| request_url(url.as_str()))
            else {
                return RedirectEvidence::Unavailable(
                    "Location does not resolve to a supported HTTP(S) URL",
                );
            };
            if !source.total_ms.is_finite() || source.total_ms < 0.0 {
                return RedirectEvidence::Unavailable(
                    "Reported duration cannot define a matching window",
                );
            }
            let mut ids = Vec::new();
            let mut more = false;
            if let Some(candidates) = by_url.get(target.as_str()) {
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
                    if method_matches(source, candidate) {
                        if ids.len() == 20 {
                            more = true;
                            break;
                        }
                        ids.push(*id);
                    }
                }
            }
            RedirectEvidence::Candidates {
                target: target.into(),
                ids,
                more,
            }
        })
        .collect()
}
