/// A recognized exporter label, not a verified observation of network traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedProtocol {
    /// The exact HTTP/1.0 label.
    Http10,
    /// The exact HTTP/1.1 label.
    Http11,
    /// An explicitly supported HTTP/2 label.
    Http2,
    /// An explicitly supported HTTP/3 label.
    Http3,
    /// A label outside this tool's recognition policy.
    Unrecognized,
}
impl RecordedProtocol {
    /// Recognize only documented spellings while the request retains the raw text.
    pub fn from_label(label: &str) -> Self {
        match label {
            "HTTP/1.0" => Self::Http10,
            "HTTP/1.1" => Self::Http11,
            "HTTP/2" | "HTTP/2.0" | "http/2.0" | "h2" => Self::Http2,
            "HTTP/3" | "HTTP/3.0" | "h3" => Self::Http3,
            _ => Self::Unrecognized,
        }
    }
    /// A short classification label for display.
    pub fn name(self) -> &'static str {
        match self {
            Self::Http10 => "HTTP/1.0",
            Self::Http11 => "HTTP/1.1",
            Self::Http2 => "HTTP/2",
            Self::Http3 => "HTTP/3",
            Self::Unrecognized => "Unrecognized label",
        }
    }
    /// General protocol context, without asserting it was observed in the capture.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Http10 => {
                "HTTP/1.0 is an earlier version; this label does not identify a connection"
            }
            Self::Http11 => {
                "HTTP/1.1 uses textual start lines and headers; bodies can contain binary data"
            }
            Self::Http2 => {
                "HTTP/2 can interleave frames from different streams on one TCP connection"
            }
            Self::Http3 => {
                "HTTP/3 carries exchanges on QUIC streams; speed is not established here"
            }
            Self::Unrecognized => "No protocol explanation is inferred from this label",
        }
    }
}
