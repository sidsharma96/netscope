//! Offline HAR validation. Captured values never become error messages.
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::DateTime;
use netscope_core::*;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::{fmt, io::Read, path::Path};

/// Explicit import budgets, checked before session or decoded-body allocation.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum input bytes.
    pub input_bytes: usize,
    /// Maximum source entries.
    pub entries: usize,
    /// Maximum decoded bytes per response body.
    pub decoded_body_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 32 * 1024 * 1024,
            entries: 10_000,
            decoded_body_bytes: 2 * 1024 * 1024,
        }
    }
}
/// An import failure with a safe field path and fixed explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError {
    /// Consumed field path, or input for read and JSON failures.
    pub path: String,
    /// Explanation without captured values.
    pub reason: String,
}
impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.reason)
    }
}
impl std::error::Error for ImportError {}
type Result<T> = std::result::Result<T, ImportError>;
fn error(path: &str, reason: &str) -> ImportError {
    ImportError {
        path: path.into(),
        reason: reason.into(),
    }
}

// Deserialize each object ourselves: Value alone would discard duplicate keys.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON data")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate object member"));
                    }
                    let Unique(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
/// Read a local file without network access, enforcing the limit during reading.
/// # Errors
/// Returns a safe read, limit, JSON, or consumed-field error.
pub fn import_file(path: &Path, limits: Limits) -> Result<Session> {
    let file = std::fs::File::open(path).map_err(|_| error("input", "could not open file"))?;
    import_reader(file, limits)
}
/// Read a bounded stream and validate the whole capture before returning a session.
/// # Errors
/// Rejects oversized input, read failures, and invalid HAR fields.
pub fn import_reader(reader: impl Read, limits: Limits) -> Result<Session> {
    let bound = u64::try_from(limits.input_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(bound)
        .read_to_end(&mut bytes)
        .map_err(|_| error("input", "could not read file"))?;
    if bytes.len() > limits.input_bytes {
        return Err(error("input", "input byte limit exceeded"));
    }
    import_bytes(&bytes, limits)
}
/// Validate UTF-8 JSON and the M1 inspection profile.
/// # Errors
/// Rejects duplicate keys, malformed JSON, limits, and invalid consumed fields.
pub fn import_bytes(bytes: &[u8], limits: Limits) -> Result<Session> {
    if bytes.len() > limits.input_bytes {
        return Err(error("input", "input byte limit exceeded"));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    let Unique(root) = Unique::deserialize(&mut parser).map_err(|e| {
        error(
            "input",
            if e.to_string().starts_with("duplicate object member") {
                "duplicate JSON object member"
            } else {
                "invalid JSON or parser depth limit exceeded"
            },
        )
    })?;
    parser
        .end()
        .map_err(|_| error("input", "trailing data after JSON"))?;
    object(&root, "root")?;
    let log = field(&root, "log", "log")?;
    object(log, "log")?;
    if string(field(log, "version", "log.version")?, "log.version")? != "1.2" {
        return Err(error("log.version", "expected HAR version 1.2"));
    }
    let creator = field(log, "creator", "log.creator")?;
    object(creator, "log.creator")?;
    required_string(creator, "name", "log.creator")?;
    required_string(creator, "version", "log.creator")?;
    let entries = array(field(log, "entries", "log.entries")?, "log.entries")?;
    if entries.len() > limits.entries {
        return Err(error("log.entries", "entry limit exceeded"));
    }
    let mut requests = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        requests.push(parse_entry(
            entry,
            &format!("log.entries[{index}]"),
            limits,
        )?);
    }
    Ok(Session::new(requests))
}
fn object<'a>(v: &'a Value, p: &str) -> Result<&'a Map<String, Value>> {
    v.as_object().ok_or_else(|| error(p, "expected object"))
}
fn array<'a>(v: &'a Value, p: &str) -> Result<&'a Vec<Value>> {
    v.as_array().ok_or_else(|| error(p, "expected array"))
}
fn field<'a>(v: &'a Value, key: &str, p: &str) -> Result<&'a Value> {
    v.get(key).ok_or_else(|| error(p, "required field missing"))
}
fn string(v: &Value, p: &str) -> Result<String> {
    v.as_str()
        .map(str::to_owned)
        .ok_or_else(|| error(p, "expected string"))
}
fn required_string(v: &Value, key: &str, p: &str) -> Result<String> {
    let p = format!("{p}.{key}");
    string(field(v, key, &p)?, &p)
}
fn optional_string(v: &Value, key: &str, p: &str) -> Result<Option<String>> {
    v.get(key)
        .map(|x| string(x, &format!("{p}.{key}")))
        .transpose()
}
fn optional_bool(v: &Value, key: &str, p: &str) -> Result<Option<bool>> {
    v.get(key)
        .map(|x| {
            x.as_bool()
                .ok_or_else(|| error(&format!("{p}.{key}"), "expected boolean"))
        })
        .transpose()
}
fn nonempty(v: &Value, key: &str, p: &str) -> Result<String> {
    let s = required_string(v, key, p)?;
    if s.is_empty() {
        Err(error(&format!("{p}.{key}"), "expected nonempty string"))
    } else {
        Ok(s)
    }
}
fn timestamp(s: &str, p: &str) -> Result<(i64, u32)> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| (t.timestamp(), t.timestamp_subsec_nanos()))
        .map_err(|_| error(p, "expected timestamp with explicit UTC offset"))
}
fn duration(v: &Value, p: &str) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| error(p, "expected finite non-negative duration"))
}
fn unsigned(v: &Value, p: &str) -> Result<u64> {
    v.as_u64()
        .ok_or_else(|| error(p, "expected non-negative integer"))
}
fn size(v: &Value, key: &str, p: &str) -> Result<ReportedSize> {
    let p = format!("{p}.{key}");
    match v.get(key) {
        None => Ok(ReportedSize::Missing),
        Some(x) if x.as_i64() == Some(-1) => Ok(ReportedSize::Unavailable),
        Some(x) => unsigned(x, &p).map(ReportedSize::Bytes),
    }
}
fn pairs(v: &Value, key: &str, p: &str) -> Result<Vec<NameValue>> {
    let p = format!("{p}.{key}");
    array(field(v, key, &p)?, &p)?
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let p = format!("{p}[{i}]");
            object(x, &p)?;
            Ok(NameValue {
                name: required_string(x, "name", &p)?,
                value: required_string(x, "value", &p)?,
            })
        })
        .collect()
}
fn cookies(v: &Value, p: &str) -> Result<Vec<Cookie>> {
    let p = format!("{p}.cookies");
    array(field(v, "cookies", &p)?, &p)?
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let p = format!("{p}[{i}]");
            object(x, &p)?;
            let pair = NameValue {
                name: required_string(x, "name", &p)?,
                value: required_string(x, "value", &p)?,
            };
            let path = optional_string(x, "path", &p)?;
            let domain = optional_string(x, "domain", &p)?;
            let expires = optional_string(x, "expires", &p)?;
            if let Some(s) = &expires {
                timestamp(s, &format!("{p}.expires"))?;
            }
            Ok(Cookie {
                pair,
                path,
                domain,
                expires,
                http_only: optional_bool(x, "httpOnly", &p)?,
                secure: optional_bool(x, "secure", &p)?,
            })
        })
        .collect()
}
fn request_body(v: &Value, p: &str) -> Result<RequestBody> {
    let Some(v) = v.get("postData") else {
        return Ok(RequestBody::NotRecorded);
    };
    let p = format!("{p}.postData");
    object(v, &p)?;
    let mime_type = required_string(v, "mimeType", &p)?;
    match (v.get("text"), v.get("params")) {
        (Some(text), None) => Ok(RequestBody::Text {
            mime_type,
            text: string(text, &format!("{p}.text"))?,
        }),
        (None, Some(params)) => {
            let params = array(params, &format!("{p}.params"))?
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let p = format!("{p}.params[{i}]");
                    object(x, &p)?;
                    Ok(Parameter {
                        name: required_string(x, "name", &p)?,
                        value: optional_string(x, "value", &p)?,
                        file_name: optional_string(x, "fileName", &p)?,
                        content_type: optional_string(x, "contentType", &p)?,
                    })
                })
                .collect::<Result<_>>()?;
            Ok(RequestBody::Params { mime_type, params })
        }
        _ => Err(error(&p, "expected exactly one of text or params")),
    }
}
fn response_body(v: &Value, p: &str, limit: usize) -> Result<Body> {
    let text = optional_string(v, "text", p)?;
    let encoding = optional_string(v, "encoding", p)?;
    let Some(text) = text else {
        return Ok(Body::NotRecorded);
    };
    let Some(encoding) = encoding else {
        return Ok(Body::Text(text));
    };
    let failure = |reason: &str| Body::Undecoded {
        encoded: text.clone(),
        reason: reason.into(),
    };
    if encoding != "base64" {
        return Ok(failure("unsupported body encoding"));
    }
    // Even invalid encodings remain bounded; do not allocate an oversized decode.
    let padding = text.bytes().rev().take_while(|b| *b == b'=').count().min(2);
    let estimated = text.len() / 4 * 3 + (text.len() % 4) * 3 / 4;
    if estimated.saturating_sub(padding) > limit {
        return Ok(failure("decoded body limit exceeded"));
    }
    match STANDARD.decode(&text) {
        Err(_) => Ok(failure("malformed base64")),
        Ok(bytes) if bytes.len() > limit => Ok(failure("decoded body limit exceeded")),
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(Body::DecodedText(s)),
            Err(e) => Ok(Body::Binary(e.into_bytes())),
        },
    }
}
fn optional_timing(v: &Value, key: &str, p: &str) -> Result<Timing> {
    let p = format!("{p}.{key}");
    let value = v
        .get(key)
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| error(&p, "expected numeric timing"))
        })
        .transpose()?;
    Timing::from_optional_har(value)
        .map_err(|_| error(&p, "expected -1 or finite non-negative duration"))
}
fn parse_entry(v: &Value, p: &str, limits: Limits) -> Result<Request> {
    object(v, p)?;
    let started_at = required_string(v, "startedDateTime", p)?;
    let instant = timestamp(&started_at, &format!("{p}.startedDateTime"))?;
    let total_ms = duration(
        field(v, "time", &format!("{p}.time"))?,
        &format!("{p}.time"),
    )?;
    let req = field(v, "request", &format!("{p}.request"))?;
    let rp = format!("{p}.request");
    object(req, &rp)?;
    let method = required_string(req, "method", &rp)?;
    if method.is_empty()
        || !method
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
    {
        return Err(error(&format!("{rp}.method"), "expected HTTP method token"));
    }
    let url = required_string(req, "url", &rp)?;
    let parsed = url::Url::parse(&url).map_err(|_| {
        error(
            &format!("{rp}.url"),
            "expected absolute HTTP(S) URL with host",
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !url.contains("://")
        || url.chars().any(|c| c.is_control())
    {
        return Err(error(
            &format!("{rp}.url"),
            "expected absolute HTTP(S) URL with host",
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| error(&format!("{rp}.url"), "expected host"))?
        .to_owned();
    let request_version = nonempty(req, "httpVersion", &rp)?;
    let headers = pairs(req, "headers", &rp)?;
    let query = pairs(req, "queryString", &rp)?;
    let request_cookies = cookies(req, &rp)?;
    let request_header_size = size(req, "headersSize", &rp)?;
    let request_body_size = size(req, "bodySize", &rp)?;
    let request_body = request_body(req, &rp)?;
    let res = field(v, "response", &format!("{p}.response"))?;
    let sp = format!("{p}.response");
    object(res, &sp)?;
    let status = unsigned(
        field(res, "status", &format!("{sp}.status"))?,
        &format!("{sp}.status"),
    )?;
    if status != 0 && !(100..=599).contains(&status) {
        return Err(error(
            &format!("{sp}.status"),
            "expected zero or HTTP status 100 through 599",
        ));
    }
    let status_text = required_string(res, "statusText", &sp)?;
    let response_version = nonempty(res, "httpVersion", &sp)?;
    let response_headers = pairs(res, "headers", &sp)?;
    let response_cookies = cookies(res, &sp)?;
    let response_header_size = size(res, "headersSize", &sp)?;
    let response_body_size = size(res, "bodySize", &sp)?;
    let content = field(res, "content", &format!("{sp}.content"))?;
    let cp = format!("{sp}.content");
    object(content, &cp)?;
    let content_size = unsigned(
        field(content, "size", &format!("{cp}.size"))?,
        &format!("{cp}.size"),
    )?;
    let mime_type = required_string(content, "mimeType", &cp)?;
    let body = response_body(content, &cp, limits.decoded_body_bytes)?;
    let t = field(v, "timings", &format!("{p}.timings"))?;
    let tp = format!("{p}.timings");
    object(t, &tp)?;
    let timings = Timings {
        blocked: optional_timing(t, "blocked", &tp)?,
        dns: optional_timing(t, "dns", &tp)?,
        connect: optional_timing(t, "connect", &tp)?,
        ssl: optional_timing(t, "ssl", &tp)?,
        send: duration(
            field(t, "send", &format!("{tp}.send"))?,
            &format!("{tp}.send"),
        )?,
        wait: duration(
            field(t, "wait", &format!("{tp}.wait"))?,
            &format!("{tp}.wait"),
        )?,
        receive: duration(
            field(t, "receive", &format!("{tp}.receive"))?,
            &format!("{tp}.receive"),
        )?,
    };
    let server_ip = optional_string(v, "serverIPAddress", p)?
        .map(|s| {
            s.parse().map_err(|_| {
                error(
                    &format!("{p}.serverIPAddress"),
                    "expected IPv4 or IPv6 address",
                )
            })
        })
        .transpose()?;
    let connection = optional_string(v, "connection", p)?;
    let mut warnings = Vec::new();
    let sum = [timings.blocked, timings.dns, timings.connect].iter().fold(
        timings.send + timings.wait + timings.receive,
        |sum, t| {
            sum + if let Timing::Milliseconds(n) = t {
                *n
            } else {
                0.0
            }
        },
    );
    if !sum.is_finite() || (sum - total_ms).abs() > 0.01 {
        warnings.push("Timing stages differ from reported total".into());
    }
    if let Timing::Milliseconds(ssl) = timings.ssl
        && !matches!(timings.connect,Timing::Milliseconds(connect) if ssl<=connect)
    {
        warnings.push("SSL timing is inconsistent with connect timing".into());
    }
    if let Body::Undecoded { reason, .. } = &body {
        warnings.push(format!("Body could not be decoded: {reason}"));
    }
    Ok(Request {
        started_at,
        instant,
        total_ms,
        method,
        url,
        host,
        request_version,
        headers,
        query,
        cookies: request_cookies,
        request_body,
        request_header_size,
        request_body_size,
        status: status as u16,
        status_text,
        response_version,
        response_headers,
        response_cookies,
        content_size,
        mime_type,
        body,
        response_header_size,
        response_body_size,
        timings,
        server_ip,
        connection,
        warnings,
    })
}
