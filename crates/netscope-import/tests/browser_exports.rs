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
