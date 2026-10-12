//! A versioned request file contains values, never live connections or results.
use crate::{BodyFormat, NameValue, ReplayDraft, ReplayMethod, RequestId};
use serde::{Deserialize, Serialize};

/// Maximum encoded file size, including JSON escaping.
pub const SAVED_FILE_LIMIT: usize = 524_288;
/// A portable request without capture identity or response history.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDraft {
    version: u32,
    /// User-chosen label, independent of the file name.
    pub name: String,
    url: String,
    method: String,
    format: String,
    body: String,
    headers: Vec<SavedHeader>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedHeader {
    name: String,
    value: String,
}
/// Fields omitted from saved files even when explicitly selected for sending.
pub fn sensitive_saved_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "cookie2"
            | "set-cookie"
            | "x-api-key"
            | "api-key"
            | "x-auth-token"
            | "x-access-token"
    )
}
impl SavedDraft {
    /// Prepare a validated snapshot, excluding known credential headers.
    /// # Errors
    /// Rejects invalid names or requests that cannot pass review.
    pub fn from_draft(name: String, draft: &ReplayDraft) -> Result<Self, &'static str> {
        let review = draft.review()?;
        let saved = Self {
            version: 1,
            name,
            url: draft.url().into(),
            method: review.method.as_str().into(),
            format: match review.format {
                BodyFormat::Plain => "plain",
                BodyFormat::Json => "json",
            }
            .into(),
            body: review.body,
            headers: review
                .headers
                .into_iter()
                .filter(|h| !sensitive_saved_header(&h.name))
                .map(|h| SavedHeader {
                    name: h.name,
                    value: h.value,
                })
                .collect(),
        };
        saved.to_draft()?;
        Ok(saved)
    }
    /// Reconstruct independent editable values with no captured response.
    /// # Errors
    /// Rejects unsupported versions, invalid fields, or credential headers.
    pub fn to_draft(&self) -> Result<ReplayDraft, &'static str> {
        if self.version != 1 {
            return Err("Unsupported saved-draft version");
        }
        if self.name.trim().is_empty()
            || self.name.len() > 128
            || self.name.chars().any(char::is_control)
        {
            return Err("Name must contain 1–128 bytes without control characters");
        }
        let method = match self.method.as_str() {
            "GET" => ReplayMethod::Get,
            "POST" => ReplayMethod::Post,
            _ => return Err("Unsupported saved method"),
        };
        let format = match self.format.as_str() {
            "plain" => BodyFormat::Plain,
            "json" => BodyFormat::Json,
            _ => return Err("Unsupported saved body format"),
        };
        if method == ReplayMethod::Get && format != BodyFormat::Plain {
            return Err("GET must use the plain, empty body representation");
        }
        if self.headers.len() > 100
            || self
                .headers
                .iter()
                .map(|h| h.name.len() + h.value.len() + 4)
                .sum::<usize>()
                > 32_768
        {
            return Err("Saved headers exceed the request limit");
        }
        let mut headers = Vec::new();
        for h in &self.headers {
            if sensitive_saved_header(&h.name) {
                return Err(
                    "Remove credential headers from the saved file; add them after opening",
                );
            }
            let header = NameValue {
                name: h.name.clone(),
                value: h.value.clone(),
            };
            crate::validate_replay_header(&header)?;
            headers.push(header);
        }
        ReplayDraft::from_saved_parts(self.url.clone(), method, format, self.body.clone(), headers)
    }
    /// Encode exactly the values shown in the save preview.
    /// # Errors
    /// Rejects invalid data or an excessive encoded size.
    pub fn encode(&self) -> Result<String, &'static str> {
        self.to_draft()?;
        let mut text =
            serde_json::to_string_pretty(self).map_err(|_| "Could not encode saved draft")?;
        text.push('\n');
        if text.len() > SAVED_FILE_LIMIT {
            return Err("Saved file exceeds 512 KiB");
        }
        Ok(text)
    }
    /// Decode a bounded file and validate it before exposing a draft.
    /// # Errors
    /// Rejects malformed JSON, unknown fields, versions, or invalid request values.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > SAVED_FILE_LIMIT {
            return Err("Saved file exceeds 512 KiB");
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| "Invalid saved-draft JSON or fields")?;
        value.to_draft()?;
        Ok(value)
    }
}

