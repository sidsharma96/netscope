//! Derived host counts and filtered identities from a fixed synthetic capture.
use netscope_core::{HostId, RequestId, Session};
use netscope_import::{Limits, import_bytes};
fn sample() -> Result<Session, Box<dyn std::error::Error>> {
    Ok(import_bytes(
        include_bytes!("../../../tests/fixtures/har/multi-host.har"),
        Limits::default(),
    )?)
}
#[test]
fn host_identity_counts_and_chronology_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let session = sample()?;
    assert_eq!(session.host_order(), [HostId(1), HostId(0), HostId(2)]);
    assert_eq!(
        session.host(HostId(1)).map(|h| h.name.as_str()),
        Some("api.example.test")
    );
    let summary = session.host_summary(HostId(1)).ok_or("host missing")?;
    assert_eq!(
        summary.request_ids,
        [RequestId(1), RequestId(2), RequestId(3)]
    );
    assert_eq!(summary.statuses.successful, 1);
    assert_eq!(summary.statuses.server_error, 1);
    assert_eq!(summary.statuses.not_recorded, 1);
    assert_eq!(summary.warning_requests, 1);
    assert_eq!(summary.summed_duration_ms, Some(120.0));
    let assets = session.host_summary(HostId(0)).ok_or("host missing")?;
    assert_eq!(assets.request_ids, [RequestId(5), RequestId(0)]);
    assert_eq!(assets.summed_duration_ms, Some(25.0));
    assert_eq!(assets.statuses.redirection, 1);
    let all: usize = session
        .hosts()
        .iter()
        .filter_map(|host| session.host_summary(host.id))
        .map(|h| h.request_ids.len())
        .sum();
    assert_eq!(all, 6);
    assert!(session.host_summary(HostId(99)).is_none());
    Ok(())
}
#[test]
fn duration_overflow_and_zero_remain_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let session = sample()?;
    let mut requests = vec![session.requests()[0].clone(); 3];
    for request in &mut requests {
        request.total_ms = f64::MAX;
    }
    let huge = Session::new(requests);
    let summary = huge.host_summary(HostId(0)).ok_or("host missing")?;
    assert_eq!(summary.summed_duration_ms, None);
    assert_eq!(summary.request_ids.len(), 3);
    let mut zero = session.requests()[0].clone();
    zero.total_ms = 0.0;
    assert_eq!(
        Session::new(vec![zero])
            .host_summary(HostId(0))
            .map(|h| h.summed_duration_ms),
        Some(Some(0.0))
    );
    assert!(Session::new(vec![]).host_order().is_empty());
    Ok(())
}
#[test]
fn every_status_class_is_counted_without_diagnosing_status_zero()
-> Result<(), Box<dyn std::error::Error>> {
    let session = sample()?;
    let statuses = [0, 100, 199, 200, 299, 300, 399, 400, 499, 500, 599];
    let requests = statuses
        .iter()
        .map(|status| {
            let mut r = session.requests()[0].clone();
            r.status = *status;
            r.warnings.clear();
            r
        })
        .collect();
    let grouped = Session::new(requests);
    let summary = grouped.host_summary(HostId(0)).ok_or("host missing")?;
    let counts = &summary.statuses;
    assert_eq!(
        [
            counts.informational,
            counts.successful,
            counts.redirection,
            counts.client_error,
            counts.server_error,
            counts.not_recorded
        ],
        [2, 2, 2, 2, 2, 1]
    );
    assert_eq!(summary.warning_requests, 0);
    Ok(())
}

#[test]
fn sums_follow_source_order_even_when_chronological_order_differs()
-> Result<(), Box<dyn std::error::Error>> {
    let source = sample()?;
    let requests = [(3, 1e16), (1, 1.0), (2, 1.0)]
        .into_iter()
        .map(|(seconds, total)| {
            let mut request = source.requests()[0].clone();
            request.instant = (seconds, 0);
            request.total_ms = total;
            request.warnings = vec!["first warning".into(), "second warning".into()];
            request
        })
        .collect();
    let session = Session::new(requests);
    let summary = session.host_summary(HostId(0)).ok_or("host missing")?;
    assert_eq!(
        summary.request_ids,
        [RequestId(1), RequestId(2), RequestId(0)]
    );
    // Adding the small values first would retain their sum at this precision.
    assert_eq!(summary.summed_duration_ms, Some(1e16));
    assert_eq!(summary.warning_requests, 3);
    Ok(())
}
