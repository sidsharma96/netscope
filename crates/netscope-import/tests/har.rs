//! Deterministic synthetic HAR acceptance tests.
use netscope_core::{Body, ReportedSize, RequestBody, RequestId, Timing, terminal_text};
use netscope_import::{ImportError, Limits, import_bytes, import_reader};
use serde_json::{Value, json};
const SINGLE: &[u8] = include_bytes!("../../../tests/fixtures/har/single-request.har");
fn fixture() -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(SINGLE)?)
}
fn load(value: &Value) -> Result<netscope_core::Session, ImportError> {
    import_bytes(value.to_string().as_bytes(), Limits::default())
}
fn fails(value: &Value, path: &str) {
    let result = load(value);
    assert!(
        matches!(result,Err(ref error) if error.path==path),
        "{result:?}"
    );
}
#[test]
fn source_identity_headers_and_timing_survive_import() -> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(SINGLE, Limits::default())?;
    let request = &session.requests()[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.status, 200);
    assert_eq!(
        request
            .headers
            .iter()
            .map(|h| h.value.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert_eq!(request.body, Body::Text("[]".into()));
    assert_eq!(request.timings.blocked, Timing::Missing);
    assert_eq!(request.timings.dns, Timing::NotApplicable);
    assert_eq!(request.timings.connect, Timing::Milliseconds(30.0));
    assert_eq!(request.timings.ssl, Timing::Milliseconds(20.0));
    assert!(request.warnings.is_empty());
    assert!(request.connection.is_none());
    assert!(request.server_ip.is_none());
    assert!(session.request(RequestId(99)).is_none());
    Ok(())
}
#[test]
fn chronological_order_retains_offset_ties() -> Result<(), Box<dyn std::error::Error>> {
    let session = import_bytes(
        include_bytes!("../../../tests/fixtures/har/stable-order.har"),
        Limits::default(),
    )?;
    assert_eq!(
        session.chronological(),
        [RequestId(1), RequestId(2), RequestId(0)]
    );
    Ok(())
}
#[test]
fn empty_bom_extensions_and_host_orders() -> Result<(), Box<dyn std::error::Error>> {
    let empty = import_bytes(
        include_bytes!("../../../tests/fixtures/har/empty.har"),
        Limits::default(),
    )?;
    assert!(empty.chronological().is_empty());
    let mut v = fixture()?;
    v["log"]["ignored"] = json!({"any":null});
    v["log"]["entries"][0]["request"]["method"] = json!("CUSTOM-VERB");
    let mut entry = v["log"]["entries"][0].clone();
    v["log"]["entries"][0]["request"]["url"] = json!("https://Z.example/a");
    entry["request"]["url"] = json!("https://a.example/b");
    if let Some(entries) = v["log"]["entries"].as_array_mut() {
        entries.push(entry);
    }
    let bytes = [&[0xef, 0xbb, 0xbf][..], v.to_string().as_bytes()].concat();
    let s = import_bytes(&bytes, Limits::default())?;
    assert_eq!(s.hosts()[0].name, "z.example");
    assert_eq!(s.host_order()[0].0, 1);
    assert_eq!(s.requests()[0].method, "CUSTOM-VERB");
    Ok(())
}
#[test]
fn known_field_failures_have_fixed_paths_and_do_not_leak_values()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("startedDateTime", json!("secret"), "startedDateTime"),
        ("time", json!(-1), "time"),
        ("serverIPAddress", json!("secret"), "serverIPAddress"),
        ("connection", json!(4), "connection"),
    ];
    for (key, value, path) in cases {
        let mut v = fixture()?;
        v["log"]["entries"][0][key] = value;
        fails(&v, &format!("log.entries[0].{path}"));
    }
    for (key, value) in [
        ("send", json!(-1)),
        ("wait", Value::Null),
        ("receive", json!("secret")),
        ("dns", json!(-2)),
    ] {
        let mut v = fixture()?;
        v["log"]["entries"][0]["timings"][key] = value;
        fails(&v, &format!("log.entries[0].timings.{key}"));
        if let Err(e) = load(&v) {
            assert!(!e.to_string().contains("secret"));
        }
    }
    for (key, value) in [
        ("url", json!("ftp://example.test")),
        ("method", json!("GET SPACE")),
        ("headers", Value::Null),
        ("httpVersion", Value::Null),
        ("bodySize", json!(-2)),
    ] {
        let mut v = fixture()?;
        v["log"]["entries"][0]["request"][key] = value;
        fails(&v, &format!("log.entries[0].request.{key}"));
    }
    for status in [json!(99), json!(600), json!(200.5), json!("200")] {
        let mut v = fixture()?;
        v["log"]["entries"][0]["response"]["status"] = status;
        fails(&v, "log.entries[0].response.status");
    }
    Ok(())
}
#[test]
fn errors_follow_source_then_validation_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut v = fixture()?;
    let mut entry = v["log"]["entries"][0].clone();
    entry["time"] = json!(-1);
    v["log"]["entries"][0]["timings"]["wait"] = Value::Null;
    v["log"]["entries"][0]["timings"]["receive"] = Value::Null;
    if let Some(entries) = v["log"]["entries"].as_array_mut() {
        entries.push(entry);
    }
    fails(&v, "log.entries[0].timings.wait");
    Ok(())
}
#[test]
fn body_representations_and_decode_limits_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
    for (text, encoding, expected) in [
        (None, None, Body::NotRecorded),
        (Some(""), None, Body::Text("".into())),
        (Some("a"), None, Body::Text("a".into())),
        (Some("aGk="), Some("base64"), Body::DecodedText("hi".into())),
        (Some("/w=="), Some("base64"), Body::Binary(vec![255])),
        (
            Some("??"),
            Some("base64"),
            Body::Undecoded {
                encoded: "??".into(),
                reason: "malformed base64".into(),
            },
        ),
        (
            Some("a"),
            Some("gzip"),
            Body::Undecoded {
                encoded: "a".into(),
                reason: "unsupported body encoding".into(),
            },
        ),
    ] {
        let mut v = fixture()?;
        v["log"]["entries"][0]["response"]["content"] = json!({"size":2,"mimeType":"text/plain"});
        if let Some(text) = text {
            v["log"]["entries"][0]["response"]["content"]["text"] = json!(text);
        }
        if let Some(encoding) = encoding {
            v["log"]["entries"][0]["response"]["content"]["encoding"] = json!(encoding);
        }
        assert_eq!(load(&v)?.requests()[0].body, expected);
    }
    let mut v = fixture()?;
    v["log"]["entries"][0]["response"]["content"]["text"] = json!("aGk=");
    v["log"]["entries"][0]["response"]["content"]["encoding"] = json!("base64");
    let limits = Limits {
        decoded_body_bytes: 1,
        ..Limits::default()
    };
    let s = import_bytes(v.to_string().as_bytes(), limits)?;
    assert!(
        matches!(&s.requests()[0].body,Body::Undecoded{reason,..} if reason=="decoded body limit exceeded")
    );
    let exact = import_bytes(
        v.to_string().as_bytes(),
        Limits {
            decoded_body_bytes: 2,
            ..limits
        },
    )?;
    assert_eq!(exact.requests()[0].body, Body::DecodedText("hi".into()));
    v["log"]["entries"][0]["response"]["content"]["encoding"] = Value::Null;
    fails(&v, "log.entries[0].response.content.encoding");
    Ok(())
}
#[test]
fn body_absence_encoding_types_and_http_compression() -> Result<(), Box<dyn std::error::Error>> {
    let s = import_bytes(
        include_bytes!("../../../tests/fixtures/har/missing-body.har"),
        Limits::default(),
    )?;
    assert_eq!(s.requests()[0].body, Body::NotRecorded);
    let mut v = fixture()?;
    v["log"]["entries"][0]["response"]["headers"] =
        json!([{"name":"Content-Encoding","value":"gzip"}]);
    assert_eq!(load(&v)?.requests()[0].body, Body::Text("[]".into()));
    v["log"]["entries"][0]["response"]["content"] =
        json!({"size":0,"mimeType":"","encoding":"unknown"});
    assert_eq!(load(&v)?.requests()[0].body, Body::NotRecorded);
    v["log"]["entries"][0]["response"]["content"]["encoding"] = json!(4);
    fails(&v, "log.entries[0].response.content.encoding");
    Ok(())
}
#[test]
fn form_fields_sizes_cookies_and_status_zero_remain_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let mut v = fixture()?;
    v["log"]["entries"][0]["request"]["postData"] = json!({"mimeType":"multipart/form-data","params":[{"name":"x","value":""},{"name":"x","fileName":"a.txt"}]});
    v["log"]["entries"][0]["request"]["cookies"] =
        json!([{"name":"x","value":"y","secure":false,"expires":"2026-10-09T12:00:00Z"}]);
    v["log"]["entries"][0]["response"]["status"] = json!(0);
    v["log"]["entries"][0]["connection"] = json!("");
    v["log"]["entries"][0]["serverIPAddress"] = json!("::1");
    if let Some(request) = v["log"]["entries"][0]["request"].as_object_mut() {
        request.remove("bodySize");
    }
    let s = load(&v)?;
    let r = &s.requests()[0];
    assert_eq!(r.status, 0);
    assert_eq!(r.connection.as_deref(), Some(""));
    assert_eq!(r.request_body_size, ReportedSize::Missing);
    assert_eq!(r.request_header_size, ReportedSize::Unavailable);
    assert_eq!(r.cookies[0].secure, Some(false));
    assert!(
        matches!(&r.request_body,RequestBody::Params{params,..} if params.len()==2 && params[1].value.is_none())
    );
    v["log"]["entries"][0]["request"]["postData"]["text"] = json!("");
    assert!(
        matches!(&load(&v)?.requests()[0].request_body, RequestBody::TextAndParams { text, params, .. } if text.is_empty() && params.len() == 2)
    );
    Ok(())
}
#[test]
fn warning_rules_preserve_reported_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let mut v = fixture()?;
    v["log"]["entries"][0]["time"] = json!(100);
    v["log"]["entries"][0]["timings"]["ssl"] = json!(40);
    let s = load(&v)?;
    assert_eq!(s.requests()[0].total_ms, 100.0);
    assert_eq!(s.requests()[0].warnings.len(), 2);
    v["log"]["entries"][0]["timings"]["connect"] = json!(-1);
    assert_eq!(load(&v)?.requests()[0].warnings.len(), 2);
    v["log"]["entries"][0]["timings"]["send"] = json!(f64::MAX);
    v["log"]["entries"][0]["timings"]["wait"] = json!(f64::MAX);
    assert!(
        load(&v)?.requests()[0]
            .warnings
            .iter()
            .any(|s| s.contains("reported total"))
    );
    Ok(())
}
#[test]
fn json_duplicate_members_and_limits_reject_whole_capture() -> Result<(), Box<dyn std::error::Error>>
{
    for bytes in [
        &b"{"[..],
        &b"{\"ignored\":{\"secret\":1,\"secret\":2}}"[..],
        &b"{} {}"[..],
        &b"{\"x\":NaN}"[..],
        &[0xff][..],
    ] {
        assert!(import_bytes(bytes, Limits::default()).is_err());
    }
    let mut v = fixture()?;
    v["log"]["version"] = json!("1.1");
    fails(&v, "log.version");
    assert!(
        import_bytes(
            SINGLE,
            Limits {
                entries: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        import_reader(
            SINGLE,
            Limits {
                input_bytes: SINGLE.len() - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        import_reader(
            SINGLE,
            Limits {
                input_bytes: SINGLE.len(),
                ..Limits::default()
            }
        )
        .is_ok()
    );
    let nested = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    assert!(import_bytes(nested.as_bytes(), Limits::default()).is_err());
    Ok(())
}
#[test]
fn previews_preserve_utf8_boundaries_and_escape_terminal_controls() {
    let body = Body::Text(format!("{}é", "a".repeat(16383)));
    let preview = body.preview();
    assert_eq!(preview.text.len(), 16383);
    assert!(preview.truncated);
    assert_eq!(terminal_text("\u{1b}[31m\n"), "\\u{1b}[31m\\n");
    assert!(!Body::Text("".into()).preview().truncated);
    assert!(Body::Binary(vec![0; 257]).preview().truncated);
}

#[test]
fn optional_stages_preserve_each_state_and_cookie_types_are_checked()
-> Result<(), Box<dyn std::error::Error>> {
    for (value, expected) in [
        (None, Timing::Missing),
        (Some(json!(-1)), Timing::NotApplicable),
        (Some(json!(0)), Timing::Milliseconds(0.0)),
        (Some(json!(3.5)), Timing::Milliseconds(3.5)),
    ] {
        let mut v = fixture()?;
        if let Some(timings) = v["log"]["entries"][0]["timings"].as_object_mut() {
            timings.remove("dns");
            if let Some(value) = value {
                timings.insert("dns".into(), value);
            }
        }
        assert_eq!(load(&v)?.requests()[0].timings.dns, expected);
    }
    let mut v = fixture()?;
    v["log"]["entries"][0]["request"]["cookies"] =
        json!([{"name":"x","value":"y","secure":"false"}]);
    fails(&v, "log.entries[0].request.cookies[0].secure");
    v["log"]["entries"][0]["request"]["cookies"] =
        json!([{"name":"x","value":"y","expires":"2026-10-09T12:00:00"}]);
    fails(&v, "log.entries[0].request.cookies[0].expires");
    Ok(())
}
