//! Frozen native browser exports; no browser, clock, or network is needed here.
use netscope_core::{Body, ReplayDraft, RequestBody, RequestId};
use netscope_import::{Limits, import_bytes};
use serde_json::{Value, json};
const CHROME: &[u8] = include_bytes!("../../../tests/fixtures/har/browsers/chrome-154.har");
const FIREFOX: &[u8] = include_bytes!("../../../tests/fixtures/har/browsers/firefox-146.har");
const IN_PROGRESS: &[u8] =
    include_bytes!("../../../tests/fixtures/har/browsers/firefox-146-in-progress.har");

#[test]
fn complete_browser_exports_preserve_recorded_representations()
-> Result<(), Box<dyn std::error::Error>> {
    for bytes in [CHROME, FIREFOX] {
        let session = import_bytes(bytes, Limits::default())?;
        assert_eq!(session.requests().len(), 9);
        assert_eq!(
            session.requests(),
            import_bytes(bytes, Limits::default())?.requests()
        );
        assert!(session.requests().iter().all(|r| r.host == "127.0.0.1"));
        assert_eq!(
            session.chronological(),
            &(0..9).map(RequestId).collect::<Vec<_>>()
        );
        let form = &session.requests()[4];
        let RequestBody::TextAndParams {
            text,
            params,
            mime_type,
        } = &form.request_body
        else {
            panic!("both form representations must survive")
        };
        assert_eq!(mime_type, "application/x-www-form-urlencoded");
        assert_eq!(text, "tag=one&tag=two&empty=");
        assert_eq!(
            params
                .iter()
                .map(|p| (p.name.as_str(), p.value.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("tag", Some("one")),
                ("tag", Some("two")),
                ("empty", Some(""))
            ]
        );
        assert!(ReplayDraft::from_request(RequestId(4), form).is_err());
        let draft = ReplayDraft::from_request(RequestId(3), &session.requests()[3])?;
        assert_eq!(draft.body(), "{\"title\":\"café\"}");
        assert!(draft.review()?.headers.is_empty());
        assert_eq!(
            session.requests()[5].body,
            Body::Binary(vec![0, 255, 1, 254])
        );
        assert_eq!(session.requests()[6].status, 302);
        assert_eq!(session.requests()[8].status, 404);
        assert_eq!(session.requests()[0].response_cookies[0].expires, None);
    }
    let firefox = import_bytes(FIREFOX, Limits::default())?;
    assert!(
        matches!(&firefox.requests()[3].request_body, RequestBody::TextAndParams { params, .. } if params.is_empty())
    );
    Ok(())
}

#[test]
fn in_progress_native_export_does_not_invent_response_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(IN_PROGRESS, Limits::default())?;
    assert_eq!(session.requests().len(), 3);
    let request = &session.requests()[2];
    assert_eq!(request.status, 0);
    assert!(request.request_version.is_empty() && request.response_version.is_empty());
    assert!(request.mime_type.is_empty());
    assert!(
        request
            .warnings
            .iter()
            .any(|s| s == "Response MIME type not recorded")
    );
    assert_eq!(request.body, Body::Text("hello café".into()));
    Ok(())
}

#[test]
fn compatibility_tolerance_still_checks_types_and_every_representation()
-> Result<(), Box<dyn std::error::Error>> {
    let original: Value = serde_json::from_slice(CHROME)?;
    for (pointer, replacement, expected) in [
        (
            "/log/entries/0/response/cookies/0/expires",
            json!(4),
            "log.entries[0].response.cookies[0].expires",
        ),
        (
            "/log/entries/0/request/httpVersion",
            Value::Null,
            "log.entries[0].request.httpVersion",
        ),
        (
            "/log/entries/0/response/content/mimeType",
            Value::Null,
            "log.entries[0].response.content.mimeType",
        ),
        (
            "/log/entries/4/request/postData/params",
            Value::Null,
            "log.entries[4].request.postData.params",
        ),
        (
            "/log/entries/4/request/postData/params/0/value",
            json!(1),
            "log.entries[4].request.postData.params[0].value",
        ),
        (
            "/log/entries/4/request/postData/text",
            Value::Null,
            "log.entries[4].request.postData.text",
        ),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).ok_or("fixture field absent")? = replacement;
        let error = import_bytes(&serde_json::to_vec(&changed)?, Limits::default())
            .err()
            .ok_or("invalid field accepted")?;
        assert_eq!(error.path, expected);
    }
    // Disagreement is retained, not silently reconciled by decoding either form.
    let mut changed = original;
    changed["log"]["entries"][4]["request"]["postData"]["text"] = json!("different=representation");
    let session = import_bytes(&serde_json::to_vec(&changed)?, Limits::default())?;
    assert!(
        matches!(&session.requests()[4].request_body, RequestBody::TextAndParams { text, params, .. } if text == "different=representation" && params[0].name == "tag")
    );
    Ok(())
}

