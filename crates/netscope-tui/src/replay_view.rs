//! Drafts, pending sends, and new results stay separate from the capture.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use netscope_core::{NameValue, ReplayDraft, ReplayMethod, ReplayReview};

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
    Running(ActiveSend),
    Result,
    Compare,
}
// These values exist together only while this operation is running.
#[derive(Debug)]
struct ActiveSend {
    id: u64,
    started: std::time::Instant,
    pending: Option<ReplayReview>,
    cancel_requested: bool,
    streaming: bool,
    stream: Option<netscope_net::StreamSnapshot>,
}
// A retained observation keeps its own display data through later draft edits.
#[derive(Debug)]
struct CompletedSend {
    result: netscope_net::ReplayResult,
    streaming: bool,
    stream: Option<netscope_net::StreamSnapshot>,
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
    save_pending: Option<(String, netscope_core::SavedDraft)>,
    completed: Option<CompletedSend>,
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
            save_pending: None,
            completed: None,
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
    pub(crate) fn update_activity(&mut self, elapsed: std::time::Duration, reduced: bool) -> bool {
        let changed = self.waiting()
            && crate::presentation::activity(elapsed, reduced)
                != crate::presentation::activity(self.activity_elapsed, self.reduced_motion);
        self.activity_elapsed = elapsed;
        self.reduced_motion = reduced;
        changed
    }
    pub(crate) fn waiting(&self) -> bool {
        matches!(self.mode, Mode::Running(_))
    }
    pub(crate) fn title(&self) -> &'static str {
        match self.mode {
            Mode::Running(_) if self.streaming_requested() => "LIVE SSE",
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
            Mode::Running(_) if self.streaming_requested() => {
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
        matches!(&self.mode, Mode::Running(active) if active.streaming)
    }
    fn stream_snapshot(&self) -> Option<&netscope_net::StreamSnapshot> {
        if let Mode::Running(active) = &self.mode {
            active.stream.as_ref()
        } else {
            self.completed
                .as_ref()
                .and_then(|done| done.stream.as_ref())
        }
    }
    fn showing_stream(&self) -> bool {
        if self.waiting() {
            self.streaming_requested()
        } else {
            self.completed.as_ref().is_some_and(|done| done.streaming)
        }
    }
    fn result(&self) -> Option<&netscope_net::ReplayResult> {
        self.completed.as_ref().map(|done| &done.result)
    }
    pub(crate) fn progress(&mut self, id: u64, snapshot: netscope_net::StreamSnapshot) {
        if let Mode::Running(active) = &mut self.mode
            && active.id == id
        {
            active.stream = Some(snapshot);
        }
    }
    /// Take a queued explicit Send. Drawing and editing never start I/O.
    pub fn take_send(&mut self) -> Option<(u64, ReplayReview)> {
        if let Mode::Running(active) = &mut self.mode {
            active.pending.take().map(|review| (active.id, review))
        } else {
            None
        }
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
        if let Mode::Running(active) = &self.mode {
            Some(active.started)
        } else {
            None
        }
    }
    /// Whether Esc requested cancellation of the active operation.
    pub fn cancellation_requested(&self) -> bool {
        matches!(&self.mode, Mode::Running(active) if active.cancel_requested)
    }
    /// Accept a matching terminal result once; stale results cannot replace it.
    pub fn complete(&mut self, id: u64, result: netscope_net::ReplayResult) {
        if !matches!(&self.mode, Mode::Running(active) if active.id == id) {
            return;
        }
        let Mode::Running(active) = std::mem::replace(&mut self.mode, Mode::Draft) else {
            return;
        };
        // A cancellation request is not proof that it stopped the server's work.
        self.message = match (active.cancel_requested, &result.outcome) {
            (false, _) => String::new(),
            (true, netscope_net::Outcome::Cancelled) => {
                "Operation stopped; o opens its retained result".into()
            }
            (true, _) => "Finished before Esc took effect; o opens its retained result".into(),
        };
        self.completed = Some(CompletedSend {
            result,
            streaming: active.streaming,
            stream: active.stream,
        });
        self.mode = if active.cancel_requested {
            Mode::Draft
        } else {
            Mode::Result
        };
        self.offset = 0;
        self.reveal = false;
    }
    /// Report a failure to start, retaining the review for correction.
    pub fn start_failed(&mut self, id: u64) {
        if let Mode::Running(active) = &self.mode
            && active.id == id
        {
            self.mode = if active.streaming {
                Mode::StreamReview
            } else {
                Mode::Review
            };
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
        if let Mode::Running(active) = &mut self.mode {
            match key.code {
                KeyCode::Esc => active.cancel_requested = true,
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
                            self.mode = Mode::Running(ActiveSend {
                                id,
                                started: std::time::Instant::now(),
                                pending: Some(review),
                                cancel_requested: false,
                                streaming,
                                stream: None,
                            });
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
            KeyCode::Char('o') if self.completed.is_some() => {
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
}

mod render;

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
        assert!(editor.stream_snapshot().is_none());
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
        assert!(editor.stream_snapshot().is_some());
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
    #[test]
    fn a_failed_new_send_preserves_the_previous_stream_result() -> Result<(), &'static str> {
        let mut editor = editor()?;
        let mut counter = Some(1);
        editor.mode = Mode::StreamReview;
        editor.key(KeyEvent::from(KeyCode::Enter), &mut counter);
        let (id, review) = editor.take_send().ok_or("first send")?;
        let bytes = b"data: first\n\n".to_vec();
        editor.progress(
            id,
            netscope_net::StreamSnapshot {
                valid_bytes: bytes.len(),
                body: bytes.clone(),
                observed: vec![std::time::Duration::from_millis(12)],
            },
        );
        editor.complete(
            id,
            netscope_net::ReplayResult {
                method: review.method,
                request_body_bytes: 0,
                source: review.source,
                destination: review.destination,
                outcome: netscope_net::Outcome::Complete,
                status: Some(200),
                version: Some("HTTP/1.1".into()),
                headers: vec![],
                body: bytes,
                elapsed: std::time::Duration::from_millis(12),
            },
        );
        editor.mode = Mode::Review;
        editor.key(KeyEvent::from(KeyCode::Enter), &mut counter);
        let (second, _) = editor.take_send().ok_or("second send")?;
        assert!(!editor.streaming_requested());
        editor.start_failed(second);
        assert!(matches!(editor.mode, Mode::Review));
        assert!(editor.started_at().is_none());
        assert!(editor.take_send().is_none());
        editor.mode = Mode::Draft;
        editor.key(KeyEvent::from(KeyCode::Char('o')), &mut counter);
        let screen = editor.content().join("\n");
        assert!(screen.contains("Observed +0.012s"));
        assert!(screen.contains("first"));
        Ok(())
    }
}
