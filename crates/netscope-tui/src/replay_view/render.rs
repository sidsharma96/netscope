//! Presentation of an editor state; input and operation transitions stay in the parent.
use super::{Mode, ReplayEditor, preview};
use netscope_core::{ReplayMethod, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

impl ReplayEditor {
    fn value(&self, s: &str) -> String {
        if self.reveal {
            terminal_text(s)
        } else {
            "[hidden; v reveals all values]".into()
        }
    }
    pub(super) fn rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        for (i, h) in self.draft.captured().iter().enumerate() {
            rows.push(format!(
                "[{}] {}: {}{}",
                if self.draft.selected(i) { "x" } else { " " },
                terminal_text(&h.name),
                self.value(&h.value),
                self.draft
                    .header_error(i)
                    .map(|e| format!(" [blocked: {e}]"))
                    .unwrap_or_default()
            ));
        }
        for h in self.draft.added() {
            rows.push(format!(
                "Remove added {}: {}",
                terminal_text(&h.name),
                self.value(&h.value)
            ));
        }
        rows.push("Add a header".into());

        rows
    }
    fn stream_lines(&self) -> Vec<String> {
        let mut lines = vec![
            "LIVE SSE · observed by Netscope, separate from the HAR".into(),
            "Times start at accepted Send; frame batching can give events the same time.".into(),
        ];
        let Some(snapshot) = self.stream_snapshot() else {
            lines.push(
                if matches!(self.mode, Mode::Result) {
                    "No valid event-stream observations available"
                } else {
                    "Waiting for valid event-stream response data"
                }
                .into(),
            );
            return lines;
        };
        let text = std::str::from_utf8(&snapshot.body[..snapshot.valid_bytes]).unwrap_or_default();
        let parsed = netscope_core::parse_sse(text);
        lines.push(format!(
            "{} complete events · {} raw bytes · {} comments",
            snapshot.observed.len(),
            snapshot.body.len(),
            parsed.comments
        ));
        for (event, time) in parsed.events.iter().zip(&snapshot.observed) {
            lines.push(format!(
                "Observed +{:.3}s · type {:?} · id {:?}",
                time.as_secs_f64(),
                preview(event.event_type),
                preview(event.id)
            ));
            lines.push(format!("  data {:?}", preview(&event.data)));
        }
        if let Some(pending) = parsed.pending {
            lines.push(format!(
                "Unfinished block (not an event): {:?}",
                preview(pending)
            ));
        }
        if snapshot.valid_bytes < snapshot.body.len() {
            lines.push("Trailing bytes are not complete valid UTF-8".into());
        }
        lines.push(format!(
            "Retry instruction: {:?}; no reconnect is scheduled",
            parsed.retry_ms.map(preview)
        ));
        lines
    }
    pub(super) fn content(&self) -> Vec<String> {
        let (method, bytes) = if let Mode::Body(text) = &self.mode {
            (self.draft.method(), text.len())
        } else if matches!(self.mode, Mode::Result) {
            self.result()
                .map(|r| (r.method, r.request_body_bytes))
                .unwrap_or((self.draft.method(), self.draft.body().len()))
        } else {
            (self.draft.method(), self.draft.body().len())
        };
        let mut lines = vec![
            format!(
                "{} · {} · HTTP/1.1 · {}",
                if self.draft.has_capture() {
                    format!("Source #{}", self.draft.source().0)
                } else {
                    "Saved draft · no capture".into()
                },
                method.as_str(),
                if method == ReplayMethod::Get {
                    "no body".into()
                } else {
                    format!("{bytes} request body bytes")
                }
            ),
            String::new(),
        ];
        if matches!(
            self.mode,
            Mode::Draft | Mode::Headers | Mode::Review | Mode::StreamReview
        ) {
            lines.pop();
        }
        if !self.message.is_empty() {
            lines.push(self.message.clone());
        }
        match &self.mode {
            Mode::Draft => {
                if let Err(e) = self.draft.review() {
                    lines.push(format!("Draft: {e}"));
                }
                let selected = self
                    .draft
                    .captured()
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| self.draft.selected(*i))
                    .count()
                    + self.draft.added().len();
                let fields = [
                    format!("Method   {}", self.draft.method().as_str()),
                    format!("URL      {}", terminal_text(self.draft.url())),
                    format!("Headers  {selected} included"),
                    if self.draft.method() == ReplayMethod::Get {
                        "Body     None (GET)".into()
                    } else {
                        format!(
                            "Body     {} bytes · {}",
                            self.draft.body().len(),
                            self.draft.format().content_type()
                        )
                    },
                    "[ Review request → ]".into(),
                ];
                lines.push(String::new());
                for (i, field) in fields.iter().enumerate() {
                    lines.push(format!("{} {field}", if i == self.row { ">" } else { " " }));
                }
            }
            Mode::Headers => {
                lines.push("Include captured fields · remove added fields".into());
                for (i, row) in self.rows().iter().enumerate() {
                    lines.push(format!(
                        "{} {row}",
                        if i == self.header_row { ">" } else { " " }
                    ));
                }
            }
            Mode::SaveReview(path, saved) => {
                lines.push(format!("SAVE TO: {}", terminal_text(path)));
                lines
                    .push("Enter replaces this file; Esc cancels. No request will be sent.".into());
                lines.push(
                    "Plain-text file: URL, body and other headers may contain secrets.".into(),
                );
                lines.push("Known authorization, cookie and API-key headers are omitted.".into());
                lines.push("Exact file contents follow (scroll to inspect):".into());
                match saved.encode() {
                    Ok(text) => lines.extend(text.lines().map(terminal_text)),
                    Err(reason) => lines.push(reason.into()),
                }
            }
            Mode::Url(s)
            | Mode::Body(s)
            | Mode::Name(s)
            | Mode::Value(_, s)
            | Mode::SaveName(s)
            | Mode::SavePath(_, s) => {
                lines.push(
                    match self.mode {
                        Mode::SaveName(_) => {
                            "SAVE NAME · plain-text file; known credential headers omitted"
                        }
                        Mode::SavePath(_, _) => {
                            "SAVE PATH · existing file replaced only after confirmation"
                        }
                        Mode::Url(_) => "EDIT URL",
                        Mode::Body(_) => "EDIT BODY",
                        Mode::Name(_) => "NEW HEADER NAME",
                        _ => "NEW HEADER VALUE",
                    }
                    .into(),
                );
                lines.push(
                    "←/→ move · Home/End start/end · Backspace/Delete remove · Ctrl+u clear".into(),
                );
                lines.push(terminal_text(s));
                lines.push("".into());
            }
            Mode::Running(_) => {
                if let Ok(review) = self.draft.review() {
                    lines.push(format!(
                        "Destination: {}",
                        terminal_text(&review.destination)
                    ));
                }
                lines.push(
                    if self.cancellation_requested() {
                        "Cancelling; waiting for connection cleanup"
                    } else {
                        "Sending a new request · Esc cancels"
                    }
                    .into(),
                );
                lines.push(
                    if self.showing_stream() {
                        "30 seconds total · 64 KiB · 100 events · no reconnection"
                    } else {
                        "10 seconds total · 2 MiB body limit · no redirects or retries"
                    }
                    .into(),
                );
                if self.showing_stream() {
                    lines.extend(self.stream_lines());
                }
                lines.push(
                    "Cancellation stops local I/O; the server may already have acted.".into(),
                );
            }
            Mode::Compare => {}
            Mode::Result => {
                if let Some(result) = self.result() {
                    lines.push(format!(
                        "New request: {}",
                        terminal_text(&result.destination)
                    ));
                    let outcome = match result.outcome {
                        netscope_net::Outcome::Complete => "Complete",
                        netscope_net::Outcome::Cancelled => {
                            "Cancelled; server work cannot be undone"
                        }
                        netscope_net::Outcome::TimedOut => "Timed out; partial response retained",
                        netscope_net::Outcome::SizeLimited => {
                            "Size limited; partial response retained"
                        }
                        netscope_net::Outcome::CaptureLimited(reason) => reason,
                        netscope_net::Outcome::Failed(reason) => reason,
                    };
                    lines.push(format!(
                        "Outcome: {}{outcome}",
                        if matches!(result.outcome, netscope_net::Outcome::Failed(_)) {
                            "Failed: "
                        } else {
                            ""
                        }
                    ));
                    lines.push(format!(
                        "Elapsed: {:.3}s · retained {} bytes",
                        result.elapsed.as_secs_f64(),
                        result.body.len()
                    ));
                    if self.showing_stream() {
                        lines.extend(self.stream_lines());
                    }
                    if let Some(status) = result.status {
                        lines.push(format!(
                            "Response: {} · {status}",
                            result.version.as_deref().unwrap_or("unknown version")
                        ));
                        lines.push("Response headers (v reveals values):".into());
                        for (name, value) in &result.headers {
                            lines.push(format!(
                                "  {}: {}",
                                terminal_text(name),
                                self.value(&String::from_utf8_lossy(value))
                            ));
                        }
                        lines.push("Body preview (content encoding is not decoded):".into());
                        if let Ok(text) = std::str::from_utf8(&result.body) {
                            let mut end = text.len().min(16_384);
                            while !text.is_char_boundary(end) {
                                end -= 1;
                            }
                            lines.push(terminal_text(&text[..end]));
                            if end < text.len() {
                                lines.push("[preview truncated]".into());
                            }
                        } else {
                            lines.push(
                                result
                                    .body
                                    .iter()
                                    .take(256)
                                    .map(|b| format!("{b:02x} "))
                                    .collect(),
                            );
                            if result.body.len() > 256 {
                                lines.push("[binary preview truncated]".into());
                            }
                        }
                    } else {
                        lines.push("No response headers received".into());
                    }
                }
            }
            Mode::Review | Mode::StreamReview => {
                lines.push(
                    ["Summary", "Headers", "Body"]
                        .iter()
                        .enumerate()
                        .map(|(i, name)| {
                            if i == self.review_tab {
                                format!("[ {name} ]")
                            } else {
                                (*name).into()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("   "),
                );
                if let Ok(r) = self.draft.review() {
                    match self.review_tab {
                        0 => {
                            lines.push(format!(
                                "{} {}",
                                r.method.as_str(),
                                terminal_text(&r.destination)
                            ));
                            lines.push(format!(
                                "Headers: {} chosen + {} automatic",
                                r.headers.len(),
                                if r.method == ReplayMethod::Post {
                                    4
                                } else if matches!(self.mode, Mode::StreamReview) {
                                    3
                                } else {
                                    2
                                }
                            ));
                            lines.push(format!(
                                "Body: {} UTF-8 bytes · {}",
                                r.body.len(),
                                if r.method == ReplayMethod::Post {
                                    r.format.content_type()
                                } else {
                                    "no body"
                                }
                            ));
                            let omitted = self
                                .draft
                                .captured()
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| !self.draft.selected(*i))
                                .count();
                            lines.push(format!("Captured headers left out: {omitted}"));
                            if let Some(fragment) = r.fragment {
                                lines.push(format!(
                                    "Fragment omitted from request: {}",
                                    terminal_text(&fragment)
                                ));
                            }
                            lines.push("Original capture stays unchanged.".into());
                            lines.push(
                                if matches!(self.mode, Mode::StreamReview) {
                                    "LIVE SSE · 30 seconds total · 64 KiB · 100 events"
                                } else {
                                    "Limits: 10 seconds · 2 MiB response · no redirects"
                                }
                                .into(),
                            );
                        }
                        1 => {
                            lines.push("CHOSEN BY YOU".into());
                            if r.headers.is_empty() {
                                lines.push("None selected".into());
                            }
                            for h in r.headers {
                                lines.push(format!(
                                    "{}: {}",
                                    terminal_text(&h.name),
                                    self.value(&h.value)
                                ));
                            }
                            lines.push("AUTOMATIC · required for this request".into());
                            lines.push(format!("Host: {}", terminal_text(&r.authority)));
                            lines.push("Accept-Encoding: identity".into());
                            if matches!(self.mode, Mode::StreamReview) {
                                lines.push("Accept: text/event-stream (client managed)".into());
                            }
                            if r.method == ReplayMethod::Post {
                                lines.push(format!("Content-Type: {}", r.format.content_type()));
                                lines.push(format!("Content-Length: {}", r.body.len()));
                            }
                            lines.push("LEFT OUT · captured fields".into());
                            for (i, h) in self.draft.captured().iter().enumerate() {
                                if !self.draft.selected(i) {
                                    lines.push(format!(
                                        "{}{}",
                                        terminal_text(&h.name),
                                        self.draft
                                            .header_error(i)
                                            .map(|e| format!(": {e}"))
                                            .unwrap_or_default()
                                    ));
                                }
                            }
                            lines.push("Header spelling and order may differ on the wire.".into());
                        }
                        _ => {
                            lines.push(format!("OUTGOING BODY · {} UTF-8 bytes", r.body.len()));
                            lines.push(if r.method == ReplayMethod::Get {
                                "GET sends no body.".into()
                            } else if r.body.is_empty() {
                                "[empty body]".into()
                            } else {
                                terminal_text(&r.body)
                            });
                            lines.push("Control characters are escaped for display.".into());
                        }
                    }
                }
            }
        }
        lines
    }
    pub(crate) fn draw(&mut self, frame: &mut Frame, area: Rect, session: &netscope_core::Session) {
        let [steps, content] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        let active = match self.mode {
            Mode::Review | Mode::StreamReview => 1,
            Mode::Running(_) => 2,
            Mode::Result | Mode::Compare => 3,
            _ => 0,
        };
        let spans = ["Edit", "Review", "Send", "Result"]
            .iter()
            .enumerate()
            .map(|(i, name)| {
                ratatui::text::Span::styled(
                    if i == active {
                        format!("[ {} {name} ]{} ", i + 1, if i < 3 { " →" } else { "" })
                    } else {
                        format!(" {} {name}{} ", i + 1, if i < 3 { " →" } else { "" })
                    },
                    Style::default().fg(if i == active {
                        crate::workspace_view::MINT
                    } else {
                        Color::Gray
                    }),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(Line::from(spans)), steps);
        if matches!(self.mode, Mode::Compare | Mode::SaveReview(_, _)) {
            self.draw_panel(frame, content, session);
            return;
        }
        let mut context = self.context();
        if matches!(self.mode, Mode::Draft | Mode::Headers) && self.completed.is_some() {
            context.push("o opens the retained result (from an earlier send).".into());
        }
        if content.width < 100 {
            let note_height = (crate::view::wrap_words(
                &context.join(" "),
                usize::from(content.width.saturating_sub(2)),
            )
            .len()
                + 2)
            .min(6) as u16;
            let available = content.height.saturating_sub(note_height);
            let main_height = if self.editing() {
                available
            } else {
                (self
                    .content()
                    .iter()
                    .map(|s| {
                        crate::view::wrap_words(s, usize::from(content.width.saturating_sub(2)))
                            .len()
                    })
                    .sum::<usize>()
                    + 2)
                .min(usize::from(available)) as u16
            };
            let main = Rect {
                height: main_height,
                ..content
            };
            let note = Rect::new(content.x, main.bottom(), content.width, note_height);
            self.draw_panel(frame, main, session);
            let b = crate::workspace_view::block(String::new(), false);
            let inner = b.inner(note);

            frame.render_widget(
                Paragraph::new(context.join(" ")).wrap(ratatui::widgets::Wrap { trim: false }),
                inner,
            );
            return;
        }
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(58), Constraint::Min(1)]).areas(content);
        self.draw_panel(frame, left, session);
        let lines = context
            .iter()
            .flat_map(|s| {
                crate::view::wrap_words(s, usize::from(right.width.saturating_sub(2)))
                    .into_iter()
                    .chain(std::iter::once(String::new()))
            })
            .collect::<Vec<_>>();
        let right = Rect {
            height: right.height.min((lines.len() + 2) as u16),
            ..right
        };
        let b = crate::workspace_view::block(String::new(), false);
        let inner = b.inner(right);

        frame.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .map(crate::presentation::report_line)
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
    }
    fn context(&self) -> Vec<String> {
        match &self.mode {
            Mode::Draft if self.row == 0 => vec!["METHOD".into(), "GET asks for a resource. POST sends a body. Enter switches method and clears the body.".into()],
            Mode::Draft if self.row == 1 => vec!["DESTINATION".into(), "Changing the URL clears body and header choices. The capture stays unchanged.".into()],
            Mode::Draft if self.row == 2 => vec!["REQUEST HEADERS".into(), "Headers carry information such as the preferred response format. Values stay hidden until v.".into()],
            Mode::Draft if self.row == 3 => vec!["REQUEST BODY".into(), "The body is text sent to the server. POST allows a body; t switches plain text / JSON.".into()],
            Mode::Headers => vec!["CHOOSE WHAT LEAVES YOUR MACHINE".into(), "Choose which fields to send. Blocked entries explain why they cannot be copied.".into()],
            Mode::Draft => vec!["CHECK BEFORE SENDING".into(), "Review the destination, headers and body before sending.".into()],
            Mode::Review if self.review_tab == 1 => vec!["WHAT THE SERVER WILL RECEIVE".into(), "Automatic fields describe the destination and body. Left-out fields are not sent.".into()],
            Mode::Review if self.review_tab == 2 => vec!["TEXT AND BYTES".into(), "UTF-8 encodes text as bytes. Content-Length counts bytes, not letters.".into()],
            Mode::StreamReview => vec!["LIVE EVENT STREAM".into(), "The client supplies Accept: text/event-stream. Requires status 200 and uncompressed UTF-8 events. No reconnect is scheduled. s starts; Esc edits.".into()],
            Mode::Review => vec!["READY FOR YOUR DECISION".into(),"Sending makes a real request. The server may change data in response.".into()],
            Mode::Running(_) => vec!["WAITING FOR THE SERVER".into(),"Esc cancels local I/O. The server may already have acted; cancellation cannot undo that work.".into()],
            Mode::Result if self.draft.has_capture() => vec!["A NEW OBSERVATION".into(),"c compares with the captured response. A partial result contains only bytes received before the stop.".into()],
            Mode::Result => vec!["A NEW OBSERVATION".into(),"Esc returns to the saved draft. A partial result contains only bytes received before the stop.".into()],
            _ => vec!["EDITING LOCAL TEXT".into(),"Changes stay in this draft. Nothing is sent while editing.".into()],
        }
    }
    fn draw_panel(&mut self, frame: &mut Frame, area: Rect, session: &netscope_core::Session) {
        if matches!(self.mode, Mode::Compare) {
            if let Some(result) = self.completed.as_ref().map(|done| &done.result)
                && let Some(captured) = session.request(result.source)
            {
                crate::comparison_view::draw(
                    frame,
                    area,
                    captured,
                    result,
                    self.reveal,
                    &mut self.offset,
                );
            }
            return;
        }
        let title = if matches!(self.mode, Mode::SaveReview(_, _)) {
            " Save preview · Enter writes file · Esc cancels "
        } else if matches!(self.mode, Mode::Review | Mode::StreamReview) {
            " Request "
        } else if matches!(self.mode, Mode::Running(_)) {
            " Sending · Esc cancels "
        } else if matches!(self.mode, Mode::Result) {
            " New result · scroll for details "
        } else if self.editing() {
            " Edit "
        } else {
            " Request "
        };
        let title = if self.waiting() {
            format!(
                " {} Active · Esc cancels ",
                crate::presentation::activity(self.activity_elapsed, self.reduced_motion)
            )
        } else {
            title.into()
        };
        let block = crate::workspace_view::block(title, true);
        let inner = block.inner(area);
        let content = self.content();
        let mut lines: Vec<String> = content
            .iter()
            .flat_map(|s| {
                let width = usize::from(inner.width).max(2);
                if self.editing() {
                    crate::view::wrap(s, width)
                } else {
                    crate::view::wrap_words(s, width)
                }
            })
            .collect();
        let area = if self.editing() {
            area
        } else {
            Rect {
                height: area
                    .height
                    .min((lines.len() + 2).min(usize::from(u16::MAX)) as u16),
                ..area
            }
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let height = usize::from(inner.height);
        let cursor = self.edit_text().map(|text| {
            let width = usize::from(inner.width).max(2);
            let before_field: usize = content[..content.len() - 2]
                .iter()
                .map(|s| crate::view::wrap(s, width).len())
                .sum();
            let mut prefix = terminal_text(&text[..self.cursor.byte]);
            prefix.push(' ');
            let wrapped = crate::view::wrap(&prefix, width);
            let row = before_field + wrapped.len() - 1;
            let column = wrapped.last().map_or(0, |s| s.width().saturating_sub(1));
            // Reserve a blank row when the insertion point follows a full final row.
            let field_rows = crate::view::wrap(&terminal_text(text), width).len();
            if wrapped.len() > field_rows {
                lines.insert(before_field + field_rows, String::new());
            }
            (row, column)
        });
        if let Some((row, _)) = cursor {
            if row < self.offset {
                self.offset = row;
            }
            if row >= self.offset + height {
                self.offset = row + 1 - height;
            }
        } else if matches!(self.mode, Mode::Draft | Mode::Headers) {
            let selected = lines.iter().position(|s| s.starts_with("> ")).unwrap_or(0);
            if selected < self.offset {
                self.offset = selected;
            }
            if selected >= self.offset + height {
                self.offset = selected + 1 - height;
            }
        }
        self.offset = self.offset.min(lines.len().saturating_sub(height));
        frame.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .skip(self.offset)
                    .take(height)
                    .map(crate::presentation::report_line)
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
        if content
            .iter()
            .map(|s| crate::view::wrap_words(s, usize::from(inner.width).max(2)).len())
            .sum::<usize>()
            > height
            && !self.editing()
        {
            crate::view::line(
                frame,
                Rect::new(
                    area.x + 1,
                    area.bottom() - 1,
                    area.width.saturating_sub(2),
                    1,
                ),
                " ↑↓ / PgUp PgDn · more details ".into(),
                Style::default().fg(crate::presentation::MUTED),
            );
        }
        if let Some((row, column)) = cursor {
            frame.set_cursor_position((
                inner.x + column as u16,
                inner.y + (row - self.offset) as u16,
            ));
        }
    }
}
