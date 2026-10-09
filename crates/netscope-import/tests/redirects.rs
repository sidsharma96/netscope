//! Fixed capture evidence; matching never sends a request or reads a clock.
use netscope_core::{NameValue, RedirectEvidence, Request, RequestId, Session};
use netscope_import::{Limits, import_bytes};

fn request() -> Result<Request, Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/single-request.har"),
        Limits::default(),
    )?;
    Ok(session.requests()[0].clone())
}
fn source() -> Result<Request, Box<dyn std::error::Error>> {
    let mut r = request()?;
    r.url = "https://example.test/old/page".into();
    r.instant = (100, 0);
    r.total_ms = 10.0;
    r.status = 302;
    r.method = "GET".into();
    r.response_headers = vec![NameValue {
        name: "lOcAtIoN".into(),
        value: "../new#heading".into(),
    }];
    Ok(r)
}
fn candidate(ms: u32, method: &str) -> Result<Request, Box<dyn std::error::Error>> {
    let mut r = request()?;
    r.url = "https://EXAMPLE.test:443/new#other".into();
    r.instant = (100 + i64::from(ms / 1000), (ms % 1000) * 1_000_000);
    r.method = method.into();
    r.status = 200;
    Ok(r)
}
fn ids(session: &Session, id: usize) -> Result<&[RequestId], Box<dyn std::error::Error>> {
    match session.redirect(RequestId(id)) {
        Some(RedirectEvidence::Candidates { ids, .. }) => Ok(ids),
        _ => Err("expected candidate analysis".into()),
    }
}
#[test]
fn relative_location_normalization_and_ambiguous_ties_keep_source_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new(vec![
        candidate(20, "GET")?,
        source()?,
        candidate(10, "GET")?,
        candidate(10, "GET")?,
    ]);
    assert_eq!(
        ids(&session, 1)?,
        [RequestId(2), RequestId(3), RequestId(0)]
    );
    match session.redirect(RequestId(1)) {
        Some(RedirectEvidence::Candidates { target, more, .. }) => {
            assert_eq!(target, "https://example.test/new");
            assert!(!more);
        }
        _ => return Err("missing evidence".into()),
    }
    assert!(session.redirect(RequestId(99)).is_none());
    Ok(())
}
#[test]
fn window_excludes_overlap_and_includes_both_end_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new(vec![
        source()?,
        candidate(0, "GET")?,
        candidate(9, "GET")?,
        candidate(10, "GET")?,
        candidate(30_010, "GET")?,
        candidate(30_011, "GET")?,
    ]);
    assert_eq!(ids(&session, 0)?, [RequestId(3), RequestId(4)]);
    let mut zero = source()?;
    zero.total_ms = 0.0;
    let session = Session::new(vec![zero, candidate(0, "GET")?, candidate(1, "GET")?]);
    assert_eq!(ids(&session, 0)?, [RequestId(2)]);
    Ok(())
}
#[test]
fn methods_follow_status_rules_without_case_folding() -> Result<(), Box<dyn std::error::Error>> {
    for status in [301, 302, 303, 307, 308] {
        let mut r = source()?;
        r.status = status;
        r.method = "POST".into();
        let session = Session::new(vec![
            r,
            candidate(10, "GET")?,
            candidate(11, "POST")?,
            candidate(12, "HEAD")?,
            candidate(13, "get")?,
        ]);
        let expected = match status {
            301 | 302 => vec![RequestId(1), RequestId(2)],
            303 => vec![RequestId(1), RequestId(3)],
            _ => vec![RequestId(2)],
        };
        assert_eq!(ids(&session, 0)?, expected);
    }
    Ok(())
}
#[test]
fn unsupported_status_and_missing_conflicting_or_unusable_location_are_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    for status in [0, 200, 201, 300, 304, 305, 306, 399] {
        let mut r = source()?;
        r.status = status;
        assert_eq!(
            Session::new(vec![r]).redirect(RequestId(0)),
            Some(&RedirectEvidence::NotApplicable)
        );
    }
    let mut missing = source()?;
    missing.response_headers.clear();
    let mut duplicate = source()?;
    duplicate
        .response_headers
        .push(duplicate.response_headers[0].clone());
    for r in [missing, duplicate] {
        assert!(matches!(
            Session::new(vec![r]).redirect(RequestId(0)),
            Some(RedirectEvidence::Unavailable(_))
        ));
    }
    for value in [
        "",
        "  ",
        "/new\n",
        "https://[",
        "file:///new",
        "https://example.test\\new",
    ] {
        let mut r = source()?;
        r.response_headers[0].value = value.into();
        assert!(
            matches!(
                Session::new(vec![r]).redirect(RequestId(0)),
                Some(RedirectEvidence::Unavailable(_))
            ),
            "{value:?}"
        );
    }
    Ok(())
}
#[test]
fn matching_preserves_query_order_scheme_and_nondefault_port()
-> Result<(), Box<dyn std::error::Error>> {
    for url in [
        "http://example.test/new",
        "https://example.test:444/new",
        "https://example.test/new?x=1",
        "https://example.test/NEW",
    ] {
        let mut next = candidate(10, "GET")?;
        next.url = url.into();
        assert!(ids(&Session::new(vec![source()?, next]), 0)?.is_empty());
    }
    let mut r = source()?;
    r.response_headers[0].value = "/new?a=1&b=2".into();
    let mut next = candidate(10, "GET")?;
    next.url = "https://example.test/new?b=2&a=1".into();
    assert!(ids(&Session::new(vec![r, next]), 0)?.is_empty());
    Ok(())
}
#[test]
fn candidate_storage_is_bounded_and_does_not_hide_ambiguity()
-> Result<(), Box<dyn std::error::Error>> {
    let mut requests = vec![source()?];
    for ms in 10..31 {
        requests.push(candidate(ms, "GET")?);
    }
    let session = Session::new(requests);
    assert_eq!(ids(&session, 0)?.len(), 20);
    assert!(matches!(
        session.redirect(RequestId(0)),
        Some(RedirectEvidence::Candidates { more: true, .. })
    ));
    Ok(())
}
#[test]
fn fixture_uses_headers_even_when_exporter_metadata_disagrees()
-> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/redirects.har"),
        Limits::default(),
    )?;
    assert_eq!(ids(&session, 0)?, [RequestId(1), RequestId(2)]);
    assert_eq!(ids(&session, 3)?, [RequestId(4)]);
    assert!(matches!(
        session.redirect(RequestId(5)),
        Some(RedirectEvidence::Unavailable(_))
    ));
    Ok(())
}
