//! Replay drafts own copies and never execute network work.
use netscope_core::{NameValue, ReplayDraft, ReportedSize, Request, RequestBody, RequestId};
use netscope_import::{Limits, import_bytes};
fn request() -> Result<Request, Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/single-request.har"),
        Limits::default(),
    )?;
    Ok(session.requests()[0].clone())
}
#[test]
fn draft_eligibility_does_not_drop_a_body() -> Result<(), Box<dyn std::error::Error>> {
    let base = request()?;
    for size in [
        ReportedSize::Missing,
        ReportedSize::Unavailable,
        ReportedSize::Bytes(1),
    ] {
        let mut r = base.clone();
        r.request_body_size = size;
        assert!(ReplayDraft::from_request(RequestId(0), &r).is_err());
    }
    let mut r = base.clone();
    r.method = "POST".into();
    assert!(ReplayDraft::from_request(RequestId(0), &r).is_err());
    r = base.clone();
    r.request_body = RequestBody::Text {
        mime_type: "text/plain".into(),
        text: String::new(),
    };
    assert!(ReplayDraft::from_request(RequestId(0), &r).is_err());
    Ok(())
}
#[test]
fn duplicate_choices_and_url_reset_preserve_original_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let r = request()?;
    let original = r.clone();
    let mut draft = ReplayDraft::from_request(RequestId(3), &r)?;
    assert!(draft.review()?.headers.is_empty());
    draft.toggle(0)?;
    draft.toggle(1)?;
    draft.add(NameValue {
        name: "Authorization".into(),
        value: "synthetic".into(),
    })?;
    assert_eq!(draft.review()?.headers.len(), 3);
    let old = draft.url().to_owned();
    assert!(!draft.set_url(old));
    assert!(draft.selected(0));
    assert!(draft.set_url("http://localhost:8080/books?b=2&a=1#part".into()));
    let preview = draft.review()?;
    assert!(preview.headers.is_empty());
    assert_eq!(preview.source, RequestId(3));
    assert_eq!(preview.destination, "http://localhost:8080/books?b=2&a=1");
    assert_eq!(preview.fragment.as_deref(), Some("part"));
    assert_eq!(r, original);
    Ok(())
}
#[test]
fn urls_require_explicit_supported_destinations() -> Result<(), Box<dyn std::error::Error>> {
    let mut draft = ReplayDraft::from_request(RequestId(0), &request()?)?;
    for url in [
        "/relative",
        "http:example.test",
        "file:///tmp/a",
        "https://user:pass@example.test",
        "https://@example.test",
        "https://example.test/a b",
        "https://example.test/\n",
        "https://example.test\\a",
        "https://[",
    ] {
        draft.set_url(url.into());
        assert!(draft.review().is_err(), "{url:?}");
    }
    draft.set_url("https://EXAMPLE.test:443/a/../books".into());
    assert_eq!(draft.review()?.destination, "https://example.test/books");
    draft.set_url(format!("https://example.test/{}", "a".repeat(16_384)));
    assert!(draft.review().is_err());
    Ok(())
}
#[test]
fn headers_are_validated_without_leaking_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = request()?;
    r.headers.extend([
        NameValue {
            name: "Connection".into(),
            value: "X-Hop".into(),
        },
        NameValue {
            name: "x-hop".into(),
            value: "private".into(),
        },
        NameValue {
            name: "bad name".into(),
            value: "private".into(),
        },
    ]);
    let mut draft = ReplayDraft::from_request(RequestId(0), &r)?;
    for i in [2, 3, 4, 999] {
        assert!(draft.toggle(i).is_err());
    }
    for name in [
        "Host",
        "Content-Length",
        "TRANSFER-ENCODING",
        "Connection",
        "Proxy-Connection",
        "Keep-Alive",
        "TE",
        "Trailer",
        "Upgrade",
        "Proxy-Authorization",
        "Accept-Encoding",
        ":authority",
    ] {
        assert!(
            draft
                .add(NameValue {
                    name: name.into(),
                    value: "private".into()
                })
                .is_err()
        );
    }
    for value in ["private\r\nInjected: yes", "private\x1b", "privateé"] {
        let error = draft
            .add(NameValue {
                name: "X-Test".into(),
                value: value.into(),
            })
            .err()
            .ok_or("expected validation failure")?;
        assert!(!error.contains("private"));
    }
    draft.add(NameValue {
        name: "Cookie".into(),
        value: "synthetic=1".into(),
    })?;
    draft.add(NameValue {
        name: "X-Empty".into(),
        value: String::new(),
    })?;
    assert_eq!(draft.review()?.headers.len(), 2);
    draft.remove(0);
    assert_eq!(draft.review()?.headers[0].name, "X-Empty");
    Ok(())
}

#[test]
fn preview_authority_and_entry_limits_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let mut draft = ReplayDraft::from_request(RequestId(0), &request()?)?;
    for (url, authority) in [
        ("http://localhost:8080/", "localhost:8080"),
        ("https://example.test:443/", "example.test"),
        ("http://[::1]:8080/", "[::1]:8080"),
    ] {
        draft.set_url(url.into());
        assert_eq!(draft.review()?.authority, authority);
    }
    assert!(
        draft
            .add(NameValue {
                name: "X-Test".into(),
                value: "a".repeat(8193)
            })
            .is_err()
    );
    for _ in 0..100 {
        draft.add(NameValue {
            name: "X-Test".into(),
            value: String::new(),
        })?;
    }
    assert!(
        draft
            .add(NameValue {
                name: "X-Test".into(),
                value: String::new()
            })
            .is_err()
    );
    draft.set_url("http:///example.test".into());
    assert!(draft.review().is_err());
    Ok(())
}
