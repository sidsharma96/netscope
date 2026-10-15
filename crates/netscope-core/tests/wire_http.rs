//! Fixed bytes rather than reconstructed HAR content or external network observations.
use netscope_core::wire_http::inspect_http;
#[test]
fn chunks_trailers_and_pipeline_have_explicit_wire_ranges() {
    let wire=b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3;tag=x\r\nabc\r\n2\r\nde\r\n0\r\nX-End: yes\r\n\r\nHTTP/1.1 204 Empty\r\n\r\n";
    let found = inspect_http(wire, false);
    assert_eq!(found.messages.len(), 2);
    assert!(found.stopped.is_none());
    assert_eq!(found.messages[0].body, b"abcde");
    assert_eq!(found.messages[0].chunks.len(), 3);
    assert_eq!(&wire[found.messages[0].chunks[0].line.clone()], b"3;tag=x");
    assert_eq!(&wire[found.messages[0].chunks[0].data.clone()], b"abc");
    assert!(found.messages[1].body.is_empty());
}
#[test]
fn every_chunked_prefix_keeps_absence_distinct_from_termination() {
    let wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n";
    for cut in 0..wire.len() {
        let parsed = inspect_http(&wire[..cut], false);
        assert!(parsed.messages.iter().all(|m| !m.framed));
    }
    assert!(inspect_http(wire, false).messages[0].framed);
    for tail in [
        b"xyz\r\n".as_slice(),
        b"ffffffffffffffffffffffff\r\n",
        b"3\r\nabcXX",
        b"0\r\n bad\r\n\r\n",
    ] {
        let mut wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        wire.extend_from_slice(tail);
        assert!(!inspect_http(&wire, false).messages[0].framed);
    }
}
#[test]
fn ambiguous_lengths_coding_upgrade_and_close_do_not_guess_later_messages() {
    for header in [
        "Content-Length: 1\r\nContent-Length: 1\r\n",
        "Transfer-Encoding: gzip, chunked\r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 2\r\n",
        "Content-Length: -1\r\n",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\n{header}\r\nxxHTTP/1.1 204 Empty\r\n\r\n");
        let parsed = inspect_http(bytes.as_bytes(), false);
        assert_eq!(parsed.messages.len(), 1);
        assert!(!parsed.messages[0].framed);
    }
    let close = b"HTTP/1.1 200 OK\r\n\r\ncontent";
    assert!(!inspect_http(close, false).messages[0].framed);
    assert!(inspect_http(close, true).messages[0].framed);
    let upgrade = b"HTTP/1.1 101 Switching\r\n\r\nGET / HTTP/1.1\r\n\r\n";
    assert_eq!(inspect_http(upgrade, true).messages.len(), 1);
    assert!(!inspect_http(upgrade, true).messages[0].framed);
}
#[test]
fn limits_and_non_http_starts_are_explicit() {
    let repeated = b"GET / HTTP/1.1\r\n\r\n".repeat(9);
    let parsed = inspect_http(&repeated, false);
    assert_eq!(parsed.messages.len(), 8);
    assert!(parsed.stopped.is_some());
    for bytes in [
        b"".as_slice(),
        b"\x16\x03\x03\0\x10encrypted".as_slice(),
        b"HEAD / HTTP/1.1\r\n\r\n",
        b"HTTP/1.1 999 invalid\r\n\r\n",
        b"GET / HTTP/1.1\r\n bad: folded\r\n\r\n",
    ] {
        let found = inspect_http(bytes, false);
        assert!(found.messages.is_empty());
        assert!(found.stopped.is_some());
    }
    let body = b"x".repeat(300000);
    let mut wire = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
    wire.extend(body);
    let parsed = inspect_http(&wire, true);
    assert!(!parsed.messages[0].framed);
    assert!(parsed.messages[0].body.len() < 262144);
}
#[test]
fn status_rule_precedes_transfer_and_length_ambiguity() {
    for status in ["100 Continue", "204 No Content", "304 Not Modified"] {
        let wire = format!(
            "HTTP/1.1 {status}\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\nHTTP/1.1 204 Empty\r\n\r\n"
        );
        let parsed = inspect_http(wire.as_bytes(), false);
        assert_eq!(parsed.messages.len(), 2, "{status}");
        assert!(parsed.messages[0].framed);
        assert_eq!(
            parsed.messages[0].framing,
            "Status-defined header-only boundary"
        );
    }
}
#[test]
fn chunk_size_may_have_whitespace_before_extension() {
    let wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3 ;name=v\r\nabc\r\n2\t;x\r\nde\r\n0\r\n\r\n";
    let parsed = inspect_http(wire, false);
    assert!(parsed.messages[0].framed, "{}", parsed.messages[0].framing);
    assert_eq!(parsed.messages[0].body, b"abcde");
    // Whitespace is allowed only before an extension, not after a bare size.
    let bare = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3 \r\nabc\r\n0\r\n\r\n";
    let parsed = inspect_http(bare, false);
    assert_eq!(parsed.messages[0].framing, "Malformed chunk size/extension");
}