// No real capture uses this ID; opening a saved file must not invent an observation.
pub(crate) const SAVED_SOURCE: RequestId = RequestId(usize::MAX);

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Result<ReplayDraft, &'static str> {
        ReplayDraft::from_saved_parts(
            "http://127.0.0.1:8080/books#note".into(),
            ReplayMethod::Post,
            BodyFormat::Json,
            " {\"title\":\"café\",\"price\":1.00}\n".into(),
            vec![
                NameValue {
                    name: "X-Example".into(),
                    value: "one".into(),
                },
                NameValue {
                    name: "x-example".into(),
                    value: "two".into(),
                },
                NameValue {
                    name: "aUtHoRiZaTiOn".into(),
                    value: "secret-token".into(),
                },
                NameValue {
                    name: "Cookie".into(),
                    value: "secret-cookie".into(),
                },
                NameValue {
                    name: "X-Api-Key".into(),
                    value: "secret-key".into(),
                },
            ],
        )
    }
    #[test]
    fn round_trip_preserves_text_and_duplicates_without_credentials_or_capture()
    -> Result<(), &'static str> {
        let original = draft()?;
        let file = SavedDraft::from_draft("Book café".into(), &original)?.encode()?;
        assert!(!file.contains("secret-"));
        let saved = SavedDraft::decode(file.as_bytes())?;
        let reopened = saved.to_draft()?;
        assert!(!reopened.has_capture());
        assert_eq!(reopened.url(), original.url());
        assert_eq!(reopened.body(), original.body());
        assert_eq!(reopened.review()?.headers.len(), 2);
        assert_eq!(reopened.review()?.headers[1].value, "two");
        assert_eq!(original.review()?.headers.len(), 5);
        assert_eq!(saved.encode()?, file);
        Ok(())
    }
    #[test]
    fn malformed_versions_unknown_fields_and_invalid_requests_are_rejected()
    -> Result<(), &'static str> {
        let encoded = SavedDraft::from_draft("Book".into(), &draft()?)?.encode()?;
        let base: serde_json::Value = serde_json::from_str(&encoded).map_err(|_| "fixture")?;
        for (key, value) in [
            ("version", serde_json::json!(2)),
            ("method", serde_json::json!("DELETE")),
            ("format", serde_json::json!("binary")),
            ("body", serde_json::json!("broken json")),
            ("name", serde_json::json!("\u{1b}secret")),
            ("url", serde_json::json!("file:///tmp/a")),
            ("unknown", serde_json::json!(true)),
            (
                "headers",
                serde_json::json!([{"name":"Authorization","value":"secret"}]),
            ),
            (
                "headers",
                serde_json::json!([{"name":"Content-Length","value":"3"}]),
            ),
        ] {
            let mut document = base.clone();
            document[key] = value;
            assert!(
                SavedDraft::decode(document.to_string().as_bytes()).is_err(),
                "accepted {key}"
            );
        }
        assert!(SavedDraft::decode(b"{").is_err());
        assert!(SavedDraft::decode(format!("{encoded} null").as_bytes()).is_err());
        assert!(
            SavedDraft::decode(encoded.replacen("{", "{\"version\":1,", 1).as_bytes()).is_err()
        );
        assert!(SavedDraft::decode(&vec![b' '; SAVED_FILE_LIMIT + 1]).is_err());
        Ok(())
    }
    #[test]
    fn names_and_body_limits_apply_before_saving() -> Result<(), &'static str> {
        let draft = draft()?;
        for name in [
            String::new(),
            " ".into(),
            "a".repeat(129),
            "line\nname".into(),
        ] {
            assert!(SavedDraft::from_draft(name, &draft).is_err());
        }
        let mut plain = ReplayDraft::from_saved_parts(
            "http://localhost/".into(),
            ReplayMethod::Post,
            BodyFormat::Plain,
            "é".repeat(8192),
            vec![],
        )?;
        assert!(SavedDraft::from_draft("Boundary".into(), &plain).is_ok());
        plain.toggle_format();
        assert!(SavedDraft::from_draft("Invalid JSON".into(), &plain).is_err());
        Ok(())
    }
}
