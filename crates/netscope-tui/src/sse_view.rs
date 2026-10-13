//! Display bounded derived events beside the retained response representation.
use netscope_core::{Request, SseInspection, SseLimit, inspect_sse};

fn preview(text: &str, limit: usize) -> String {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut value = format!("{:?}", &text[..end]);
    if end < text.len() {
        value.push_str(" … [display shortened]");
    }
    value
}
pub(crate) fn lines(request: &Request) -> Vec<String> {
    let Some(inspection) = inspect_sse(request) else {
        return Vec::new();
    };
    let mut lines = vec![
        String::new(),
        "SSE EVENTS · derived from retained text".into(),
        "Order only · event arrival times and actual browser delivery are not recorded".into(),
    ];
    match inspection {
        SseInspection::Unavailable(reason) => lines.push(format!("SSE unavailable: {reason}")),
        SseInspection::Parsed(stream) => {
            lines.push(format!(
                "{} complete events in examined text",
                stream.events.len()
            ));
            lines.push("ID buffer starts empty here; earlier connection state is unknown".into());
            for (i, event) in stream.events.iter().enumerate() {
                lines.push(format!(
                    "Event {} · type: {}",
                    i + 1,
                    preview(event.event_type, 256)
                ));
                lines.push(format!("  ID buffer: {}", preview(event.id, 256)));
                lines.push(format!("  Data: {}", preview(&event.data, 2048)));
            }
            lines.push(format!(
                "Examined comments: {} · ignored fields/values: {}",
                stream.comments, stream.ignored
            ));
            lines.push(format!(
                "Last examined ID buffer: {}",
                preview(stream.last_id, 256)
            ));
            lines.push(match stream.retry_ms {
                Some(value) => format!(
                    "Last retry setting: {} ms · instruction, not an observed retry",
                    preview(value, 256)
                ),
                None => "No valid retry setting in examined text".into(),
            });
            if let Some(pending) = stream.pending {
                lines.push("Unfinished trailing block · no event emitted from this block".into());
                lines.push(format!("  Retained tail: {}", preview(pending, 2048)));
            }
            if let Some(limit) = stream.limit {
                lines.push(match limit {
                    SseLimit::Text => "SSE analysis stopped at 64 KiB of retained text; later content not examined",
                    SseLimit::Events => "SSE analysis stopped before event 101; later content not examined",
                }.into());
            }
            lines
                .push("Blank-line boundaries do not establish that the capture is complete".into());
        }
    }
    lines.push(String::new());
    lines
}
