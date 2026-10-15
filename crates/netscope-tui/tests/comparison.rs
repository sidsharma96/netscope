//! Compare observations with fixed data; no network or wall clock is involved.
use netscope_core::{Body, NameValue, Request, RequestId};
use netscope_import::{Limits, import_bytes};
use netscope_net::{
    Outcome, ReplayResult,
    comparison::{BodyComparison, compare_body, compare_headers},
};
fn observations() -> Result<(Request, ReplayResult), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/single-request.har"),
        Limits::default(),
    )?;
    let mut captured = session.requests()[0].clone();
    captured.body = Body::Text("abc".into());
    captured.response_headers.clear();
    let replay = ReplayResult {
        method: netscope_core::ReplayMethod::Get,
        request_body_bytes: 0,
        source: RequestId(0),
        destination: "http://localhost/different".into(),
        outcome: Outcome::Complete,
        status: Some(200),
        version: Some("HTTP/1.1".into()),
        headers: vec![],
        body: b"abc".to_vec(),
        elapsed: std::time::Duration::ZERO,
    };
    Ok((captured, replay))
}
#[test]
fn available_bytes_are_distinct_from_missing_and_undecoded_bodies()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut captured, mut replay) = observations()?;
    assert_eq!(compare_body(&captured, &replay), BodyComparison::Equal(3));
    captured.body = Body::DecodedText("abc".into());
    assert_eq!(compare_body(&captured, &replay), BodyComparison::Equal(3));
    captured.body = Body::Binary(vec![0xff, 0]);
    replay.body = vec![0xff, 0];
    assert_eq!(compare_body(&captured, &replay), BodyComparison::Equal(2));
    captured.body = Body::Text(String::new());
    replay.body.clear();
    assert_eq!(compare_body(&captured, &replay), BodyComparison::Equal(0));
    for body in [
        Body::NotRecorded,
        Body::Undecoded {
            encoded: "?".into(),
            reason: "invalid".into(),
        },
    ] {
        captured.body = body;
        assert!(matches!(
            compare_body(&captured, &replay),
            BodyComparison::Unavailable(_)
        ));
    }
    captured.body = Body::Text(String::new());
    replay.status = None;
    assert!(matches!(
        compare_body(&captured, &replay),
        BodyComparison::Unavailable(_)
    ));
    Ok(())
}
#[test]
fn incomplete_results_describe_only_available_prefixes() -> Result<(), Box<dyn std::error::Error>> {
    let (captured, mut replay) = observations()?;
    for outcome in [
        Outcome::Cancelled,
        Outcome::TimedOut,
        Outcome::SizeLimited,
        Outcome::Failed("read"),
    ] {
        replay.outcome = outcome;
        replay.body = b"ab".to_vec();
        assert_eq!(
            compare_body(&captured, &replay),
            BodyComparison::MatchingPrefix(2)
        );
        replay.body = b"abcd".to_vec();
        assert_eq!(
            compare_body(&captured, &replay),
            BodyComparison::MatchingPrefix(3)
        );
        replay.body = b"ax".to_vec();
        assert_eq!(
            compare_body(&captured, &replay),
            BodyComparison::DifferentPrefix(1)
        );
        replay.body.clear();
        assert_eq!(compare_body(&captured, &replay), BodyComparison::NoOverlap);
    }
    replay.outcome = Outcome::Complete;
    replay.body = b"ab".to_vec();
    assert_eq!(
        compare_body(&captured, &replay),
        BodyComparison::DifferentLengths
    );
    replay.body = b"axc".to_vec();
    assert_eq!(
        compare_body(&captured, &replay),
        BodyComparison::DifferentByte(1)
    );
    // With one side empty there are no shared bytes to call matching.
    replay.body.clear();
    let empty = compare_body(&captured, &replay);
    assert_eq!(empty.label(), "Stored lengths differ; one body is empty");
    Ok(())
}
#[test]
fn content_encoding_blocks_comparison_even_if_bytes_match() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut captured, mut replay) = observations()?;
    captured.response_headers.push(NameValue {
        name: "Content-Encoding".into(),
        value: "gzip".into(),
    });
    assert!(matches!(
        compare_body(&captured, &replay),
        BodyComparison::Unavailable(_)
    ));
    captured.response_headers.clear();
    replay
        .headers
        .push(("content-encoding".into(), b"br".to_vec()));
    assert!(matches!(
        compare_body(&captured, &replay),
        BodyComparison::Unavailable(_)
    ));
    replay.headers[0].1 = b"identity".to_vec();
    assert_eq!(compare_body(&captured, &replay), BodyComparison::Equal(3));
    Ok(())
}
#[test]
fn headers_ignore_name_case_but_keep_duplicate_values_and_order()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut captured, mut replay) = observations()?;
    captured.response_headers = vec![
        NameValue {
            name: "X-B".into(),
            value: "one".into(),
        },
        NameValue {
            name: "x-b".into(),
            value: "two".into(),
        },
        NameValue {
            name: "X-A".into(),
            value: "only capture".into(),
        },
    ];
    replay.headers = vec![
        ("x-b".into(), b"one".to_vec()),
        ("X-B".into(), b"two".to_vec()),
        ("X-C".into(), b"only replay".to_vec()),
    ];
    let rows = compare_headers(&captured, &replay);
    assert_eq!(
        rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["x-a", "x-b", "x-c"]
    );
    assert_eq!(
        rows.iter().map(|r| r.equal).collect::<Vec<_>>(),
        vec![Some(false), Some(true), Some(false)]
    );
    replay.headers.swap(0, 1);
    assert_eq!(compare_headers(&captured, &replay)[1].equal, Some(false));
    replay.headers[0].1 = b"one ".to_vec();
    assert_eq!(compare_headers(&captured, &replay)[1].equal, Some(false));
    replay.status = None;
    assert!(
        compare_headers(&captured, &replay)
            .iter()
            .all(|r| r.equal.is_none())
    );
    Ok(())
}
#[test]
fn differences_after_the_preview_limit_are_still_detected() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut captured, mut replay) = observations()?;
    captured.body = Body::Text("a".repeat(2048));
    replay.body = vec![b'a'; 2048];
    replay.body[2047] = b'b';
    assert_eq!(
        compare_body(&captured, &replay),
        BodyComparison::DifferentByte(2047)
    );
    Ok(())
}
