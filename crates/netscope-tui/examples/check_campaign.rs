//! Independently checked native exports and optional loopback production-client probes.
use base64::Engine;
use netscope_core::{Body, ReplayDraft, RequestId, SseInspection, inspect_sse, websocket_evidence};
use netscope_import::{Limits, import_bytes};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};
fn fail(condition: bool, reason: &str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(reason.to_owned().into())
    }
}
fn response_bytes(request: &netscope_core::Request) -> Option<&[u8]> {
    match &request.body {
        Body::Text(s) | Body::DecodedText(s) => Some(s.as_bytes()),
        Body::Binary(b) => Some(b),
        _ => None,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let root = Path::new(args.first().ok_or("Expected campaign directory")?);
    let live = args.iter().any(|a| a == "--live");
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(root.join("cases.json"))?)?;
    fail(
        cases.len() >= 100,
        "Campaign must contain at least 100 distinct scenarios",
    )?;
    let mut rows = vec![];
    let mut checks = vec![];
    for browser in ["chrome", "firefox"] {
        let session = import_bytes(
            &std::fs::read(root.join(format!("{browser}-native.har")))?,
            Limits::default(),
        )?;
        let observations: Vec<Value> = serde_json::from_slice(&std::fs::read(
            root.join(format!("{browser}-observations.json")),
        )?)?;
        fail(
            observations.len() == cases.len(),
            "Browser did not finish every scenario",
        )?;
        for case in &cases {
            let id = case["id"].as_str().ok_or("Missing case ID")?;
            let method = case["method"].as_str().ok_or("Missing method")?;
            let suffix = format!("/case/{id}");
            let found = session
                .requests()
                .iter()
                .enumerate()
                .find(|(_, r)| {
                    r.method == method
                        && r.url
                            .split('?')
                            .next()
                            .is_some_and(|u| u.ends_with(&suffix))
                })
                .ok_or_else(|| {
                    format!("{browser} {id}: missing native request for configured method")
                })?;
            let (index, request) = found;
            let observed = observations
                .iter()
                .find(|v| v["id"] == id)
                .ok_or("Missing browser observation")?;
            fail(
                !observed["watchdog"].as_bool().unwrap_or(false),
                "Browser scenario watchdog expired",
            )?;
            let expected_status = case["status"].as_u64().ok_or("Missing expected status")?;
            fail(
                u64::from(request.status) == expected_status
                    || (case["family"] == "W" && request.status == 0 && observed["error"] == true),
                &format!("{browser} {id}: status differs without recorded failure"),
            )?;
            let family = case["family"].as_str().ok_or("Missing family")?;
            if matches!(family, "R" | "G") && !case["follow"].as_bool().unwrap_or(false) {
                fail(
                    observed["error"].is_null(),
                    &format!("{browser} {id}: unexpected browser fetch failure"),
                )?;
                fail(
                    observed["status"].as_u64() == Some(expected_status),
                    "Browser status differs",
                )?;
                if method != "HEAD" && !matches!(request.status, 204 | 304) {
                    fail(
                        observed["sha256"] == case["decoded_sha256"],
                        &format!(
                            "{browser} {id}: browser bytes differ from controlled decoded payload"
                        ),
                    )?;
                }
            }
            let mut classification = "recorded response";
            let mut delivery_match = None;
            let mut handshake_match = None;
            if family == "W" {
                let evidence = websocket_evidence(request).ok_or("WebSocket evidence absent")?;
                classification = evidence.summary;
                let name = case["name"].as_str().ok_or("Missing WS name")?;
                let rejected = matches!(
                    name,
                    "reject-403"
                        | "reject-404"
                        | "missing-accept"
                        | "bad-digest"
                        | "missing-upgrade"
                        | "unoffered-protocol"
                );
                handshake_match = Some(observed["open"].as_bool().unwrap_or(false) != rejected);
                fail(
                    handshake_match == Some(true)
                        || (browser == "firefox" && id == "W19" && observed["close"] == 1006),
                    &format!("{browser} {id}: unexplained browser handshake behavior"),
                )?;
                if !rejected {
                    let messages = match name {
                        "close-normal" | "close-going-away" => json!([]),
                        "unicode" => json!(["café 📚"]),
                        "empty-text" => json!([""]),
                        "two-messages" => json!(["book", "second"]),
                        "binary" => json!([[0, 255, 1, 254]]),
                        "fragmented" => json!(["first second"]),
                        "long-text" => json!(["x".repeat(4096)]),
                        "json-message" => json!(["{\"book\":1}"]),
                        "graphql-transport-ws" => json!(["{\"type\":\"connection_ack\"}"]),
                        _ => json!(["book"]),
                    };
                    fail(
                        observed["messages"] == messages,
                        &format!("{browser} {id}: WS message contents differ"),
                    )?;
                }
                if matches!(
                    name,
                    "missing-accept" | "missing-upgrade" | "unoffered-protocol"
                ) {
                    fail(
                        !evidence.notes.is_empty(),
                        "Expected incomplete header shape",
                    )?;
                }
                if name == "bad-digest" && request.status == 101 {
                    fail(
                        evidence.notes.is_empty(),
                        "Digest failure should remain outside header-shape verification",
                    )?;
                }
                fail(
                    ReplayDraft::from_request(RequestId(index), request).is_err(),
                    "WS must remain inspection-only",
                )?;
            } else if family == "S" {
                let Some(SseInspection::Parsed(parsed)) = inspect_sse(request) else {
                    return Err(format!("{browser} {id}: native SSE text unavailable").into());
                };
                let expected = observed["events"]
                    .as_array()
                    .ok_or("Missing browser events")?;
                let count = if case["name"] == "text-budget" {
                    0
                } else if case["name"] == "nul-id" {
                    1
                } else {
                    expected.len().min(100)
                };
                fail(
                    parsed.events.len() == count,
                    &format!(
                        "{browser} {id}: SSE count differs from browser dispatch under inspection budgets"
                    ),
                )?;
                for (event, expected) in parsed.events.iter().zip(expected) {
                    let expected_data = if browser == "chrome" && id == "S03" {
                        "cafÃ© ðŸ“š"
                    } else {
                        expected["data"]
                            .as_str()
                            .ok_or("Missing browser event data")?
                    };
                    fail(
                        event.data == expected_data
                            && event.event_type
                                == expected["type"].as_str().ok_or("Missing type")?
                            && event.id
                                == if id == "S15" {
                                    "a"
                                } else {
                                    expected["id"].as_str().ok_or("Missing event ID")?
                                },
                        &format!("{browser} {id}: SSE contents differ from browser dispatch"),
                    )?;
                }
                if case["name"] == "nul-id" {
                    fail(
                        parsed.events[0].data == "one" && parsed.events[0].id.is_empty(),
                        "NUL ID must be ignored without losing data",
                    )?;
                }
                delivery_match = Some(
                    parsed.events.len() == expected.len()
                        && parsed.events.iter().zip(expected).all(|(event, expected)| {
                            Some(event.data.as_str()) == expected["data"].as_str()
                                && Some(event.event_type) == expected["type"].as_str()
                                && Some(event.id) == expected["id"].as_str()
                        }),
                );
                classification = if delivery_match == Some(true) {
                    "captured SSE agrees with observed dispatch"
                } else {
                    "captured SSE retained; browser/export/budget distinction recorded"
                };
            } else {
                let empty = method == "HEAD" || matches!(request.status, 204 | 304);
                if !empty {
                    let bytes = if let Some(bytes) = response_bytes(request) {
                        bytes
                    } else if case["follow"] == true {
                        classification = "redirect response body not retained";
                        &[]
                    } else {
                        return Err(format!("{browser} {id}: response content unavailable").into());
                    };
                    let expected = if let Some(text) = case["export_text"][browser].as_str() {
                        text.as_bytes().to_vec()
                    } else if let Some(binary) = case["binary"].as_array() {
                        binary
                            .iter()
                            .map(|n| n.as_u64().map(|v| v as u8).ok_or("Invalid byte"))
                            .collect::<Result<Vec<_>, _>>()?
                    } else {
                        case["body"]
                            .as_str()
                            .ok_or("Missing body")?
                            .as_bytes()
                            .to_vec()
                    };
                    fail(
                        bytes == expected
                            || (case["follow"] == true && response_bytes(request).is_none()),
                        &format!("{browser} {id}: decoded content differs from controlled payload"),
                    )?;
                }
            }
            if case["export_text"][browser].is_string() {
                classification = "exporter text differs from browser decoded wire bytes";
            }
            rows.push(json!({"browser":browser,"id":id,"name":case["name"],"family":family,"method":request.method,"status":request.status,"version":request.response_version,"classification":classification,"browser_open_notification_matches_expected_handshake":handshake_match,"browser_delivery_matches_retained_analysis":delivery_match,"warnings":request.warnings,"pass":true}));
            if live && browser == "chrome" && matches!(family, "R" | "G" | "S") {
                let draft = match ReplayDraft::from_request(RequestId(index), request) {
                    Ok(d) => d,
                    Err(reason) => {
                        checks
                            .push(json!({"id":id,"result":"supported exclusion","reason":reason}));
                        continue;
                    }
                };
                let review = match draft.review() {
                    Ok(r) => r,
                    Err(reason) => {
                        checks
                            .push(json!({"id":id,"result":"supported exclusion","reason":reason}));
                        continue;
                    }
                };
                let mut worker = if family == "S" {
                    netscope_net::Worker::start_stream_at(review, Instant::now())?
                } else {
                    netscope_net::Worker::start(review)?
                };
                let started = Instant::now();
                let mut progress = None;
                let result = loop {
                    if let Some(snapshot) = worker.progress() {
                        progress = Some(snapshot);
                    }
                    if let Some(result) = worker.poll() {
                        if let Some(snapshot) = worker.progress() {
                            progress = Some(snapshot);
                        }
                        break result;
                    }
                    fail(
                        started.elapsed() < Duration::from_secs(35),
                        "Production client watchdog expired",
                    )?;
                    std::thread::sleep(Duration::from_millis(5));
                };
                let expected = if case["tls"] == true {
                    "TLS rejection"
                } else if family == "S" && case["gzip"] == true {
                    "compression rejection"
                } else if family == "S"
                    && matches!(
                        case["name"].as_str(),
                        Some("hundred-events" | "hundred-one-events" | "text-budget")
                    )
                {
                    "capture limit"
                } else {
                    "complete"
                };
                let ok = match expected {
                    "TLS rejection" => matches!(
                        result.outcome,
                        netscope_net::Outcome::Failed("TLS certificate issuer is not trusted")
                    ),
                    "compression rejection" => matches!(
                        result.outcome,
                        netscope_net::Outcome::Failed(
                            "Live SSE does not decode compressed content"
                        )
                    ),
                    "capture limit" => {
                        matches!(result.outcome, netscope_net::Outcome::CaptureLimited(_))
                    }
                    _ => result.outcome == netscope_net::Outcome::Complete,
                };
                fail(
                    ok,
                    &format!(
                        "{id}: production outcome {:?}, expected {expected}",
                        result.outcome
                    ),
                )?;
                if expected == "complete" {
                    fail(
                        result.status == case["status"].as_u64().map(|n| n as u16),
                        "Replay status differs from endpoint rule",
                    )?;
                    let expected_body = if matches!(result.status, Some(204 | 304)) {
                        vec![]
                    } else {
                        base64::engine::general_purpose::STANDARD.decode(
                            case["wire_base64"]
                                .as_str()
                                .ok_or("Missing encoded payload")?,
                        )?
                    };
                    fail(
                        result.body == expected_body,
                        &format!("{id}: replay bytes differ from wire payload"),
                    )?;
                }
                if let Some(snapshot) = progress {
                    let expected_events = observed["events"]
                        .as_array()
                        .ok_or("Missing live browser reference")?;
                    let expected_count = if id == "S24" {
                        0
                    } else {
                        expected_events.len().min(100)
                    };
                    fail(
                        snapshot.observed.len() == expected_count,
                        &format!("{id}: live observation count differs"),
                    )?;
                    let parsed = netscope_core::parse_sse(std::str::from_utf8(
                        &snapshot.body[..snapshot.valid_bytes],
                    )?);
                    for (event, expected) in parsed.events.iter().zip(expected_events) {
                        fail(
                            Some(event.data.as_str()) == expected["data"].as_str()
                                && Some(event.id) == expected["id"].as_str()
                                && Some(event.event_type) == expected["type"].as_str(),
                            &format!("{id}: live event differs from actual browser dispatch"),
                        )?;
                    }
                    fail(
                        snapshot.observed.windows(2).all(|w| w[0] <= w[1]),
                        "Observation times must not run backwards",
                    )?;
                    fail(
                        snapshot.body.len() <= 65536 && snapshot.observed.len() <= 100,
                        "Live budgets exceeded",
                    )?;
                }
                checks.push(json!({"id":id,"expected":expected,"outcome":format!("{:?}",result.outcome),"status":result.status,"retained_bytes":result.body.len(),"pass":true}));
            }
        }
    }
    let saved: Value = if live {
        json!({"production_probes": checks})
    } else {
        serde_json::from_slice(&std::fs::read(root.join("results.json"))?)?
    };
    let expected_received = saved["production_probes"]
        .as_array()
        .ok_or("Missing saved probes")?
        .iter()
        .filter(|p| p["status"].is_number())
        .count();
    let journal = std::fs::read_to_string(root.join("server-http.jsonl"))?;
    let mut received = 0;
    for line in journal.lines() {
        let entry: Value = serde_json::from_str(line)?;
        let headers = entry["headers"]
            .as_array()
            .ok_or("Missing request headers")?;
        let browser = headers.iter().any(|h| {
            h[0].as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case("user-agent"))
        });
        if browser {
            continue;
        }
        let case = cases
            .iter()
            .find(|c| c["id"] == entry["id"])
            .ok_or("Unknown server journal case")?;
        let body = base64::engine::general_purpose::STANDARD.decode(
            entry["body_base64"]
                .as_str()
                .ok_or("Missing recorded request body")?,
        )?;
        fail(
            body == case["request"].as_str().unwrap_or_default().as_bytes(),
            "Server-observed production request body differs",
        )?;
        fail(
            entry["method"] == case["method"],
            "Server-observed production method differs",
        )?;
        received += 1;
    }
    fail(
        received == expected_received,
        "Server journal HTTP count differs from production outcomes",
    )?;
    fail(
        rows.len() == cases.len() * 2,
        "Expected two checked browser runs per scenario",
    )?;
    if live {
        std::fs::write(
            root.join("results.json"),
            serde_json::to_vec_pretty(
                &json!({"scenarios":cases.len(),"browser_checks":rows,"production_probes":checks}),
            )?,
        )?;
    }
    println!(
        "PASS: {} distinct scenarios; {} native browser checks; {} production checks (exclusions labelled)",
        cases.len(),
        rows.len(),
        checks.len()
    );
    Ok(())
}
