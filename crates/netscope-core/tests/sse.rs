//! Deterministic retained-text framing and inspection-budget checks.
use netscope_core::{SSE_EVENT_LIMIT, SSE_TEXT_LIMIT, SseLimit, parse_sse};

#[test]
fn mixed_line_endings_bom_comments_and_multiline_data() {
    let stream = parse_sse("\u{feff}: note\r\nevent: added\rdata: café\ndata: second\r\n\r\n");
    assert_eq!(stream.events.len(), 1);
    assert_eq!(stream.events[0].event_type, "added");
    assert_eq!(stream.events[0].data, "café\nsecond");
    assert_eq!(stream.comments, 1);
    assert_eq!(stream.pending, None);
    assert_eq!(stream.limit, None);
}
#[test]
fn empty_data_emits_but_comments_and_metadata_alone_do_not() {
    let stream = parse_sse(": comment\n\nevent: ignored\nid: x\n\ndata\n\ndata:\ndata\n\n");
    assert_eq!(stream.events.len(), 2);
    assert_eq!(stream.events[0].data, "");
    assert_eq!(stream.events[0].event_type, "message");
    assert_eq!(stream.events[1].data, "\n");
    assert_eq!(stream.events[1].id, "x");
    assert_eq!(parse_sse("").events.len(), 0);
}
#[test]
fn fields_are_literal_and_only_one_space_is_removed() {
    let stream = parse_sse(
        "Data: ignored\n data: ignored\ndata:  leading\ndata:\ttab\ndata: x:y\nevent: first\nevent: last\n\n",
    );
    assert_eq!(stream.ignored, 2);
    assert_eq!(stream.events[0].data, " leading\n\ttab\nx:y");
    assert_eq!(stream.events[0].event_type, "last");
    let stream = parse_sse("data: a\nevent:\n\ndata: b\n\n");
    assert!(stream.events.iter().all(|e| e.event_type == "message"));
    assert!(parse_sse("\u{feff}\u{feff}data: x\n\n").events.is_empty());
}
#[test]
fn ids_persist_reset_and_ignore_null_while_retry_is_not_a_timestamp() {
    let stream = parse_sse(
        "id: first\nretry: 001500\ndata: one\n\ndata: two\nid: bad\0id\nretry: +20\nretry: 1.5\nretry:\nretry: １２\n\nid\ndata: three\n\n",
    );
    assert_eq!(
        stream.events.iter().map(|e| e.id).collect::<Vec<_>>(),
        ["first", "first", ""]
    );
    assert_eq!(stream.last_id, "");
    assert_eq!(stream.retry_ms, Some("001500"));
    assert_eq!(stream.ignored, 5);
    let large = "9".repeat(200);
    let input = format!("retry: {large}\n\n");
    assert_eq!(parse_sse(&input).retry_ms, Some(large.as_str()));
}
#[test]
fn eof_never_supplies_a_blank_line_and_partial_lines_stay_in_tail() {
    for input in ["data: tail", "data: tail\n", "data: tail\r\n"] {
        let stream = parse_sse(input);
        assert!(stream.events.is_empty());
        assert_eq!(stream.pending, Some(input));
    }
    let stream = parse_sse("data: done\n\nid: tail\ndata: end");
    assert_eq!(stream.events[0].data, "done");
    assert_eq!(stream.last_id, "tail");
    assert_eq!(stream.pending, Some("id: tail\ndata: end"));
    assert!(parse_sse("data: x\r\r").pending.is_none());
    assert!(parse_sse("\n\n").events.is_empty());
}
#[test]
fn text_budget_respects_utf8_and_does_not_complete_a_cut_block() {
    let input = format!("data: {}界\n\n", "x".repeat(SSE_TEXT_LIMIT - 7));
    let stream = parse_sse(&input);
    assert_eq!(stream.limit, Some(SseLimit::Text));
    assert!(stream.events.is_empty());
    assert!(
        stream
            .pending
            .is_some_and(|tail| tail.len() <= SSE_TEXT_LIMIT)
    );
    let exact = format!(":{}\n\n", "x".repeat(SSE_TEXT_LIMIT - 3));
    assert_eq!(parse_sse(&exact).limit, None);
    let input = format!("data: first\n\n{input}");
    assert_eq!(parse_sse(&input).events[0].data, "first");
}
#[test]
fn event_budget_reports_more_events_without_claiming_the_total() {
    let exact = "data: x\n\n".repeat(SSE_EVENT_LIMIT);
    let stream = parse_sse(&exact);
    assert_eq!(stream.events.len(), SSE_EVENT_LIMIT);
    assert_eq!(stream.limit, None);
    let extra = format!("{exact}data: next\n\ndata: tail");
    let stream = parse_sse(&extra);
    assert_eq!(stream.events.len(), SSE_EVENT_LIMIT);
    assert_eq!(stream.limit, Some(SseLimit::Events));
    assert_eq!(stream.pending, None);
}
