//! Conservative HTTP/1.1 framing candidates from a uniquely observed TCP prefix.
use std::ops::Range;
type Fields = Vec<(String, Vec<u8>)>;

/// One recorded chunk's size line and data boundaries in the inspected prefix.
#[derive(Debug)]
pub struct WireChunk {
    /// The raw size/extension line, excluding CRLF.
    pub line: Range<usize>,
    /// Body bytes declared by this chunk; zero chunk has an empty range.
    pub data: Range<usize>,
}
/// A header-led interpretation, not proof of receiver acceptance or request pairing.
#[derive(Debug)]
pub struct WireMessage {
    /// Start line decoded as ASCII.
    pub start_line: String,
    /// Ordered header values; raw values can contain non-UTF-8 bytes.
    pub headers: Vec<(String, Vec<u8>)>,
    /// Parsed framing description, including unresolved boundaries.
    pub framing: &'static str,
    /// Start of this message in the inspected prefix.
    pub start: usize,
    /// End when explicitly framed, otherwise the examined prefix end.
    pub end: usize,
    /// Whether the supported framing terminator is present.
    pub framed: bool,
    /// Explicit wire chunk boundaries; never fabricated from HAR body slices.
    pub chunks: Vec<WireChunk>,
    /// Ordered trailer fields from a completed chunked message.
    pub trailers: Vec<(String, Vec<u8>)>,
    /// Bounded application bytes after supported chunk framing is removed.
    pub body: Vec<u8>,
    /// Rule-based SDP recognition, without a media/session claim.
    pub sdp: Option<&'static str>,
}
/// Framing candidates and the reason inspection stopped.
#[derive(Debug)]
pub struct WireInspection {
    /// At most eight candidates beginning at the observed prefix start.
    pub messages: Vec<WireMessage>,
    /// Unsupported, missing, ambiguous, or budget-limited continuation.
    pub stopped: Option<&'static str>,
}
fn line(bytes: &[u8], at: usize) -> Option<(Range<usize>, usize)> {
    let rest = bytes.get(at..)?;
    let n = rest.windows(2).position(|p| p == b"\r\n")?;
    if n > 4096 {
        return None;
    }
    Some((at..at + n, at + n + 2))
}
fn fields(bytes: &[u8], mut at: usize) -> Result<(Fields, usize), &'static str> {
    let start = at;
    let mut headers = vec![];
    loop {
        if at - start > 32768 {
            return Err("Header/trailer budget exceeded");
        }
        let (range, next) =
            line(bytes, at).ok_or("Header/trailer line unavailable or exceeds 4 KiB")?;
        if next - start > 32768 {
            return Err("Header/trailer budget exceeded");
        }
        if range.is_empty() {
            return Ok((headers, next));
        }
        if headers.len() == 100 {
            return Err("More than 100 header/trailer fields");
        }
        let data = &bytes[range];
        let split = data
            .iter()
            .position(|b| *b == b':')
            .ok_or("Invalid header/trailer field")?;
        if split == 0
            || !data[..split]
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(b))
        {
            return Err("Invalid header/trailer name");
        }
        let value = &data[split + 1..];
        if value.iter().any(|b| (*b < 32 && *b != b'\t') || *b == 127) {
            return Err("Invalid header/trailer control byte");
        }
        headers.push((
            String::from_utf8(data[..split].to_vec()).map_err(|_| "Non-ASCII header name")?,
            value.to_vec(),
        ));
        at = next;
    }
}
fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t'))
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t'))
        .map_or(start, |n| n + 1);
    &bytes[start..end]
}
fn values<'a>(headers: &'a [(String, Vec<u8>)], name: &str) -> Vec<&'a [u8]> {
    headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| trim(v))
        .collect()
}
fn sdp(headers: &[(String, Vec<u8>)], body: &[u8]) -> Option<&'static str> {
    let types = values(headers, "content-type");
    if types.len() != 1
        || !trim(types[0].split(|b| *b == b';').next()?).eq_ignore_ascii_case(b"application/sdp")
    {
        return None;
    }
    let text = std::str::from_utf8(body).ok()?;
    let lines: Vec<_> = text.lines().collect();
    if lines.first() != Some(&"v=0")
        || !["o=", "s=", "t=", "m="]
            .iter()
            .all(|prefix| lines.iter().any(|s| s.starts_with(prefix)))
    {
        return None;
    }
    Some(
        if lines.iter().any(|s| s.starts_with("a=ice-ufrag:"))
            && lines.iter().any(|s| s.starts_with("a=fingerprint:"))
        {
            "SDP-like text with ICE username and DTLS fingerprint fields; not proof of WebRTC/media"
        } else {
            "SDP-like session description; not validated signaling or media"
        },
    )
}
/// Inspect only a uniquely observed prefix, stopping at unknown or ambiguous framing.
/// The caller must never concatenate across gaps or conflicting bytes.
pub fn inspect_http(bytes: &[u8], observed_end: bool) -> WireInspection {
    if bytes.is_empty() {
        return WireInspection {
            messages: vec![],
            stopped: Some("No uniquely observed application bytes at prefix origin"),
        };
    }
    let limited = bytes.len() > crate::reassembly::STREAM_LIMIT;
    let bytes = &bytes[..bytes.len().min(crate::reassembly::STREAM_LIMIT)];
    let observed_end = observed_end && !limited;
    let mut output = WireInspection {
        messages: vec![],
        stopped: None,
    };
    let mut at = 0;
    while at < bytes.len() {
        if output.messages.len() == 8 {
            output.stopped = Some("Eight-message inspection limit");
            break;
        }
        match message(bytes, at, observed_end) {
            Ok(parsed) => {
                let next = parsed.end;
                let framed = parsed.framed;
                output.messages.push(parsed);
                if !framed {
                    output.stopped = Some("Message boundary unresolved; no later message guessed");
                    break;
                }
                if next <= at {
                    output.stopped = Some("No supported framing progress");
                    break;
                }
                at = next;
            }
            Err(reason) => {
                output.stopped = Some(reason);
                break;
            }
        }
    }
    if limited && output.stopped.is_none() {
        output.stopped = Some("256 KiB prefix inspection limit");
    }
    output
}
fn message(bytes: &[u8], at: usize, observed_end: bool) -> Result<WireMessage, &'static str> {
    let (first, mut position) =
        line(bytes, at).ok_or("No complete HTTP/1.1 start line in prefix")?;
    let start_line =
        std::str::from_utf8(&bytes[first]).map_err(|_| "Start line is not ASCII HTTP/1.1")?;
    if !start_line.is_ascii() || start_line.bytes().any(|b| b < 32 || b == 127) {
        return Err("Invalid start-line controls");
    }
    let parts: Vec<_> = start_line.split(' ').collect();
    let response = parts.first() == Some(&"HTTP/1.1");
    let status = if response {
        parts
            .get(1)
            .filter(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u16>().ok())
    } else {
        None
    };
    if response {
        if parts.len() < 3 || !status.is_some_and(|n| (100..=599).contains(&n)) {
            return Err("Unsupported HTTP/1.1 response start");
        }
    } else if parts.len() != 3
        || !matches!(parts[0], "GET" | "POST")
        || parts[1].is_empty()
        || parts[2] != "HTTP/1.1"
    {
        return Err("No supported GET/POST or HTTP/1.1 response start at prefix");
    }
    let (headers, head_end) = fields(bytes, position)?;
    position = head_end;
    let mut result = WireMessage {
        start_line: start_line.into(),
        headers,
        framing: "Unresolved",
        start: at,
        end: bytes.len(),
        framed: false,
        chunks: vec![],
        trailers: vec![],
        body: vec![],
        sdp: None,
    };
    let transfer = values(&result.headers, "transfer-encoding");
    let lengths = values(&result.headers, "content-length");
    if !transfer.is_empty() && !lengths.is_empty() {
        result.framing = "Ambiguous Transfer-Encoding with Content-Length";
        return Ok(result);
    }
    if status.is_some_and(|n| (100..200).contains(&n) || n == 204 || n == 304) {
        result.end = head_end;
        result.framed = true;
        result.framing = "Status-defined header-only boundary";
        // An upgrade changes the protocol, so later bytes are not another HTTP message.
        if status == Some(101) {
            result.framed = false;
            result.framing = "101 changes protocol; later HTTP framing unavailable";
        }
        return Ok(result);
    }
    if !transfer.is_empty() {
        if transfer.len() != 1 || !transfer[0].eq_ignore_ascii_case(b"chunked") {
            result.framing = "Transfer coding outside supported single chunked subset";
            return Ok(result);
        }
        result.framing = "Chunked; waiting for complete terminator";
        loop {
            if result.chunks.len() == 1024 {
                result.framing = "1024-chunk inspection limit";
                return Ok(result);
            }
            let Some((range, next)) = line(bytes, position) else {
                return Ok(result);
            };
            let raw = &bytes[range.clone()];
            let size = raw
                .split(|b| *b == b';')
                .next()
                .ok_or("Missing chunk size")?;
            if size.is_empty()
                || size.len() > 16
                || !size.iter().all(u8::is_ascii_hexdigit)
                || raw.iter().any(|b| *b < 32 || *b > 126)
            {
                result.framing = "Malformed chunk size/extension";
                return Ok(result);
            }
            let count = usize::from_str_radix(
                std::str::from_utf8(size).map_err(|_| "Invalid size bytes")?,
                16,
            )
            .ok()
            .filter(|n| *n <= crate::reassembly::STREAM_LIMIT);
            let Some(count) = count else {
                result.framing = "Chunk size exceeds supported span";
                return Ok(result);
            };
            position = next;
            if count == 0 {
                result.chunks.push(WireChunk {
                    line: range,
                    data: position..position,
                });
                match fields(bytes, position) {
                    Ok((trailers, end)) => {
                        result.trailers = trailers;
                        result.end = end;
                        result.framed = true;
                        result.framing = "Chunked terminator and trailers observed";
                    }
                    Err(_) => {
                        result.framing = "Zero chunk observed; trailers incomplete or invalid"
                    }
                }
                break;
            }
            let end = position.checked_add(count).ok_or("Chunk size overflow")?;
            if end + 2 > bytes.len() {
                return Ok(result);
            }
            if bytes.get(end..end + 2) != Some(b"\r\n") {
                result.framing = "Chunk data terminator invalid";
                return Ok(result);
            }
            result.chunks.push(WireChunk {
                line: range,
                data: position..end,
            });
            result.body.extend_from_slice(&bytes[position..end]);
            position = end + 2;
        }
    } else if !lengths.is_empty() {
        if lengths.len() != 1 || lengths[0].is_empty() || !lengths[0].iter().all(u8::is_ascii_digit)
        {
            result.framing = "Content-Length outside supported single decimal subset";
            return Ok(result);
        }
        let count = std::str::from_utf8(lengths[0])
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n <= crate::reassembly::STREAM_LIMIT);
        let Some(count) = count else {
            result.framing = "Content-Length exceeds supported span";
            return Ok(result);
        };
        let end = head_end + count;
        result
            .body
            .extend_from_slice(&bytes[head_end..bytes.len().min(end)]);
        result.framing = "Content-Length; waiting for declared bytes";
        if end <= bytes.len() {
            result.end = end;
            result.framed = true;
            result.framing = "Content-Length boundary observed";
        }
    } else if !response {
        result.framing = "Request without transfer framing has no body";
        result.end = head_end;
        result.framed = true;
    } else {
        result.body.extend_from_slice(&bytes[head_end..]);
        result.framing = "Close-delimited response; observed end unavailable";
        if observed_end {
            result.framing = "Close-delimited response with unambiguous observed SYN/FIN span";
            result.framed = true;
        }
    }
    let encoding = values(&result.headers, "content-encoding");
    if result.framed && encoding.iter().all(|s| s.eq_ignore_ascii_case(b"identity")) {
        result.sdp = sdp(&result.headers, &result.body);
    }
    Ok(result)
}
