//! Editable request data and validation, without any network operations.
use crate::{NameValue, ReportedSize, Request, RequestBody, RequestId};
use url::Url;

/// Maximum UTF-8 request-body size for the editor and transport.
pub const REQUEST_BODY_LIMIT: usize = 16_384;
/// Methods supported by reviewed replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMethod {
    /// Fetch without a request body in this profile.
    Get,
    /// Submit an explicitly reviewed body.
    Post,
}
impl ReplayMethod {
    /// HTTP method spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}
/// Interpretation selected for POST text, without changing its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFormat {
    /// UTF-8 text without a structured syntax requirement.
    Plain,
    /// One JSON value, validated without reformatting.
    Json,
}
impl BodyFormat {
    /// Client-managed media type for the selected representation.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Plain => "text/plain; charset=utf-8",
            Self::Json => "application/json",
        }
    }
}
/// Validate the outgoing representation without sending or reformatting it.
/// # Errors
/// Rejects GET bodies, excessive byte lengths, and malformed JSON.
pub fn validate_replay_body(
    method: ReplayMethod,
    format: BodyFormat,
    body: &str,
) -> Result<(), &'static str> {
    if body.len() > REQUEST_BODY_LIMIT {
        return Err("Request body exceeds 16 KiB");
    }
    if method == ReplayMethod::Get {
        if !body.is_empty() {
            return Err("GET bodies are unsupported; choose POST");
        }
    } else if format == BodyFormat::Json {
        // Parsing checks syntax; the original text remains the outgoing body.
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|_| "Body must be one valid JSON value within numeric/nesting limits")?;
    }
    Ok(())
}
/// A separately owned draft; mutation never changes an imported request.
#[derive(Debug)]
pub struct ReplayDraft {
    source: RequestId,
    url: String,
    method: ReplayMethod,
    format: BodyFormat,
    body: String,
    captured: Vec<NameValue>,
    selected: Vec<bool>,
    added: Vec<NameValue>,
    connection_fields: Vec<String>,
}
/// A validated semantic preview. This value cannot send network traffic.
#[derive(Debug, PartialEq)]
pub struct ReplayReview {
    /// Reviewed method.
    pub method: ReplayMethod,
    /// Selected POST representation.
    pub format: BodyFormat,
    /// Exact UTF-8 body; empty for GET.
    pub body: String,
    /// Capture identity, not a new captured request.
    pub source: RequestId,
    /// Serialized HTTP(S) destination without a fragment.
    pub destination: String,
    /// Destination authority used by the client-managed Host field.
    pub authority: String,
    /// A fragment retained for disclosure, never sent.
    pub fragment: Option<String>,
    /// Explicitly chosen fields, preserving duplicate entries.
    pub headers: Vec<NameValue>,
}
impl ReplayDraft {
    /// Copy eligible request data into an independent draft.
    /// # Errors
    /// Rejects unsupported methods and missing or unsupported body representations.
    pub fn from_request(source: RequestId, request: &Request) -> Result<Self, &'static str> {
        let (method, format, body) = match request.method.as_str() {
            "GET"
                if request.request_body_size == ReportedSize::Bytes(0)
                    && matches!(request.request_body, RequestBody::NotRecorded) =>
            {
                (ReplayMethod::Get, BodyFormat::Plain, String::new())
            }
            "GET" => {
                return Err(
                    "Replay requires reported body size 0 and no captured request body for GET",
                );
            }
            "POST" => {
                if request.headers.iter().any(|h| {
                    h.name.eq_ignore_ascii_case("content-encoding")
                        && !h.value.trim().eq_ignore_ascii_case("identity")
                }) {
                    return Err("Encoded POST bodies are unsupported");
                }
                match &request.request_body {
                    RequestBody::NotRecorded
                        if request.request_body_size == ReportedSize::Bytes(0) =>
                    {
                        (ReplayMethod::Post, BodyFormat::Plain, String::new())
                    }
                    RequestBody::Text { mime_type, text } => {
                        if text.len() > REQUEST_BODY_LIMIT {
                            return Err("Request body exceeds 16 KiB");
                        }
                        let mut parts = mime_type.split(';');
                        let format = match parts
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_ascii_lowercase()
                            .as_str()
                        {
                            "text/plain" => BodyFormat::Plain,
                            "application/json" => BodyFormat::Json,
                            _ => return Err("Only plain text and JSON POST bodies are supported"),
                        };
                        for parameter in parts {
                            if !parameter.trim().eq_ignore_ascii_case("charset=utf-8") {
                                return Err("Only UTF-8 POST text is supported");
                            }
                        }
                        (ReplayMethod::Post, format, text.clone())
                    }
                    _ => {
                        return Err(
                            "POST requires captured plain text/JSON or an explicitly empty body",
                        );
                    }
                }
            }
            _ => return Err("Replay drafts require GET or POST"),
        };
        let connection_fields = request
            .headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case("connection"))
            .flat_map(|h| h.value.split(',').map(|s| s.trim().to_ascii_lowercase()))
            .collect();
        Ok(Self {
            source,
            method,
            format,
            body,
            url: request.url.clone(),
            captured: request.headers.clone(),
            selected: vec![false; request.headers.len()],
            added: vec![],
            connection_fields,
        })
    }
    /// Source request identity.
    pub fn source(&self) -> RequestId {
        self.source
    }
    /// Current editable URL text, including invalid drafts.
    pub fn url(&self) -> &str {
        &self.url
    }
    /// Current method.
    pub fn method(&self) -> ReplayMethod {
        self.method
    }
    /// Selected body interpretation.
    pub fn format(&self) -> BodyFormat {
        self.format
    }
    /// Exact editable body text.
    pub fn body(&self) -> &str {
        &self.body
    }
    /// Switch method and clear the previous body and format.
    pub fn toggle_method(&mut self) {
        self.method = if self.method == ReplayMethod::Get {
            ReplayMethod::Post
        } else {
            ReplayMethod::Get
        };
        self.body.clear();
        self.format = BodyFormat::Plain;
    }
    /// Switch the POST interpretation without reformatting its text.
    pub fn toggle_format(&mut self) {
        if self.method == ReplayMethod::Post {
            self.format = if self.format == BodyFormat::Plain {
                BodyFormat::Json
            } else {
                BodyFormat::Plain
            };
        }
    }
    /// Save editable POST text; malformed JSON remains available for repair.
    /// # Errors
    /// Rejects a GET body or text exceeding the editor's byte limit.
    pub fn set_body(&mut self, body: String) -> Result<(), &'static str> {
        if self.method != ReplayMethod::Post {
            return Err("Choose POST before editing a body");
        }
        if body.len() > REQUEST_BODY_LIMIT {
            return Err("Request body exceeds 16 KiB");
        }
        self.body = body;
        Ok(())
    }
    /// Captured header choices in their original order.
    pub fn captured(&self) -> &[NameValue] {
        &self.captured
    }
    /// Manually added fields in insertion order.
    pub fn added(&self) -> &[NameValue] {
        &self.added
    }
    /// Whether a captured entry is selected.
    pub fn selected(&self, index: usize) -> bool {
        self.selected.get(index).copied().unwrap_or(false)
    }
    /// Replace the URL; any text change clears the body and outgoing header choices.
    pub fn set_url(&mut self, url: String) -> bool {
        if self.url == url {
            return false;
        }
        self.url = url;
        self.selected.fill(false);
        self.added.clear();
        self.body.clear();
        true
    }
    /// Explain why a captured field cannot be selected.
    pub fn header_error(&self, index: usize) -> Option<&'static str> {
        let header = match self.captured.get(index) {
            Some(h) => h,
            None => return Some("Header no longer exists"),
        };
        if self
            .connection_fields
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&header.name))
        {
            return Some("Named by captured Connection; cannot copy");
        }
        validate_replay_header(header).err()
    }
    /// Toggle one captured header without merging duplicate names.
    /// # Errors
    /// Rejects absent, reserved, or invalid entries.
    pub fn toggle(&mut self, index: usize) -> Result<(), &'static str> {
        if let Some(reason) = self.header_error(index) {
            return Err(reason);
        }
        self.selected[index] = !self.selected[index];
        Ok(())
    }
    /// Append one explicitly entered header.
    /// # Errors
    /// Rejects invalid/reserved fields or more than 100 manual entries.
    pub fn add(&mut self, header: NameValue) -> Result<(), &'static str> {
        validate_replay_header(&header)?;
        if self.added.len() >= 100 {
            return Err("Limit: 100 manually added headers");
        }
        self.added.push(header);
        Ok(())
    }
    /// Remove an added entry if present.
    pub fn remove(&mut self, index: usize) {
        if index < self.added.len() {
            self.added.remove(index);
        }
    }
    /// Validate a fresh preview; no previous preview is cached in the draft.
    /// # Errors
    /// Rejects invalid or unsupported destinations.
    pub fn review(&self) -> Result<ReplayReview, &'static str> {
        validate_replay_body(self.method, self.format, &self.body)?;
        if self.url.len() > 16_384 {
            return Err("URL exceeds 16 KiB");
        }
        if self
            .url
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
            || self.url.contains('\\')
            || !self.url.contains("://")
        {
            return Err("Use an absolute HTTP(S) URL without whitespace, controls, or backslashes");
        }
        let mut parsed = Url::parse(&self.url).map_err(|_| "Invalid URL")?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("Only HTTP(S) URLs with a host are supported");
        }
        let authority = self
            .url
            .split_once("://")
            .map(|(_, s)| s.split(['/', '?', '#']).next().unwrap_or(""))
            .unwrap_or("");
        if authority.is_empty() {
            return Err("URL must contain an explicit host after ://");
        }
        if authority.contains('@') || !parsed.username().is_empty() || parsed.password().is_some() {
            return Err("Remove embedded URL credentials; choose headers explicitly");
        }
        let host = parsed.host_str().ok_or("URL needs a host")?;
        let authority = match parsed.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        };
        let fragment = parsed.fragment().map(str::to_owned);
        parsed.set_fragment(None);
        let mut headers: Vec<_> = self
            .captured
            .iter()
            .zip(&self.selected)
            .filter(|(_, chosen)| **chosen)
            .map(|(h, _)| h.clone())
            .collect();
        headers.extend(self.added.iter().cloned());
        Ok(ReplayReview {
            method: self.method,
            format: self.format,
            body: self.body.clone(),
            source: self.source,
            destination: parsed.into(),
            authority,
            fragment,
            headers,
        })
    }
}
/// Validate one explicitly chosen field for the initial replay profile.
/// # Errors
/// Rejects reserved names, invalid syntax, unsupported bytes, and oversized fields.
pub fn validate_replay_header(header: &NameValue) -> Result<(), &'static str> {
    let token = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b);
    if header.name.is_empty() || !header.name.bytes().all(token) {
        return Err("Header name must be an HTTP token");
    }
    if [
        "host",
        "content-length",
        "content-type",
        "content-encoding",
        "transfer-encoding",
        "connection",
        "proxy-connection",
        "keep-alive",
        "te",
        "trailer",
        "upgrade",
        "proxy-authorization",
        "accept-encoding",
    ]
    .iter()
    .any(|n| header.name.eq_ignore_ascii_case(n))
    {
        return Err("Client-managed or unsupported header");
    }
    if header.name.len() > 8192 || header.value.len() > 8192 {
        return Err("Header field exceeds 8 KiB");
    }
    if !header
        .value
        .bytes()
        .all(|b| b == b'\t' || (32..=126).contains(&b))
    {
        return Err("Header value must use printable ASCII or tabs; no line breaks");
    }
    Ok(())
}
