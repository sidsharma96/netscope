//! Shared URL and time comparisons for offline relationship suggestions.
use crate::Request;
use url::Url;

pub(crate) fn request_url(raw: &str) -> Option<Url> {
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

pub(crate) fn elapsed_ms(source: &Request, candidate: &Request) -> f64 {
    // Subtract integer timestamps before conversion, preserving small intervals.
    let seconds = i128::from(candidate.instant.0) - i128::from(source.instant.0);
    let nanos =
        seconds * 1_000_000_000 + i128::from(candidate.instant.1) - i128::from(source.instant.1);
    nanos as f64 / 1_000_000.0
}
