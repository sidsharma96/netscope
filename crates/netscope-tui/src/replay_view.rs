//! Draft editing stays separate from the capture and has no send operation.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use netscope_core::{NameValue, ReplayDraft, terminal_text};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
enum Mode {
    Draft,
    Url(String),
    Name(String),
    Value(String, String),
    Review,
}
/// UI state for a local draft. Every header value is masked until explicitly revealed.
#[derive(Debug)]
pub struct ReplayEditor {
    draft: ReplayDraft,
    mode: Mode,
    row: usize,
    offset: usize,
    reveal: bool,
    message: String,
}
impl ReplayEditor {
    pub(crate) fn new(draft: ReplayDraft) -> Self {
        Self {
            draft,
            mode: Mode::Draft,
            row: 0,
            offset: 0,
            reveal: false,
            message: String::new(),
        }
    }
    /// Whether ordinary characters belong to a text field.
    pub fn editing(&self) -> bool {
        matches!(self.mode, Mode::Url(_) | Mode::Name(_) | Mode::Value(_, _))
    }
    /// Borrow the current draft for inspection and tests.
    pub fn draft(&self) -> &ReplayDraft {
        &self.draft
    }
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if self.editing() {
            if key.code == KeyCode::Esc {
                self.mode = Mode::Draft;
                self.message = "Edit cancelled".into();
                return false;
            }
            if key.code == KeyCode::Enter {
                let previous = std::mem::replace(&mut self.mode, Mode::Draft);
                match previous {
                    Mode::Url(value) => {
                        self.message = if self.draft.set_url(value) {
                            "URL changed; all header choices cleared"
                        } else {
                            "URL unchanged"
                        }
                        .into();
                    }
                    Mode::Name(name) => {
                        let header = NameValue {
                            name: name.clone(),
                            value: String::new(),
                        };
                        match netscope_core::validate_replay_header(&header) {
                            Ok(()) => {
                                self.message.clear();
                                self.mode = Mode::Value(name, String::new());
                            }
                            Err(reason) => {
                                self.message = reason.into();
                                self.mode = Mode::Name(name);
                            }
                        }
                    }
                    Mode::Value(name, value) => {
                        let header = NameValue {
                            name: name.clone(),
                            value: value.clone(),
                        };
                        match self.draft.add(header) {
                            Ok(()) => self.message = "Header added".into(),
                            Err(reason) => {
                                self.message = reason.into();
                                self.mode = Mode::Value(name, value);
                            }
                        }
                    }
                    _ => {}
                }
                self.reveal = false;
                return false;
            }
            let text = match &mut self.mode {
                Mode::Url(s) | Mode::Name(s) | Mode::Value(_, s) => s,
                _ => return false,
            };
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
                text.clear();
            } else if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                match key.code {
                    KeyCode::Backspace => {
                        if let Some((i, _)) = text.grapheme_indices(true).next_back() {
                            text.truncate(i);
                        }
                    }
                    KeyCode::Char(c) if !c.is_control() => {
                        if text.len() + c.len_utf8() <= 16_384 {
                            text.push(c);
                        } else {
                            self.message = "Text limit: 16 KiB".into();
                        }
                    }
                    _ => {}
                }
            }
            return false;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        if key.code == KeyCode::Char('v') {
            self.reveal = !self.reveal;
            return false;
        }
        if matches!(self.mode, Mode::Review) {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Draft;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.offset = self.offset.saturating_add(1),
                KeyCode::Enter | KeyCode::Char('s') => {
                    self.message = "Send disabled: network replay is not implemented".into()
                }
                _ => {}
            }
            return false;
        }
        let captured = self.draft.captured().len();
        let added = self.draft.added().len();
        match key.code {
            KeyCode::Esc => return true,
            KeyCode::Up | KeyCode::Char('k') => self.row = self.row.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.row = (self.row + 1).min(captured + added + 2)
            }
            KeyCode::Enter => {
                self.message.clear();
                self.reveal = false;
                if self.row == 0 {
                    self.mode = Mode::Url(self.draft.url().into());
                } else if self.row <= captured {
                    if let Err(e) = self.draft.toggle(self.row - 1) {
                        self.message = e.into();
                    }
                } else if self.row <= captured + added {
                    self.draft.remove(self.row - captured - 1);
                } else if self.row == captured + added + 1 {
                    self.mode = Mode::Name(String::new());
                } else {
                    match self.draft.review() {
                        Ok(_) => {
                            self.mode = Mode::Review;
                            self.offset = 0;
                        }
                        Err(e) => self.message = e.into(),
                    }
                }
            }
            _ => {}
        }
        false
    }
    fn value(&self, s: &str) -> String {
        if self.reveal {
            terminal_text(s)
        } else {
            "[hidden; v reveals all values]".into()
        }
    }
    fn rows(&self) -> Vec<String> {
        let mut rows = vec![format!("Edit URL: {}", terminal_text(self.draft.url()))];
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
        rows.push("Review request (Send remains disabled)".into());
        rows
    }
    fn content(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "Source #{} · GET · HTTP/1.1 planned · no body",
                self.draft.source().0
            ),
            "Send disabled: network replay is not implemented".into(),
        ];
        if !self.message.is_empty() {
            lines.push(self.message.clone());
        }
        match &self.mode {
            Mode::Draft => {
                if let Err(e) = self.draft.review() {
                    lines.push(format!("URL: {e}"));
                }
                for (i, row) in self.rows().iter().enumerate() {
                    lines.push(format!("{} {row}", if i == self.row { ">" } else { " " }));
                }
            }
            Mode::Url(s) | Mode::Name(s) | Mode::Value(_, s) => {
                lines.push(
                    match self.mode {
                        Mode::Url(_) => "EDIT URL",
                        Mode::Name(_) => "NEW HEADER NAME",
                        _ => "NEW HEADER VALUE",
                    }
                    .into(),
                );
                lines.push(
                    "Append text · Backspace deletes · Ctrl+u clears · Enter saves · Esc cancels"
                        .into(),
                );
                lines.push(terminal_text(s));
                lines.push("[insertion point at end]".into());
            }
            Mode::Review => {
                if let Ok(r) = self.draft.review() {
                    lines.push(format!(
                        "Effective destination: {}",
                        terminal_text(&r.destination)
                    ));
                    if let Some(fragment) = r.fragment {
                        lines.push(format!(
                            "Fragment omitted from request: {}",
                            terminal_text(&fragment)
                        ));
                    }
                    lines.push("Limits: 10 seconds total · 2 MiB response body (planned)".into());
                    lines.push("Explicit headers:".into());
                    if r.headers.is_empty() {
                        lines.push("  None selected".into());
                    }
                    for h in r.headers {
                        lines.push(format!(
                            "  {}: {}",
                            terminal_text(&h.name),
                            self.value(&h.value)
                        ));
                    }
                    lines.push(format!(
                        "Client-managed Host: {}",
                        terminal_text(&r.authority)
                    ));
                    lines.push("Client-managed Accept-Encoding: identity".into());
                    lines.push(
                        "No body framing, cookies, auth, or User-Agent added implicitly".into(),
                    );
                    lines.push(
                        "Semantic preview; wire spelling/order awaits the HTTP client".into(),
                    );
                    lines.push("Captured headers left out:".into());
                    for (i, h) in self.draft.captured().iter().enumerate() {
                        if !self.draft.selected(i) {
                            lines.push(format!(
                                "  {}{}",
                                terminal_text(&h.name),
                                self.draft
                                    .header_error(i)
                                    .map(|e| format!(": {e}"))
                                    .unwrap_or_default()
                            ));
                        }
                    }
                }
            }
        }
        lines
    }
    pub(crate) fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let title = if matches!(self.mode, Mode::Review) {
            " Review · Esc edits · Enter cannot send "
        } else {
            " Draft · Enter chooses · v reveals · Esc returns "
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines: Vec<String> = self
            .content()
            .iter()
            .flat_map(|s| crate::view::wrap(s, usize::from(inner.width).max(2)))
            .collect();
        let height = usize::from(inner.height);
        if self.editing() {
            self.offset = lines.len().saturating_sub(height);
        } else if matches!(self.mode, Mode::Draft) {
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
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            inner,
        );
    }
}
