//! Explain retained classic upgrade headers without recovering socket traffic.
use crate::{NameValue, Request};

fn values<'a>(headers: &'a [NameValue], name: &str) -> Vec<&'a str> {
    headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.trim_matches([' ', '\t']))
        .collect()
}
fn token(headers: &[NameValue], name: &str, wanted: &str) -> bool {
    values(headers, name).iter().any(|value| {
        value
            .split(',')
            .any(|v| v.trim_matches([' ', '\t']).eq_ignore_ascii_case(wanted))
    })
}
/// A limited explanation of the recorded classic HTTP/1.1 upgrade exchange.
#[derive(Debug, PartialEq, Eq)]
pub struct WebSocketEvidence {
    /// A fixed summary of status and the checked header shape.
    pub summary: &'static str,
    /// Missing or inconsistent retained metadata; never a full handshake verdict.
    pub notes: Vec<&'static str>,
}
/// Recognize WebSocket-specific fields/tokens and inspect the classic header shape.
/// Challenge digests, extensions, origin policy and later frames are not validated.
pub fn websocket_evidence(request: &Request) -> Option<WebSocketEvidence> {
    let candidate = token(&request.headers, "upgrade", "websocket")
        || token(&request.response_headers, "upgrade", "websocket")
        || request
            .headers
            .iter()
            .chain(&request.response_headers)
            .any(|h| h.name.to_ascii_lowercase().starts_with("sec-websocket-"))
        || request.url.split_once(':').is_some_and(|(scheme, _)| {
            scheme.eq_ignore_ascii_case("ws") || scheme.eq_ignore_ascii_case("wss")
        });
    if !candidate {
        return None;
    }
    if request.request_version != "HTTP/1.1" || request.response_version != "HTTP/1.1" {
        return Some(WebSocketEvidence {
            summary: "Outside classic HTTP/1.1 upgrade inspection",
            notes: vec!["Other reported versions require their own handshake interpretation"],
        });
    }
    let mut notes = Vec::new();
    if request.method != "GET" {
        notes.push("Classic upgrade expects GET");
    }
    for (headers, name, wanted, reason) in [
        (
            &request.headers,
            "upgrade",
            "websocket",
            "Request Upgrade does not contain websocket",
        ),
        (
            &request.headers,
            "connection",
            "upgrade",
            "Request Connection does not contain Upgrade",
        ),
        (
            &request.response_headers,
            "upgrade",
            "websocket",
            "Response Upgrade does not contain websocket",
        ),
        (
            &request.response_headers,
            "connection",
            "upgrade",
            "Response Connection does not contain Upgrade",
        ),
    ] {
        if !token(headers, name, wanted) {
            notes.push(reason);
        }
    }
    let keys = values(&request.headers, "sec-websocket-key");
    if keys.len() != 1 || keys[0].is_empty() {
        notes.push("Request key is missing, empty, or repeated");
    }
    if values(&request.headers, "sec-websocket-version") != ["13"] {
        notes.push("Request WebSocket version is not one recorded 13 value");
    }
    let accepts = values(&request.response_headers, "sec-websocket-accept");
    if accepts.len() != 1 || accepts[0].is_empty() {
        notes.push("Response accept value is missing, empty, or repeated");
    }
    let selected = values(&request.response_headers, "sec-websocket-protocol");
    if !selected.is_empty() {
        let offered = values(&request.headers, "sec-websocket-protocol");
        if selected.len() != 1
            || selected[0].is_empty()
            || selected[0].contains(',')
            || !offered.iter().any(|v| {
                v.split(',')
                    .any(|t| t.trim_matches([' ', '\t']) == selected[0])
            })
        {
            notes.push("Selected subprotocol is not one exact offered value");
        }
    }
    let summary = if request.status == 0 {
        "No HTTP status recorded for this upgrade metadata"
    } else if request.status != 101 {
        "Upgrade metadata present; no 101 response recorded"
    } else if notes.is_empty() {
        "Recorded 101 with expected classic upgrade header shape"
    } else {
        "Recorded 101 with incomplete or inconsistent upgrade metadata"
    };
    Some(WebSocketEvidence { summary, notes })
}
