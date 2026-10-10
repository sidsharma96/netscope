use crate::{HostSummary, Timing};
use std::{collections::BTreeMap, net::IpAddr};

/// A request position in the original capture, independent of display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestId(pub usize);
/// A host position in first-occurrence order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HostId(pub usize);
/// A size reported by the exporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportedSize {
    /// No field was recorded.
    Missing,
    /// The exporter reported -1.
    Unavailable,
    /// A recorded byte count.
    Bytes(u64),
}
/// Captured response bytes, with representation failures kept visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// No body text was retained.
    NotRecorded,
    /// Text exactly as exported.
    Text(String),
    /// UTF-8 text decoded from base64.
    DecodedText(String),
    /// Non-UTF-8 bytes decoded from base64.
    Binary(Vec<u8>),
    /// Encoded text retained after a recoverable decoding failure.
    Undecoded {
        /// The original encoded representation.
        encoded: String,
        /// A fixed reason which contains no captured values.
        reason: String,
    },
}
/// A captured request body; parameters do not imply reconstructable wire bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestBody {
    /// No body representation was retained.
    NotRecorded,
    /// Captured text and its reported MIME type.
    Text {
        /// Exporter MIME type.
        mime_type: String,
        /// Captured text, including an empty string.
        text: String,
    },
    /// Ordered captured form fields.
    Params {
        /// Exporter MIME type.
        mime_type: String,
        /// Captured fields.
        params: Vec<Parameter>,
    },
}

/// An ordered captured name and value.
#[derive(Debug, Clone, PartialEq)]
pub struct NameValue {
    /// Original name.
    pub name: String,
    /// Original value.
    pub value: String,
}

/// A captured form field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// Field name.
    pub name: String,
    /// Retained value, when present.
    pub value: Option<String>,
    /// Reported filename.
    pub file_name: Option<String>,
    /// Reported content type.
    pub content_type: Option<String>,
}

/// A captured cookie without jar behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct Cookie {
    /// Original name and value.
    pub pair: NameValue,
    /// Reported path.
    pub path: Option<String>,
    /// Reported domain.
    pub domain: Option<String>,
    /// Validated timestamp as exported.
    pub expires: Option<String>,
    /// Reported HTTP-only attribute.
    pub http_only: Option<bool>,
    /// Reported secure attribute.
    pub secure: Option<bool>,
}

/// Reported stages; SSL is part of connect.
#[derive(Debug, Clone, PartialEq)]
pub struct Timings {
    /// Reported blocked duration.
    pub blocked: Timing,
    /// Reported dns duration.
    pub dns: Timing,
    /// Reported connect duration.
    pub connect: Timing,
    /// Reported ssl duration.
    pub ssl: Timing,
    /// Required send duration in milliseconds.
    pub send: f64,
    /// Required wait duration in milliseconds.
    pub wait: f64,
    /// Required receive duration in milliseconds.
    pub receive: f64,
}

/// A normalised URL host.
#[derive(Debug, Clone, PartialEq)]
pub struct Host {
    /// First-occurrence identity.
    pub id: HostId,
    /// Normalised host name.
    pub name: String,
}

/// One validated HAR entry; source position is assigned by Session.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Original timestamp.
    pub started_at: String,
    /// Comparable UTC seconds and nanoseconds.
    pub instant: (i64, u32),
    /// Exporter-reported total duration.
    pub total_ms: f64,
    /// Original HTTP token.
    pub method: String,
    /// Original absolute URL.
    pub url: String,
    /// Normalised URL host.
    pub host: String,
    /// Reported request protocol.
    pub request_version: String,
    /// Request headers in source order.
    pub headers: Vec<NameValue>,
    /// Query fields in source order.
    pub query: Vec<NameValue>,
    /// Request cookies in source order.
    pub cookies: Vec<Cookie>,
    /// Captured request representation.
    pub request_body: RequestBody,
    /// Reported request header size.
    pub request_header_size: ReportedSize,
    /// Reported request body size.
    pub request_body_size: ReportedSize,
    /// Recorded status; zero means no status recorded.
    pub status: u16,
    /// Exporter status text.
    pub status_text: String,
    /// Reported response protocol.
    pub response_version: String,
    /// Response headers in source order.
    pub response_headers: Vec<NameValue>,
    /// Response cookies in source order.
    pub response_cookies: Vec<Cookie>,
    /// Exporter-reported content size.
    pub content_size: u64,
    /// Exporter-reported MIME type.
    pub mime_type: String,
    /// Captured response representation.
    pub body: Body,
    /// Reported response header size.
    pub response_header_size: ReportedSize,
    /// Reported response body size.
    pub response_body_size: ReportedSize,
    /// Retained timing stages.
    pub timings: Timings,
    /// Exporter-reported address, without inferred connections.
    pub server_ip: Option<IpAddr>,
    /// Exporter connection metadata, including empty strings.
    pub connection: Option<String>,
    /// Fixed consistency and representation warnings.
    pub warnings: Vec<String>,
}

