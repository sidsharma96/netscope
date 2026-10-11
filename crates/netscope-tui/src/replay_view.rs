//! Drafts, pending sends, and new results stay separate from the capture.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use netscope_core::{NameValue, ReplayDraft, ReplayReview, terminal_text};
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
    Running(u64),
    Result,
    Compare,
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
    pending: Option<(u64, ReplayReview)>,
    cancel: bool,
    started: Option<std::time::Instant>,
    result: Option<netscope_net::ReplayResult>,
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
            pending: None,
            cancel: false,
            started: None,
            result: None,
        }
    }
    /// Whether ordinary characters belong to a text field.
    pub fn editing(&self) -> bool {
        matches!(self.mode, Mode::Url(_) | Mode::Name(_) | Mode::Value(_, _))
    }
    pub(crate) fn waiting(&self) -> bool {
        matches!(self.mode, Mode::Running(_))
    }
    pub(crate) fn title(&self) -> &'static str {
        match self.mode {
            Mode::Running(_) => "SENDING",
            Mode::Result => "NEW RESULT",
            Mode::Compare => "RESPONSE COMPARISON",
            Mode::Review => "REVIEW",
            _ => "REPLAY DRAFT",
        }
    }
    pub(crate) fn footer(&self) -> &'static str {
        match self.mode {
            Mode::Running(_) => "Esc cancel · q / Ctrl+c quit and cancel",
            Mode::Result => "↑/↓ scroll · c compare · v reveal headers · Esc edit draft · q quit",
            Mode::Compare => "↑/↓ scroll both · v reveal headers · c / Esc result · q quit",
            Mode::Review => "↑/↓ scroll · v reveal headers · Enter / s send · Esc edit · q quit",
            _ => "↑/↓ move · Enter choose/save · Esc back · Ctrl+c quit",
        }
    }
    /// Borrow the current draft for inspection and tests.
    pub fn draft(&self) -> &ReplayDraft {
        &self.draft
    }
    /// Take a queued explicit Send. Drawing and editing never start I/O.
    pub fn take_send(&mut self) -> Option<(u64, ReplayReview)> {
        self.pending.take()
    }
    /// Instant at which the current Send was accepted, before worker dispatch.
    pub fn started_at(&self) -> Option<std::time::Instant> {
        self.started
    }
    /// Whether Esc requested cancellation of the active operation.
    pub fn cancellation_requested(&self) -> bool {
        self.cancel
    }
    /// Accept a matching terminal result once; stale results cannot replace it.
    pub fn complete(&mut self, id: u64, result: netscope_net::ReplayResult) {
        if !matches!(self.mode, Mode::Running(active) if active == id) {
            return;
        }
        self.result = Some(result);
        self.pending = None;
        self.mode = if self.cancel {
            Mode::Draft
        } else {
            Mode::Result
        };
        self.message = if self.cancel {
            "Operation stopped; o opens its retained result".into()
        } else {
            String::new()
        };
        self.cancel = false;
        self.offset = 0;
        self.reveal = false;
    }
    /// Report a failure to start, retaining the review for correction.
    pub fn start_failed(&mut self, id: u64) {
        if matches!(self.mode, Mode::Running(active) if active == id) {
            self.mode = Mode::Review;
            self.pending = None;
            self.message = "Could not start replay worker".into();
        }
    }
    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if matches!(self.mode, Mode::Running(_)) {
            if key.code == KeyCode::Esc {
                self.cancel = true;
            }
            return false;
        }
        if matches!(self.mode, Mode::Compare) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('c') => {
                    self.mode = Mode::Result;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.offset = self.offset.saturating_add(1),
                KeyCode::Char('v') => self.reveal = !self.reveal,
                _ => {}
            }
            return false;
        }
        if matches!(self.mode, Mode::Result) {
            match key.code {
                KeyCode::Char('c') => {
                    self.mode = Mode::Compare;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Esc => {
                    self.mode = Mode::Draft;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.offset = self.offset.saturating_add(1),
                KeyCode::Char('v') => self.reveal = !self.reveal,
                _ => {}
            }
            return false;
        }
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
                    match self.draft.review().and_then(|r| {
                        netscope_net::validate(&r)?;
                        Ok(r)
                    }) {
                        Ok(review) => {
                            static NEXT: std::sync::atomic::AtomicU64 =
                                std::sync::atomic::AtomicU64::new(1);
                            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            self.started = Some(std::time::Instant::now());
                            self.pending = Some((id, review));
                            self.mode = Mode::Running(id);
                            self.message.clear();
                            self.offset = 0;
                            self.reveal = false;
                        }
                        Err(reason) => self.message = reason.into(),
                    }
                }
                _ => {}
            }
            return false;
        }
        let captured = self.draft.captured().len();
        let added = self.draft.added().len();
        match key.code {
            KeyCode::Esc => return true,
            KeyCode::Char('o') if self.result.is_some() => {
                self.mode = Mode::Result;
                self.offset = 0;
                self.reveal = false;
            }
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
                    match self.draft.review().and_then(|r| {
                        netscope_net::validate(&r)?;
                        Ok(r)
                    }) {
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
        rows.push("Review request".into());
        rows
    }
    fn content(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "Source #{} · GET · HTTP/1.1 · no body",
                self.draft.source().0
            ),
            "Only Enter or s from Review sends a request".into(),
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
            Mode::Running(_) => {
                if let Ok(review) = self.draft.review() {
                    lines.push(format!(
                        "Destination: {}",
                        terminal_text(&review.destination)
                    ));
                }
                lines.push(
                    if self.cancel {
                        "Cancelling; waiting for connection cleanup"
                    } else {
                        "Sending a new request · Esc cancels"
                    }
                    .into(),
                );
                lines.push("10 seconds total · 2 MiB body limit · no redirects or retries".into());
                lines.push(
                    "Cancellation stops local I/O; the server may already have acted.".into(),
                );
            }
            Mode::Compare => {}
            Mode::Result => {
                if let Some(result) = &self.result {
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
                    lines.push("Limits: 10 seconds total · 2 MiB response body".into());
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
                    lines.push("Semantic preview; header spelling/order may differ".into());
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
    pub(crate) fn draw(&mut self, frame: &mut Frame, area: Rect, session: &netscope_core::Session) {
        if matches!(self.mode, Mode::Compare) {
            if let Some(result) = &self.result
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
        let title = if matches!(self.mode, Mode::Review) {
            " Review · Esc edits · Enter or s sends "
        } else if matches!(self.mode, Mode::Running(_)) {
            " Sending · Esc cancels "
        } else if matches!(self.mode, Mode::Result) {
            " New result · ↑/↓ scroll · v reveals · Esc edits "
        } else {
            " Draft · Enter chooses · o last result · Esc returns "
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
