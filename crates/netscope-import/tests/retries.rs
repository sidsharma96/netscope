//! Fixed inputs distinguish possible repeats from known client intent.
use netscope_core::{
    Cookie, NameValue, ReportedSize, Request, RequestBody, RequestId, RetryEvidence, Session,
};
use netscope_import::{Limits, import_bytes};

fn source() -> Result<Request, Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/retries.har"),
        Limits::default(),
    )?;
    let mut r = session.requests()[0].clone();
    r.instant = (100, 0);
    Ok(r)
}
fn later(source: &Request, ms: u32) -> Request {
    let mut r = source.clone();
    r.instant = (100 + i64::from(ms / 1000), (ms % 1000) * 1_000_000);
    r.status = 200;
    r
}
fn ids(session: &Session) -> Result<&[RequestId], Box<dyn std::error::Error>> {
    match session.retry(RequestId(0)) {
        Some(RetryEvidence::Candidates { ids, .. }) => Ok(ids),
        _ => Err("expected a completed search".into()),
    }
}
#[test]
fn status_policy_does_not_diagnose_missing_status() -> Result<(), Box<dyn std::error::Error>> {
    for status in [408, 429, 500, 502, 503, 504] {
        let mut r = source()?;
        r.status = status;
        let session = Session::new(vec![r.clone(), later(&r, 10)]);
        assert_eq!(ids(&session)?, [RequestId(1)]);
    }
    for status in [200, 301, 400, 401, 403, 404, 501, 505] {
        let mut r = source()?;
        r.status = status;
        assert_eq!(
            Session::new(vec![r]).retry(RequestId(0)),
            Some(&RetryEvidence::NotApplicable)
        );
    }
    let mut r = source()?;
    r.status = 0;
    let session = Session::new(vec![r]);
    assert!(matches!(
        session.retry(RequestId(0)),
        Some(RetryEvidence::Unavailable(_))
    ));
    assert!(session.retry(RequestId(99)).is_none());
    Ok(())
}
#[test]
fn method_and_body_evidence_are_required_on_both_sides() -> Result<(), Box<dyn std::error::Error>> {
    let base = source()?;
    for method in ["POST", "PUT", "DELETE", "get"] {
        let mut r = base.clone();
        r.method = method.into();
        assert!(matches!(
            Session::new(vec![r]).retry(RequestId(0)),
            Some(RetryEvidence::Unavailable(_))
        ));
    }
    let mut head = base.clone();
    head.method = "HEAD".into();
    assert_eq!(
        ids(&Session::new(vec![
            head.clone(),
            later(&head, 10),
            later(&base, 11)
        ]))?,
        [RequestId(1)]
    );
    for size in [
        ReportedSize::Missing,
        ReportedSize::Unavailable,
        ReportedSize::Bytes(1),
    ] {
        let mut r = base.clone();
        r.request_body_size = size;
        assert!(matches!(
            Session::new(vec![r.clone()]).retry(RequestId(0)),
            Some(RetryEvidence::Unavailable(_))
        ));
        assert!(ids(&Session::new(vec![base.clone(), later(&r, 10)]))?.is_empty());
    }
    let mut r = base.clone();
    r.request_body = RequestBody::Text {
        mime_type: "text/plain".into(),
        text: String::new(),
    };
    assert!(matches!(
        Session::new(vec![r.clone()]).retry(RequestId(0)),
        Some(RetryEvidence::Unavailable(_))
    ));
    assert!(ids(&Session::new(vec![base, later(&r, 10)]))?.is_empty());
    Ok(())
}
#[test]
fn recorded_metadata_is_compared_without_sorting_or_dropping_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let mut r = source()?;
    r.cookies.push(Cookie {
        pair: NameValue {
            name: "session".into(),
            value: "synthetic".into(),
        },
        path: None,
        domain: None,
        expires: None,
        http_only: None,
        secure: Some(true),
    });
    let mut variants = vec![later(&r, 10); 5];
    variants[0].headers.reverse();
    variants[1].headers[0].value = "different".into();
    variants[2].cookies[0].secure = Some(false);
    variants[3].query.push(NameValue {
        name: "extra".into(),
        value: "1".into(),
    });
    variants[4].headers[0].name = "x-example".into();
    let mut requests = vec![r.clone()];
    requests.extend(variants);
    requests.push(later(&r, 10));
    assert_eq!(ids(&Session::new(requests))?, [RequestId(6)]);
    Ok(())
}
#[test]
fn time_window_retains_ties_without_inventing_direction() -> Result<(), Box<dyn std::error::Error>>
{
    let r = source()?;
    let session = Session::new(vec![
        r.clone(),
        later(&r, 30_011),
        later(&r, 30_010),
        later(&r, 0),
        later(&r, 9),
        later(&r, 10),
        later(&r, 10),
    ]);
    assert_eq!(ids(&session)?, [RequestId(5), RequestId(6), RequestId(2)]);
    let mut zero = r.clone();
    zero.total_ms = 0.0;
    assert_eq!(
        ids(&Session::new(vec![zero, later(&r, 0), later(&r, 1)]))?,
        [RequestId(2)]
    );
    for duration in [f64::NAN, f64::INFINITY, -1.0] {
        let mut invalid = r.clone();
        invalid.total_ms = duration;
        assert!(matches!(
            Session::new(vec![invalid]).retry(RequestId(0)),
            Some(RetryEvidence::Unavailable(_))
        ));
    }
    Ok(())
}
#[test]
fn url_comparison_does_not_merge_distinct_resources() -> Result<(), Box<dyn std::error::Error>> {
    let r = source()?;
    let mut requests = vec![r.clone()];
    for url in [
        "https://EXAMPLE.test:443/books#chapter",
        "http://example.test/books",
        "https://example.test:444/books",
        "https://example.test/Books",
        "https://example.test/books?x=1",
    ] {
        let mut next = later(&r, 10);
        next.url = url.into();
        requests.push(next);
    }
    assert_eq!(ids(&Session::new(requests))?, [RequestId(1)]);
    Ok(())
}
#[test]
fn results_are_bounded_without_requiring_success_or_interpreting_retry_after()
-> Result<(), Box<dyn std::error::Error>> {
    let mut r = source()?;
    r.response_headers.push(NameValue {
        name: "Retry-After".into(),
        value: "3600".into(),
    });
    let mut requests = vec![r.clone()];
    for ms in 10..31 {
        let mut next = later(&r, ms);
        next.status = 0;
        requests.push(next);
    }
    let session = Session::new(requests);
    assert_eq!(ids(&session)?.len(), 20);
    assert!(matches!(
        session.retry(RequestId(0)),
        Some(RetryEvidence::Candidates { more: true, .. })
    ));
    let fixture = import_bytes(
        include_bytes!("../../../tests/fixtures/har/retries.har"),
        Limits::default(),
    )?;
    assert_eq!(ids(&fixture)?, [RequestId(1), RequestId(2)]);
    assert!(
        matches!(fixture.retry(RequestId(3)),Some(RetryEvidence::Candidates { ids,more:false }) if ids == &[RequestId(4)])
    );
    Ok(())
}