/// Owned capture data with separate chronological and host display orders.
#[derive(Debug)]
pub struct Session {
    requests: Vec<Request>,
    chronological: Vec<RequestId>,
    hosts: Vec<Host>,
    host_order: Vec<HostId>,
    host_summaries: Vec<HostSummary>,
    redirects: Vec<crate::RedirectEvidence>,
    retries: Vec<crate::RetryEvidence>,
}
impl Session {
    /// Build view orders without moving requests from their source positions.
    /// Inputs should be validated by an importer before construction.
    pub fn new(requests: Vec<Request>) -> Self {
        let mut chronological: Vec<_> = (0..requests.len()).map(RequestId).collect();
        chronological.sort_by_key(|id| (requests[id.0].instant, id.0));
        let mut hosts: Vec<Host> = Vec::new();
        let mut host_summaries: Vec<HostSummary> = Vec::new();
        let mut host_ids = BTreeMap::new();
        for request in &requests {
            let id = *host_ids.entry(request.host.as_str()).or_insert_with(|| {
                let id = HostId(hosts.len());
                hosts.push(Host {
                    id,
                    name: request.host.clone(),
                });
                host_summaries.push(HostSummary::new(id));
                id
            });
            // Count each source entry once, before any display order is applied.
            host_summaries[id.0].record(request);
        }
        for id in &chronological {
            if let Some(host) = host_ids.get(requests[id.0].host.as_str()) {
                host_summaries[host.0].request_ids.push(*id);
            }
        }
        let mut host_order: Vec<_> = hosts.iter().map(|host| host.id).collect();
        host_order.sort_by(|left, right| hosts[left.0].name.cmp(&hosts[right.0].name));
        let retries = crate::retries::analyze(&requests, &chronological);
        let redirects = crate::redirects::analyze(&requests, &chronological);
        Self {
            redirects,
            retries,
            requests,
            chronological,
            hosts,
            host_order,
            host_summaries,
        }
    }
    /// Borrow possible retry evidence without asserting client intent.
    pub fn retry(&self, id: RequestId) -> Option<&crate::RetryEvidence> {
        self.retries.get(id.0)
    }
    /// Borrow redirect evidence derived from this complete capture.
    pub fn redirect(&self, id: RequestId) -> Option<&crate::RedirectEvidence> {
        self.redirects.get(id.0)
    }
    /// Source-order requests, borrowed rather than copied.
    pub fn requests(&self) -> &[Request] {
        &self.requests
    }
    /// Chronological IDs with stable source-order ties.
    pub fn chronological(&self) -> &[RequestId] {
        &self.chronological
    }
    /// Look up a source ID; foreign or out-of-range IDs may not resolve.
    pub fn request(&self, id: RequestId) -> Option<&Request> {
        self.requests.get(id.0)
    }
    /// Hosts in first-occurrence order.
    pub fn hosts(&self) -> &[Host] {
        &self.hosts
    }
    /// Look up a host by its source-assigned identity.
    pub fn host(&self, id: HostId) -> Option<&Host> {
        self.hosts.get(id.0)
    }
    /// Borrow the derived summary for a host, if that ID exists.
    pub fn host_summary(&self, id: HostId) -> Option<&HostSummary> {
        self.host_summaries.get(id.0)
    }
    /// Host IDs in lexicographic display order.
    pub fn host_order(&self) -> &[HostId] {
        &self.host_order
    }
}
/// Escape terminal controls before displaying captured strings or filenames.
pub fn terminal_text(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
/// A preview retaining whether the byte limit shortened the representation.
#[derive(Debug, PartialEq, Eq)]
pub struct Preview {
    /// Safe visible text.
    pub text: String,
    /// Whether bytes were omitted.
    pub truncated: bool,
}
impl Body {
    /// Preview at most 16 KiB of text or 256 binary bytes, on UTF-8 boundaries.
    pub fn preview(&self) -> Preview {
        match self {
            Self::NotRecorded => Preview {
                text: "Response body not recorded".into(),
                truncated: false,
            },
            Self::Text(s) | Self::DecodedText(s) | Self::Undecoded { encoded: s, .. } => {
                let mut end = s.len().min(16 * 1024);
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                Preview {
                    text: terminal_text(&s[..end]),
                    truncated: end < s.len(),
                }
            }
            Self::Binary(bytes) => Preview {
                text: bytes
                    .iter()
                    .take(256)
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                truncated: bytes.len() > 256,
            },
        }
    }
}