const CHROME_H2: &[u8] = include_bytes!("../../../tests/fixtures/har/browsers/chrome-154-h2.har");
const FIREFOX_H2: &[u8] = include_bytes!("../../../tests/fixtures/har/browsers/firefox-146-h2.har");

#[test]
fn https_exports_keep_decoded_content_and_reported_sizes_separate()
-> Result<(), Box<dyn std::error::Error>> {
    use netscope_core::{RecordedProtocol, ReportedSize};
    let text = "A book about café.\n".repeat(64);
    assert_eq!(text.len(), 1280);
    for (bytes, label, gzip_size, br_size) in [
        (
            CHROME_H2,
            "http/2.0",
            ReportedSize::Unavailable,
            ReportedSize::Unavailable,
        ),
        (
            FIREFOX_H2,
            "HTTP/2",
            ReportedSize::Bytes(234),
            ReportedSize::Bytes(220),
        ),
    ] {
        let session = import_bytes(bytes, Limits::default())?;
        assert_eq!(session.requests().len(), 6);
        for request in session.requests() {
            assert!(request.url.starts_with("https://127.0.0.1:"));
            assert_eq!(request.request_version, label);
            assert_eq!(request.response_version, label);
            assert_eq!(
                RecordedProtocol::from_label(&request.response_version),
                RecordedProtocol::Http2
            );
            assert_eq!(request.status, 200);
        }
        for (index, encoding, length, reported) in
            [(2, "gzip", "52", gzip_size), (3, "br", "40", br_size)]
        {
            let request = &session.requests()[index];
            assert_eq!(request.body, Body::Text(text.clone()));
            assert_eq!(request.content_size, 1280);
            assert_eq!(request.response_body_size, reported);
            for (name, value) in [("content-encoding", encoding), ("content-length", length)] {
                assert!(
                    request
                        .response_headers
                        .iter()
                        .any(|h| h.name.eq_ignore_ascii_case(name) && h.value == value)
                );
            }
        }
        assert_eq!(session.requests()[1].body, session.requests()[5].body);
        assert_eq!(
            session.requests()[4].body,
            Body::Binary(vec![0, 255, 1, 254])
        );
        assert_eq!(session.requests()[4].content_size, 4);
    }
    Ok(())
}

#[test]
fn native_connection_timings_keep_unavailable_and_zero_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    use netscope_core::Timing;
    let chrome = import_bytes(CHROME_H2, Limits::default())?;
    let firefox = import_bytes(FIREFOX_H2, Limits::default())?;
    assert_eq!(chrome.requests()[0].timings.dns, Timing::Milliseconds(0.0));
    for index in 1..6 {
        let c = &chrome.requests()[index].timings;
        assert_eq!(c.dns, Timing::Unavailable);
        assert_eq!(c.connect, Timing::Unavailable);
        assert_eq!(c.ssl, Timing::Unavailable);
        let f = &firefox.requests()[index].timings;
        assert_eq!(f.dns, Timing::Milliseconds(0.0));
        assert_eq!(f.connect, Timing::Milliseconds(0.0));
        assert_eq!(f.ssl, Timing::Milliseconds(0.0));
    }
    // Omission is a derived case, not a claim about either native export.
    let mut changed: Value = serde_json::from_slice(CHROME_H2)?;
    changed["log"]["entries"][1]["timings"]
        .as_object_mut()
        .ok_or("timings")?
        .remove("connect");
    let omitted = import_bytes(&serde_json::to_vec(&changed)?, Limits::default())?;
    assert_eq!(omitted.requests()[1].timings.connect, Timing::Missing);
    Ok(())
}
