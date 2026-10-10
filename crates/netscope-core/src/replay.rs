//! Editable request data and validation, without any network operations.
use crate::{NameValue, ReportedSize, Request, RequestBody, RequestId};
use url::Url;

/// A separately owned draft; mutation never changes an imported request.
#[derive(Debug)]
pub struct ReplayDraft {
    source: RequestId,
    url: String,
    captured: Vec<NameValue>,
    selected: Vec<bool>,
    added: Vec<NameValue>,
    connection_fields: Vec<String>,
}
/// A validated semantic preview. This value cannot send network traffic.
#[derive(Debug, PartialEq)]
pub struct ReplayReview {
    /// Capture identity, not a new captured request.
    pub source: RequestId,
    /// Serialized HTTP(S) destination without a fragment.
    pub destination: String,
    /// Destination authority used by the future client-managed Host field.
    pub authority: String,
    /// A fragment retained for disclosure, never sent.
    pub fragment: Option<String>,
    /// Explicitly chosen fields, preserving duplicate entries.
    pub headers: Vec<NameValue>,
}
impl ReplayDraft {
    /// Copy eligible request data into an independent draft.
    /// # Errors
    /// Rejects non-GET requests and uncertain or captured request bodies.
    pub fn from_request(source: RequestId, request: &Request) -> Result<Self, &'static str> {
        if request.method != "GET" {
            return Err("Replay drafts require a GET request");
        }
        if request.request_body_size != ReportedSize::Bytes(0)
            || !matches!(request.request_body, RequestBody::NotRecorded)
        {
            return Err("Replay requires reported body size 0 and no captured request body");
        }
        let connection_fields = request
            .headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case("connection"))
            .flat_map(|h| h.value.split(',').map(|s| s.trim().to_ascii_lowercase()))
            .collect();
        Ok(Self {
            source,
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
    /// Replace the URL; any text change clears all outgoing header choices.
    pub fn set_url(&mut self, url: String) -> bool {
        if self.url == url {
            return false;
        }
        self.url = url;
        self.selected.fill(false);
        self.added.clear();
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
