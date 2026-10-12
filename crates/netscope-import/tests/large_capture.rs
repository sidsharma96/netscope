//! Fixed workloads check identity and matching rules, never elapsed-time thresholds.
use netscope_core::{NameValue, RedirectEvidence, RequestId, RetryEvidence, Session};
use netscope_import::{Limits, import_bytes};

#[test]
fn indexed_matching_agrees_with_a_full_scan_at_window_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let seed = import_bytes(
        include_bytes!("../../../tests/fixtures/har/single-request.har"),
        Limits::default(),
    )?;
    for status in [503, 307] {
        let requests = (0..240)
            .map(|i| {
                let mut request = seed.requests()[0].clone();
                // Reverse source order and duplicate starts exercise stable chronological ties.
                let milliseconds = ((239 - i) / 2) * 500;
                request.instant = (
                    milliseconds / 1000,
                    (milliseconds % 1000) as u32 * 1_000_000,
                );
                request.total_ms = [0.0, 500.0, 1000.0, 60_000.0][i as usize % 4];
                request.status = status;
                request.response_headers = vec![NameValue {
                    name: "Location".into(),
                    value: "/api/books".into(),
                }];
                request
            })
            .collect();
        let session = Session::new(requests);
        for (index, source) in session.requests().iter().enumerate() {
            let expected: Vec<_> = session
                .chronological()
                .iter()
                .copied()
                .filter(|id| {
                    let candidate = &session.requests()[id.0];
                    let elapsed = (candidate.instant.0 - source.instant.0) as f64 * 1000.0
                        + (f64::from(candidate.instant.1) - f64::from(source.instant.1))
                            / 1_000_000.0;
                    candidate.instant > source.instant
                        && elapsed >= source.total_ms
                        && elapsed <= source.total_ms + 30_000.0
                })
                .collect();
            let (ids, more) = if status == 503 {
                let Some(RetryEvidence::Candidates { ids, more }) = session.retry(RequestId(index))
                else {
                    panic!("retry candidates")
                };
                (ids, more)
            } else {
                let Some(RedirectEvidence::Candidates { ids, more, .. }) =
                    session.redirect(RequestId(index))
                else {
                    panic!("redirect candidates")
                };
                (ids, more)
            };
            assert_eq!(ids, &expected[..expected.len().min(20)]);
            assert_eq!(*more, expected.len() > 20);
        }
    }
    Ok(())
}

#[test]
fn maximum_entry_count_imports_and_one_more_is_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let mut data: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/har/single-request.har"
    ))?;
    let entry = data["log"]["entries"][0].clone();
    data["log"]["entries"] = serde_json::Value::Array(vec![entry.clone(); 10_000]);
    let session = import_bytes(&serde_json::to_vec(&data)?, Limits::default())?;
    assert_eq!(session.chronological().first(), Some(&RequestId(0)));
    assert_eq!(session.chronological().last(), Some(&RequestId(9999)));
    data["log"]["entries"]
        .as_array_mut()
        .ok_or("entries")?
        .push(entry);
    assert!(import_bytes(&serde_json::to_vec(&data)?, Limits::default()).is_err());
    Ok(())
}
