//! Bounded interpretation of retained SSE text, without browser execution claims.
use crate::{Body, Request};

/// Maximum UTF-8 bytes examined per retained response.
pub const SSE_TEXT_LIMIT: usize = 64 * 1024;
/// Maximum complete data-bearing events retained for inspection.
pub const SSE_EVENT_LIMIT: usize = 100;

/// One blank-line-terminated event derived from the retained text.
#[derive(Debug, PartialEq, Eq)]
pub struct SseEvent<'a> {
    /// Last event field in this block, or the default `message`.
    pub event_type: &'a str,
    /// Joined data values with the final added newline removed.
    pub data: String,
    /// ID buffer at this boundary, initialized empty for this retained body.
    pub id: &'a str,
}
/// The reason inspection stopped before examining all retained text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseLimit {
    /// The retained body exceeds the text budget.
    Text,
    /// Another complete event exceeds the event budget.
    Events,
}
/// Results derived from a prefix of one retained body, not a live connection.
#[derive(Debug, PartialEq, Eq)]
pub struct SseStream<'a> {
    /// Complete data-bearing events in retained order.
    pub events: Vec<SseEvent<'a>>,
    /// Colon-prefixed comment lines examined.
    pub comments: usize,
    /// Unknown fields or invalid id/retry values examined.
    pub ignored: usize,
    /// Last valid retry value, as ASCII digits without integer overflow.
    pub retry_ms: Option<&'a str>,
    /// Last ID buffer from examined complete lines; no earlier connection state.
    pub last_id: &'a str,
    /// Trailing block without a complete blank-line boundary, if examined.
    pub pending: Option<&'a str>,
    /// Explicit analysis limit; absence does not prove capture completeness.
    pub limit: Option<SseLimit>,
}
/// Keep unavailable text distinct from a successfully parsed empty stream.
#[derive(Debug, PartialEq, Eq)]
pub enum SseInspection<'a> {
    /// The reported event-stream response has no usable text representation.
    Unavailable(&'static str),
    /// Interpretation of retained UTF-8 text.
    Parsed(SseStream<'a>),
}
/// Inspect only responses whose reported MIME essence is text/event-stream.
/// Parameters and ASCII letter case do not change that classification.
pub fn inspect_sse(request: &Request) -> Option<SseInspection<'_>> {
    let essence = request
        .mime_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim_matches([' ', '\t']);
    if !essence.eq_ignore_ascii_case("text/event-stream") {
        return None;
    }
    Some(match &request.body {
        Body::Text(text) | Body::DecodedText(text) => SseInspection::Parsed(parse_sse(text)),
        Body::NotRecorded => SseInspection::Unavailable("Response text not recorded"),
        Body::Binary(_) => SseInspection::Unavailable("Retained bytes are not UTF-8 text"),
        Body::Undecoded { .. } => {
            SseInspection::Unavailable("Response representation could not be decoded")
        }
    })
}
/// Parse complete lines and blank-line event boundaries within fixed budgets.
/// A single leading byte-order mark is ignored. End of text supplies no boundary.
pub fn parse_sse(text: &str) -> SseStream<'_> {
    let mut end = text.len().min(SSE_TEXT_LIMIT);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let limited = end < text.len();
    let bounded = &text[..end];
    let text = bounded.strip_prefix('\u{feff}').unwrap_or(bounded);
    let mut stream = SseStream {
        events: Vec::new(),
        comments: 0,
        ignored: 0,
        retry_ms: None,
        last_id: "",
        pending: None,
        limit: limited.then_some(SseLimit::Text),
    };
    let mut cursor = 0;
    let mut block_start = 0;
    let mut data = String::new();
    let mut event_type = "";
    while cursor < text.len() {
        let Some(length) = text[cursor..].find(['\r', '\n']) else {
            break;
        };
        let line_end = cursor + length;
        let line = &text[cursor..line_end];
        cursor = line_end + 1;
        if text.as_bytes()[line_end] == b'\r' && text.as_bytes().get(cursor) == Some(&b'\n') {
            cursor += 1;
        }
        if line.is_empty() {
            if !data.is_empty() {
                if stream.events.len() == SSE_EVENT_LIMIT {
                    stream.limit = Some(SseLimit::Events);
                    return stream;
                }
                data.pop(); // Remove only the newline appended by the last data field.
                stream.events.push(SseEvent {
                    event_type: if event_type.is_empty() {
                        "message"
                    } else {
                        event_type
                    },
                    data: std::mem::take(&mut data),
                    id: stream.last_id,
                });
            }
            event_type = "";
            block_start = cursor;
        } else if line.starts_with(':') {
            stream.comments += 1;
        } else {
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "data" => {
                    data.push_str(value);
                    data.push('\n');
                }
                "event" => event_type = value,
                "id" if !value.contains('\0') => stream.last_id = value,
                "retry" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                    stream.retry_ms = Some(value)
                }
                _ => stream.ignored += 1,
            }
        }
    }
    if block_start < text.len() {
        stream.pending = Some(&text[block_start..]);
    }
    stream
}
