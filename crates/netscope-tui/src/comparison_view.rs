//! Paired rows keep both observations aligned while wrapping and scrolling.
use netscope_core::{Body, Request, terminal_text};
use netscope_net::{
    Outcome, ReplayResult,
    comparison::{captured_bytes, compare_body, compare_headers},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

fn preview(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        let mut end = text.len().min(1024);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        format!(
            "{}{}",
            terminal_text(&text[..end]),
            if end < text.len() {
                " [preview shortened]"
            } else {
                ""
            }
        )
    } else {
        let text = bytes
            .iter()
            .take(128)
            .map(|b| format!("{b:02x} "))
            .collect::<String>();
        format!(
            "{text}{}",
            if bytes.len() > 128 {
                "[binary preview shortened]"
            } else {
                "[hex bytes]"
            }
        )
    }
}
fn captured_body(body: &Body) -> String {
    match body {
        Body::NotRecorded => "Body not recorded".into(),
        Body::Undecoded { .. } => "Body retained in an undecoded representation".into(),
        _ => captured_bytes(body)
            .map(|bytes| format!("{} stored bytes; completeness unproven", bytes.len()))
            .unwrap_or_default(),
    }
}
fn outcome(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Complete => "Complete",
        Outcome::Cancelled => "Cancelled / partial",
        Outcome::TimedOut => "Timed out / partial",
        Outcome::SizeLimited => "Size limited / partial",
        Outcome::Failed(_) => "Failed / possibly partial",
    }
}
fn rows(captured: &Request, result: &ReplayResult, reveal: bool) -> Vec<(String, String)> {
    let mut rows = vec![
        (
            format!("Captured method: {}", terminal_text(&captured.method)),
            format!(
                "Reviewed: {}; {} body bytes",
                result.method.as_str(),
                result.request_body_bytes
            ),
        ),
        (
            format!("Source #{}", result.source.0),
            "New request; capture unchanged".into(),
        ),
        (
            terminal_text(&captured.url),
            terminal_text(&result.destination),
        ),
        (
            "Destinations".into(),
            if captured.url == result.destination {
                "Same stored URL text"
            } else {
                "Different stored URL text"
            }
            .into(),
        ),
        (
            format!(
                "Status: {}",
                if captured.status == 0 {
                    "unavailable".into()
                } else {
                    captured.status.to_string()
                }
            ),
            format!(
                "Status: {}",
                result
                    .status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "no response headers".into())
            ),
        ),
        (
            "Status comparison".into(),
            match result.status {
                Some(status) if captured.status != 0 => {
                    if status == captured.status {
                        "Same status"
                    } else {
                        "Different status"
                    }
                }
                _ => "Unavailable",
            }
            .into(),
        ),
        (
            format!("Reported: {}", terminal_text(&captured.response_version)),
            format!(
                "Received: {}",
                result
                    .version
                    .as_deref()
                    .map(terminal_text)
                    .unwrap_or_else(|| "unavailable".into())
            ),
        ),
        (
            format!("Reported total: {} ms", captured.total_ms),
            format!("New elapsed: {:.3} s", result.elapsed.as_secs_f64()),
        ),
        (
            "Different conditions may apply".into(),
            "No speed comparison inferred".into(),
        ),
        (
            captured_body(&captured.body),
            format!(
                "{}; {} retained bytes",
                outcome(result.outcome),
                result.body.len()
            ),
        ),
        (
            "Body bytes (positions start at 0)".into(),
            compare_body(captured, result).label(),
        ),
        (
            "BODY PREVIEW (bounded)".into(),
            "BODY PREVIEW (bounded)".into(),
        ),
        (
            captured_bytes(&captured.body)
                .map(preview)
                .unwrap_or_else(|| "Unavailable".into()),
            if result.status.is_some() {
                preview(&result.body)
            } else {
                "No response headers received".into()
            },
        ),
        (
            "RESPONSE HEADERS".into(),
            "Names ignore case; values/order exact".into(),
        ),
    ];
    let headers = compare_headers(captured, result);
    if headers.is_empty() {
        rows.push((
            "No stored headers".into(),
            if result.status.is_some() {
                "No received headers"
            } else {
                "No response headers received"
            }
            .into(),
        ));
    }
    for header in headers {
        let label = match header.equal {
            Some(true) => "Stored values match",
            Some(false) => "Stored values differ",
            None => "Comparison unavailable",
        };
        rows.push((terminal_text(&header.name), label.into()));
        let count = header.captured.len().max(header.replayed.len());
        for index in 0..count {
            let value = |v: Option<&&[u8]>, missing: &str| match v {
                Some(bytes) if reveal => match std::str::from_utf8(bytes) {
                    Ok(text) => terminal_text(text),
                    Err(_) => preview(bytes),
                },
                Some(_) => "[hidden; v reveals values]".into(),
                None => missing.into(),
            };
            rows.push((
                value(header.captured.get(index), "Not in stored header list"),
                value(
                    header.replayed.get(index),
                    if result.status.is_some() {
                        "Not in received header list"
                    } else {
                        "No response headers received"
                    },
                ),
            ));
        }
    }
    rows
}
pub(crate) fn draw(
    frame: &mut Frame,
    area: Rect,
    captured: &Request,
    result: &ReplayResult,
    reveal: bool,
    offset: &mut usize,
) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
    let blocks = [
        Block::default()
            .borders(Borders::ALL)
            .title(" Captured response "),
        Block::default()
            .borders(Borders::ALL)
            .title(" Replay response "),
    ];
    let areas = [blocks[0].inner(left), blocks[1].inner(right)];
    frame.render_widget(&blocks[0], left);
    frame.render_widget(&blocks[1], right);
    let mut columns = [Vec::new(), Vec::new()];
    for (a, b) in rows(captured, result, reveal) {
        let mut a = crate::view::wrap(&a, usize::from(areas[0].width).max(2));
        let mut b = crate::view::wrap(&b, usize::from(areas[1].width).max(2));
        let height = a.len().max(b.len()).max(1);
        a.resize(height, String::new());
        b.resize(height, String::new());
        columns[0].extend(a);
        columns[1].extend(b);
    }
    let height = usize::from(areas[0].height);
    *offset = (*offset).min(columns[0].len().saturating_sub(height));
    for (lines, area) in columns.into_iter().zip(areas) {
        frame.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .skip(*offset)
                    .take(height)
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            area,
        );
    }
}
