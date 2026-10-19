//! Drafts, pending sends, and new results stay separate from the capture.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use netscope_core::{NameValue, ReplayDraft, ReplayMethod, ReplayReview, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
enum Mode {
    Draft,
    Headers,
    Url(String),
    Body(String),
    Name(String),
    Value(String, String),
    SaveName(String),
    SavePath(String, String),
    SaveReview(String, netscope_core::SavedDraft),
    Review,
    StreamReview,
    Running(u64),
    Result,
    Compare,
}
/// UI state for a local draft. Every header value is masked until explicitly revealed.
#[derive(Debug)]
pub struct ReplayEditor {
    draft: ReplayDraft,
    cursor: crate::text_edit::Cursor,
    mode: Mode,
    row: usize,
    header_row: usize,
    review_tab: usize,
    offset: usize,
    reveal: bool,
    message: String,
    pending: Option<(u64, ReplayReview)>,
    save_pending: Option<(String, netscope_core::SavedDraft)>,
    cancel: bool,
    started: Option<std::time::Instant>,
    result: Option<netscope_net::ReplayResult>,
    streaming: bool,
    stream: Option<netscope_net::StreamSnapshot>,
    activity_elapsed: std::time::Duration,
    reduced_motion: bool,
}
impl ReplayEditor {
    pub(crate) fn new(draft: ReplayDraft) -> Self {
        Self {
            draft,
            cursor: crate::text_edit::Cursor::default(),
            mode: Mode::Draft,
            row: 1,
            header_row: 0,
            review_tab: 0,
            offset: 0,
            reveal: false,
            message: String::new(),
            pending: None,
            save_pending: None,
            cancel: false,
            started: None,
            result: None,
            streaming: false,
            stream: None,
            activity_elapsed: std::time::Duration::ZERO,
            reduced_motion: false,
        }
    }
    /// Whether ordinary characters belong to a text field.
    pub fn editing(&self) -> bool {
        matches!(
            self.mode,
            Mode::Url(_)
                | Mode::Body(_)
                | Mode::Name(_)
                | Mode::Value(_, _)
                | Mode::SaveName(_)
                | Mode::SavePath(_, _)
        )
    }
    // The terminal supplies time; drawing the same state produces the same cells.
    pub(crate) fn update_activity(&mut self, elapsed: std::time::Duration, reduced: bool) {
        self.activity_elapsed = elapsed;
        self.reduced_motion = reduced;
    }
    pub(crate) fn waiting(&self) -> bool {
        matches!(self.mode, Mode::Running(_))
    }
    pub(crate) fn title(&self) -> &'static str {
        match self.mode {
            Mode::Running(_) if self.streaming => "LIVE SSE",
            Mode::Running(_) => "SENDING",
            Mode::Result => "NEW RESULT",
            Mode::Compare => "RESPONSE COMPARISON",
            Mode::StreamReview => "REVIEW LIVE SSE",
            Mode::Review => "REVIEW",
            _ => "REPLAY DRAFT",
        }
    }
    pub(crate) fn footer(&self) -> &'static str {
        match self.mode {
            Mode::SaveReview(_, _) => "↑/↓ scroll · Enter writes file · Esc cancel · q quit",
            Mode::Body(_) => "Ctrl+n newline · Tab tab · Enter save · Esc cancel · Ctrl+c quit",
            Mode::Url(_)
            | Mode::Name(_)
            | Mode::Value(_, _)
            | Mode::SaveName(_)
            | Mode::SavePath(_, _) => {
                "Text input · Ctrl+u clear · Enter save · Esc cancel · Ctrl+c quit"
            }
            Mode::Running(_) if self.streaming => {
                "↑/↓ scroll · Esc cancel · q / Ctrl+c quit and cancel"
            }
            Mode::Running(_) => "Esc cancel · q / Ctrl+c quit and cancel",
            Mode::Result if !self.draft.has_capture() => {
                "↑/↓ scroll · v reveal headers · Esc edit draft · q quit"
            }
            Mode::Result => "↑/↓ scroll · c compare · v reveal headers · Esc edit draft · q quit",
            Mode::Compare => "↑/↓ scroll both · v reveal headers · c / Esc result · q quit",
            Mode::StreamReview => {
                "↑/↓ scroll · v headers · Enter / s start live SSE · Esc edit · q quit"
            }
            Mode::Headers => "↑/↓ choose · Enter include/remove · a add · v reveal · Esc done",
            Mode::Review if self.draft.method() == ReplayMethod::Post => {
                "←/→ section · ↑/↓ scroll · v reveal · Enter / s send · Esc edit"
            }
            Mode::Review => {
                "←/→ section · ↑/↓ scroll · v reveal · Enter / s send · l live SSE · Esc edit"
            }
            _ => "↑/↓ choose · Enter edit/review · t format · w save · Esc back · q quit",
        }
    }
    /// Borrow the current draft for inspection and tests.
    pub fn draft(&self) -> &ReplayDraft {
        &self.draft
    }
    pub(crate) fn streaming_requested(&self) -> bool {
        self.streaming
    }
    pub(crate) fn progress(&mut self, id: u64, snapshot: netscope_net::StreamSnapshot) {
        if matches!(self.mode, Mode::Running(active) if active == id) {
            self.stream = Some(snapshot);
        }
    }
    /// Take a queued explicit Send. Drawing and editing never start I/O.
    pub fn take_send(&mut self) -> Option<(u64, ReplayReview)> {
        self.pending.take()
    }
    /// Take an explicitly confirmed save for the I/O layer.
    pub fn take_save(&mut self) -> Option<(String, netscope_core::SavedDraft)> {
        self.save_pending.take()
    }
    /// Report file-writing success or a fixed error without altering the request.
    pub fn saved(&mut self, result: Result<(), &'static str>) {
        self.message = result.map_or_else(str::to_owned, |()| {
            "Draft saved; reopen with netscope --draft <file>".into()
        });
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
        // Esc can arrive after the worker finished but before this result was accepted,
        // so the message follows the outcome, not the cancel request.
        self.message = match (self.cancel, &result.outcome) {
            (false, _) => String::new(),
            (true, netscope_net::Outcome::Cancelled) => {
                "Operation stopped; o opens its retained result".into()
            }
            (true, _) => "Finished before Esc took effect; o opens its retained result".into(),
        };
        self.result = Some(result);
        self.pending = None;
        self.mode = if self.cancel {
            Mode::Draft
        } else {
            Mode::Result
        };
        self.cancel = false;
        self.offset = 0;
        self.reveal = false;
    }
    /// Report a failure to start, retaining the review for correction.
    pub fn start_failed(&mut self, id: u64) {
        if matches!(self.mode, Mode::Running(active) if active == id) {
            self.mode = if self.streaming {
                Mode::StreamReview
            } else {
                Mode::Review
            };
            self.pending = None;
            self.message = "Could not start replay worker".into();
        }
    }
    fn edit_text(&self) -> Option<&str> {
        match &self.mode {
            Mode::Url(s)
            | Mode::Body(s)
            | Mode::Name(s)
            | Mode::Value(_, s)
            | Mode::SaveName(s)
            | Mode::SavePath(_, s) => Some(s),
            _ => None,
        }
    }
    pub(crate) fn key(&mut self, key: KeyEvent, next_send_id: &mut Option<u64>) -> bool {
        let was_editing = self.editing();
        let leave = self.key_inner(key, next_send_id);
        if (!was_editing || key.code == KeyCode::Enter) && self.editing() {
            self.cursor.byte = self.edit_text().map_or(0, str::len);
        }
        leave
    }
    fn key_inner(&mut self, key: KeyEvent, next_send_id: &mut Option<u64>) -> bool {
        if matches!(self.mode, Mode::Running(_)) {
            match key.code {
                KeyCode::Esc => self.cancel = true,
                KeyCode::Up => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down => self.offset = self.offset.saturating_add(1),
                _ => {}
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
                KeyCode::Char('c') if self.draft.has_capture() => {
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
                    Mode::SaveName(name) => {
                        match netscope_core::SavedDraft::from_draft(name.clone(), &self.draft) {
                            Ok(_) => self.mode = Mode::SavePath(name, String::new()),
                            Err(reason) => {
                                self.message = reason.into();
                                self.mode = Mode::SaveName(name);
                            }
                        }
                    }
                    Mode::SavePath(name, path) => {
                        if path.is_empty() || path.chars().any(char::is_control) {
                            self.message = "Enter a file path without control characters".into();
                            self.mode = Mode::SavePath(name, path);
                        } else {
                            match netscope_core::SavedDraft::from_draft(name.clone(), &self.draft) {
                                Ok(saved) => {
                                    self.mode = Mode::SaveReview(path, saved);
                                    self.offset = 0;
                                    self.message.clear();
                                }
                                Err(reason) => {
                                    self.message = reason.into();
                                    self.mode = Mode::SaveName(name);
                                }
                            }
                        }
                    }
                    Mode::Url(value) => {
                        self.message = if self.draft.set_url(value) {
                            "URL changed; all header choices cleared; body cleared"
                        } else {
                            "URL unchanged"
                        }
                        .into();
                    }
                    Mode::Body(value) => {
                        if let Err(reason) = self.draft.set_body(value.clone()) {
                            self.mode = Mode::Body(value);
                            self.message = reason.into();
                        }
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
                            Ok(()) => {
                                self.message = "Header added".into();
                                self.mode = Mode::Headers;
                            }
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
            let body_edit = matches!(self.mode, Mode::Body(_));
            let text = match &mut self.mode {
                Mode::Url(s)
                | Mode::Body(s)
                | Mode::Name(s)
                | Mode::Value(_, s)
                | Mode::SaveName(s)
                | Mode::SavePath(_, s) => s,
                _ => return false,
            };
            if body_edit
                && ((key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('n'))
                    || key.code == KeyCode::Tab)
            {
                let value = if key.code == KeyCode::Tab { '\t' } else { '\n' };
                if !self.cursor.insert(text, value) {
                    self.message = "Text limit: 16 KiB".into();
                }
            } else if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('u')
            {
                self.cursor.clear(text);
            } else if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                match key.code {
                    KeyCode::Left => self.cursor.left(text),
                    KeyCode::Right => self.cursor.right(text),
                    KeyCode::Home => self.cursor.byte = 0,
                    KeyCode::End => self.cursor.end(text),
                    KeyCode::Backspace => self.cursor.backspace(text),
                    KeyCode::Delete => self.cursor.delete(text),
                    KeyCode::Char(c) if !c.is_control() => {
                        if !self.cursor.insert(text, c) {
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
        if matches!(self.mode, Mode::SaveReview(_, _)) {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Draft;
                    self.offset = 0;
                    self.message = "Save cancelled".into();
                }
                KeyCode::Up | KeyCode::Char('k') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.offset = self.offset.saturating_add(1),
                KeyCode::Enter => {
                    if let Mode::SaveReview(path, saved) =
                        std::mem::replace(&mut self.mode, Mode::Draft)
                    {
                        self.save_pending = Some((path, saved));
                        self.offset = 0;
                    }
                }
                _ => {}
            }
            return false;
        }
        if key.code == KeyCode::Char('v') {
            self.reveal = !self.reveal;
            return false;
        }
        if matches!(self.mode, Mode::Headers) {
            let count = self.draft.captured().len() + self.draft.added().len();
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Draft;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.header_row = self.header_row.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.header_row = (self.header_row + 1).min(count)
                }
                KeyCode::Char('a') => self.mode = Mode::Name(String::new()),
                KeyCode::Enter => {
                    self.message.clear();
                    let captured = self.draft.captured().len();
                    if self.header_row < captured {
                        if let Err(e) = self.draft.toggle(self.header_row) {
                            self.message = e.into();
                        }
                    } else if self.header_row < count {
                        self.draft.remove(self.header_row - captured);
                    } else {
                        self.mode = Mode::Name(String::new());
                    }
                }
                _ => {}
            }
            return false;
        }
        if matches!(self.mode, Mode::Review | Mode::StreamReview) {
            match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                    self.review_tab =
                        (self.review_tab + if key.code == KeyCode::Left { 2 } else { 1 }) % 3;
                    self.offset = 0;
                }

                KeyCode::Esc => {
                    self.mode = Mode::Draft;
                    self.offset = 0;
                    self.reveal = false;
                }
                KeyCode::Up | KeyCode::Char('k') => self.offset = self.offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.offset = self.offset.saturating_add(1),
                KeyCode::Char('l') => {
                    match self
                        .draft
                        .review()
                        .and_then(|r| netscope_net::validate_stream(&r))
                    {
                        Ok(()) => {
                            self.mode = Mode::StreamReview;
                            self.offset = 0;
                            self.message.clear();
                        }
                        Err(reason) => self.message = reason.into(),
                    }
                }
                KeyCode::Enter | KeyCode::Char('s') => {
                    let streaming = matches!(self.mode, Mode::StreamReview);
                    match self.draft.review().and_then(|r| {
                        if streaming {
                            netscope_net::validate_stream(&r)?;
                        } else {
                            netscope_net::validate(&r)?;
                        }
                        Ok(r)
                    }) {
                        Ok(review) => {
                            let Some(id) = *next_send_id else {
                                self.message =
                                    "Send IDs exhausted; restart Netscope to send again".into();
                                return false;
                            };
                            *next_send_id = id.checked_add(1);
                            self.started = Some(std::time::Instant::now());
                            self.pending = Some((id, review));
                            self.streaming = streaming;
                            self.stream = None;
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
        match key.code {
            KeyCode::Char('w') => {
                self.mode = Mode::SaveName(String::new());
                self.message.clear();
            }
            KeyCode::Char('m') => {
                self.draft.toggle_method();
                self.message = "Method changed; body cleared and format reset to plain text".into();
                self.reveal = false;
            }
            KeyCode::Char('t') => {
                self.draft.toggle_format();
                self.message = if self.draft.method() == ReplayMethod::Post {
                    "Body format changed; text preserved"
                } else {
                    "Choose POST to set a body format"
                }
                .into();
            }
            KeyCode::Char('b') => {
                if self.draft.method() == ReplayMethod::Post {
                    self.mode = Mode::Body(self.draft.body().into());
                    self.message.clear();
                } else {
                    self.message = "Choose POST before editing a body".into();
                }
            }
            KeyCode::Esc => return true,
            KeyCode::Char('o') if self.result.is_some() => {
                self.mode = Mode::Result;
                self.offset = 0;
                self.reveal = false;
            }
            KeyCode::Up | KeyCode::Char('k') => self.row = self.row.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.row = (self.row + 1).min(4),
            KeyCode::Enter => {
                self.message.clear();
                self.reveal = false;
                if self.row == 0 {
                    self.draft.toggle_method();
                    self.message =
                        "Method changed; body cleared and format reset to plain text".into();
                } else if self.row == 1 {
                    self.mode = Mode::Url(self.draft.url().into());
                } else if self.row == 2 {
                    self.mode = Mode::Headers;
                    self.header_row = 0;
                } else if self.row == 3 {
                    if self.draft.method() == ReplayMethod::Post {
                        self.mode = Mode::Body(self.draft.body().into());
                    } else {
                        self.message = "Choose POST before editing a body".into();
                    }
                } else {
                    self.review_tab = 0;
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
        let Some(snapshot) = &self.stream else {
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
    fn content(&self) -> Vec<String> {
        let (method, bytes) = if let Mode::Body(text) = &self.mode {
            (self.draft.method(), text.len())
        } else if matches!(self.mode, Mode::Result) {
            self.result
                .as_ref()
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
                    if self.cancel {
                        "Cancelling; waiting for connection cleanup"
                    } else {
                        "Sending a new request · Esc cancels"
                    }
                    .into(),
                );
                lines.push(
                    if self.streaming {
                        "30 seconds total · 64 KiB · 100 events · no reconnection"
                    } else {
                        "10 seconds total · 2 MiB body limit · no redirects or retries"
                    }
                    .into(),
                );
                if self.streaming {
                    lines.extend(self.stream_lines());
                }
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
                    if self.streaming {
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
        if matches!(self.mode, Mode::Draft | Mode::Headers) && self.result.is_some() {
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

fn preview(text: &str) -> String {
    let mut end = text.len().min(512);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut value = text[..end].to_owned();
    if end < text.len() {
        value.push_str("… [preview shortened]");
    }
    value
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    fn editor() -> Result<ReplayEditor, &'static str> {
        let saved = netscope_core::SavedDraft::decode(br#"{"version":1,"name":"Local","url":"http://127.0.0.1:1/events","method":"GET","format":"plain","body":"","headers":[]}"#)?;
        let mut editor = ReplayEditor::new(saved.to_draft()?);
        editor.mode = Mode::Review;
        Ok(editor)
    }
    #[test]
    fn animation_changes_only_the_active_indicator_and_never_queues_work()
    -> Result<(), Box<dyn std::error::Error>> {
        use ratatui::{Terminal, backend::TestBackend};
        let mut editor = editor()?;
        let mut counter = Some(1);
        editor.key(KeyEvent::from(KeyCode::Char('l')), &mut counter);
        let session = netscope_core::Session::new(vec![]);
        let mut terminal = Terminal::new(TestBackend::new(100, 30))?;
        editor.update_activity(std::time::Duration::from_millis(180), false);
        terminal.draw(|f| editor.draw(f, f.area(), &session))?;
        assert!(editor.take_send().is_none());
        editor.key(KeyEvent::from(KeyCode::Enter), &mut counter);
        let sent = editor.take_send().ok_or("missing explicit send")?;
        let mut images = Vec::new();
        for ms in [0, 180, 180] {
            editor.update_activity(std::time::Duration::from_millis(ms), false);
            terminal.draw(|f| editor.draw(f, f.area(), &session))?;
            images.push(terminal.backend().buffer().clone());
            assert!(editor.take_send().is_none());
            assert!(editor.take_save().is_none());
        }
        assert_ne!(images[0], images[1]);
        assert_eq!(images[1], images[2]);
        for ms in [0, 180] {
            editor.update_activity(std::time::Duration::from_millis(ms), true);
            terminal.draw(|f| editor.draw(f, f.area(), &session))?;
            images.push(terminal.backend().buffer().clone());
        }
        assert_eq!(images[3], images[4]);
        assert_eq!(sent.0, 1);
        assert_eq!(counter, Some(2));
        editor.key(KeyEvent::from(KeyCode::Esc), &mut counter);
        assert!(editor.cancellation_requested());
        Ok(())
    }
    #[test]
    fn live_review_is_separate_and_stale_progress_cannot_replace_it() -> Result<(), &'static str> {
        let mut editor = editor()?;
        let mut counter = Some(7);
        editor.key(KeyEvent::from(KeyCode::Char('l')), &mut counter);
        assert!(matches!(editor.mode, Mode::StreamReview));
        assert!(editor.take_send().is_none());
        assert_eq!(counter, Some(7));
        let review = editor.content().join("\n");
        assert!(review.contains("30 seconds total"));
        editor.key(KeyEvent::from(KeyCode::Right), &mut counter);
        assert!(
            editor
                .content()
                .join("\n")
                .contains("Accept: text/event-stream")
        );
        editor.key(KeyEvent::from(KeyCode::Enter), &mut counter);
        assert!(editor.streaming_requested());
        assert_eq!(editor.take_send().ok_or("No send")?.0, 7);
        editor.key(KeyEvent::from(KeyCode::Char('s')), &mut counter);
        assert!(editor.take_send().is_none());
        editor.progress(6, netscope_net::StreamSnapshot::default());
        assert!(editor.stream.is_none());
        editor.progress(
            7,
            netscope_net::StreamSnapshot {
                body: b"data: first\n\ndata: pending\n".to_vec(),
                valid_bytes: b"data: first\n\ndata: pending\n".len(),
                observed: vec![std::time::Duration::from_millis(12)],
            },
        );
        let live = editor.content().join("\n");
        assert!(live.contains("Observed +0.012s"));
        assert!(live.contains("Unfinished block"));
        editor.key(KeyEvent::from(KeyCode::Esc), &mut counter);
        assert!(editor.cancellation_requested());
        assert!(editor.stream.is_some());
        Ok(())
    }
    #[test]
    fn post_and_selected_accept_stay_in_review_without_send() -> Result<(), &'static str> {
        for post in [true, false] {
            let mut editor = editor()?;
            if post {
                editor.draft.toggle_method();
            } else {
                editor.draft.add(NameValue {
                    name: "Accept".into(),
                    value: "text/plain".into(),
                })?;
            }
            let mut counter = Some(1);
            editor.key(KeyEvent::from(KeyCode::Char('l')), &mut counter);
            assert!(matches!(editor.mode, Mode::Review));
            assert!(!editor.message.is_empty());
            assert!(editor.take_send().is_none());
            assert_eq!(counter, Some(1));
        }
        Ok(())
    }
}
