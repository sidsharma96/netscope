use netscope_core::{
    Body, Cookie, HostId, NameValue, ReportedSize, Request, RequestBody, RequestId, Session,
    Timing, terminal_text,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A keyboard action, independent of terminal event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move between the request list and its details.
    Focus,
    /// Choose a section of the captured request.
    Section(crate::InspectorTab),
    /// Toggle the explanatory request journey.
    Journey,
    /// Move up one request or inspector line.
    Up,
    /// Move down one request or inspector line.
    Down,
    /// Move upward by the current view's visible rows.
    PageUp,
    /// Move downward by the current view's visible rows.
    PageDown,
    /// Select the first row or scroll to the beginning.
    First,
    /// Select the last row or scroll to the end.
    Last,
    /// Inspect the selected request.
    Open,
    /// Return to overview, or close help.
    Back,
    /// Toggle help.
    Help,
    /// Open or leave the host view.
    Hosts,
    /// Restore the unfiltered request list.
    AllRequests,
    /// Open a separate replay draft.
    Replay,
    /// Edit the URL substring filter.
    Search,
    /// End the application.
    Quit,
}
/// The current view; inspector identity is a source ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Chronological request list.
    Overview,
    /// One captured request.
    Inspector(RequestId),
    /// Alphabetical host summaries.
    Hosts,
    /// Local replay draft and review for a captured request.
    Replay(RequestId),
}
/// Small mutable view state. The session remains separately owned.
#[derive(Debug)]
pub struct App {
    /// Selected inspector section, shared by preview and focused details.
    pub inspector_tab: crate::InspectorTab,
    /// Whether the explanatory journey is open.
    pub journey: bool,
    /// Selected reported stage in the journey, from Blocked through Receive.
    pub journey_stage: usize,
    /// Current view.
    pub screen: Screen,
    /// Selected row in the current request list; none when empty.
    pub selected: Option<usize>,
    /// First visible overview row.
    pub overview_offset: usize,
    /// First visible wrapped inspector line.
    pub inspector_offset: usize,
    /// Whether help is visible.
    pub help: bool,
    /// Whether quitting was requested.
    pub quit: bool,
    /// The active URL-host condition, or every host when absent.
    pub host_filter: Option<HostId>,
    /// Selected row in the alphabetical host list.
    pub host_selected: Option<usize>,
    /// First visible host row.
    pub host_offset: usize,
    /// Local editor, present only while a replay draft is open.
    pub replay: Option<crate::replay_view::ReplayEditor>,
    notice: String,
    query: String,
    search: Option<(String, crate::text_edit::Cursor)>,
    matches: Vec<RequestId>,
    // Allocation outlives editors; None means every available ID has been used.
    next_send_id: Option<u64>,
    all_requests_position: Option<(Option<usize>, usize)>,
    host_return: Screen,
    width: u16,
    height: u16,
}
impl App {
    /// Open independent saved data, without inventing a captured response.
    /// # Errors
    /// Rejects invalid or unsupported saved requests.
    pub fn from_saved(
        session: &Session,
        saved: &netscope_core::SavedDraft,
    ) -> Result<Self, &'static str> {
        let draft = saved.to_draft()?;
        let mut app = Self::new(session);
        app.screen = Screen::Replay(draft.source());
        app.replay = Some(crate::ReplayEditor::new(draft));
        Ok(app)
    }

    /// Initialise selections from the session rather than inventing empty rows.
    pub fn new(session: &Session) -> Self {
        Self {
            inspector_tab: crate::InspectorTab::Summary,
            journey: false,
            journey_stage: 5,
            screen: Screen::Overview,
            selected: (!session.requests().is_empty()).then_some(0),
            overview_offset: 0,
            inspector_offset: 0,
            help: false,
            quit: false,
            host_filter: None,
            host_selected: (!session.hosts().is_empty()).then_some(0),
            host_offset: 0,
            replay: None,
            notice: String::new(),
            query: String::new(),
            search: None,
            matches: Vec::new(),
            next_send_id: Some(1),
            all_requests_position: None,
            host_return: Screen::Overview,
            width: 100,
            height: 30,
        }
    }
    /// Borrow the chronological source IDs visible through the current filter.
    pub fn request_order<'a>(&'a self, session: &'a Session) -> &'a [RequestId] {
        if !self.query.is_empty() {
            return &self.matches;
        }
        match self.host_filter {
            None => session.chronological(),
            Some(id) => session
                .host_summary(id)
                .map(|summary| summary.request_ids.as_slice())
                .unwrap_or(&[]),
        }
    }
    fn selected_id(&self, session: &Session) -> Option<RequestId> {
        self.selected
            .and_then(|i| self.request_order(session).get(i))
            .copied()
    }
    fn rebuild_search(&mut self, session: &Session, selected: Option<RequestId>) {
        self.matches.clear();
        if !self.query.is_empty() {
            let order = match self.host_filter {
                None => session.chronological(),
                Some(id) => session
                    .host_summary(id)
                    .map_or(&[][..], |s| s.request_ids.as_slice()),
            };
            self.matches.extend(order.iter().copied().filter(|id| {
                session
                    .request(*id)
                    .is_some_and(|r| r.url.contains(&self.query))
            }));
        }
        self.selected = selected
            .and_then(|id| self.request_order(session).iter().position(|r| *r == id))
            .or_else(|| (!self.request_order(session).is_empty()).then_some(0));
        self.overview_offset = 0;
        self.clamp(session);
    }
    /// Record a resize; small terminals preserve selection and scroll state.
    pub fn resize(&mut self, width: u16, height: u16, session: &Session) {
        self.width = width;
        self.height = height;
        if self.usable() {
            self.clamp(session);
        }
    }
    fn usable(&self) -> bool {
        self.width >= 80 && self.height >= 24
    }
    fn clamp(&mut self, session: &Session) {
        let count = self.request_order(session).len();
        clamp_list(
            &mut self.selected,
            &mut self.overview_offset,
            count,
            usize::from(self.height.saturating_sub(9)),
        );
        clamp_list(
            &mut self.host_selected,
            &mut self.host_offset,
            session.host_order().len(),
            usize::from(self.height.saturating_sub(17)),
        );
        if let Screen::Inspector(id) = self.screen
            && let Some(request) = session.request(id)
        {
            self.journey_stage = self.journey_stage.min(crate::journey_view::STAGES - 1);
            let (len, rows) = if self.journey {
                let (width, rows) = crate::journey_view::detail_size(self.width, self.height);
                (
                    crate::journey_view::detail_lines(request, self.journey_stage, width).len(),
                    rows,
                )
            } else {
                let (width, rows) = crate::workspace_view::detail_size(self.width, self.height);
                (
                    crate::workspace_view::detail_lines(
                        session,
                        request,
                        id,
                        self.inspector_tab,
                        width,
                    )
                    .len(),
                    rows,
                )
            };
            self.inspector_offset = self
                .inspector_offset
                .min(len.saturating_sub(usize::from(rows)));
        }
    }
    fn show_hosts(&mut self, session: &Session) {
        self.host_return = self.screen;
        if let Some(host) = self.host_filter {
            self.host_selected = session.host_order().iter().position(|id| *id == host);
        }
        self.screen = Screen::Hosts;
    }
    fn show_all(&mut self, session: &Session) {
        self.journey = false;
        if self.host_filter.is_none() && self.query.is_empty() {
            self.screen = Screen::Overview;
            return;
        }
        let selected = self.selected_id(session);
        self.host_filter = None;
        self.query.clear();
        self.rebuild_search(session, selected);
        if let Some((selected, offset)) = self.all_requests_position.take() {
            self.selected = selected;
            self.overview_offset = offset;
        }
        self.screen = Screen::Overview;
    }
    /// Route text editing before global shortcuts; Ctrl+C always quits.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, session: &Session) {
        use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.search.is_some() {
            if !self.usable() {
                return;
            }
            if key.code == KeyCode::Esc {
                self.search = None;
                return;
            }
            if key.code == KeyCode::Enter {
                let selected = self.selected_id(session);
                if let Some((query, _)) = self.search.take() {
                    self.query = query;
                    // A saved row belongs to the old query's list.
                    self.all_requests_position = None;
                    self.rebuild_search(session, selected);
                }
                return;
            }
            if let Some((text, cursor)) = self.search.as_mut() {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
                    cursor.clear(text);
                } else if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    match key.code {
                        KeyCode::Left => cursor.left(text),
                        KeyCode::Right => cursor.right(text),
                        KeyCode::Home => cursor.byte = 0,
                        KeyCode::End => cursor.end(text),
                        KeyCode::Backspace => cursor.backspace(text),
                        KeyCode::Delete => cursor.delete(text),
                        KeyCode::Char(c)
                            if !c.is_control() && text.len() + c.len_utf8() <= 1024 =>
                        {
                            cursor.insert(text, c);
                        }
                        _ => {}
                    }
                }
            }
            return;
        }
        if let Screen::Replay(id) = self.screen {
            if let Some(editor) = self.replay.as_mut() {
                if !editor.editing() && key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
                    self.quit = true;
                    return;
                }
                if (self.width < 80 || self.height < 24) && !editor.waiting() {
                    return;
                }
                if editor.key(key, &mut self.next_send_id) {
                    if editor.draft().has_capture() {
                        self.replay = None;
                        self.screen = Screen::Inspector(id);
                    } else {
                        self.quit = true;
                    }
                }
            }
            return;
        }
        if let Some(action) = crate::terminal::key_action(key) {
            self.apply(action, session);
        }
    }
    /// Apply a key action without I/O or clock reads.
    pub fn apply(&mut self, action: Action, session: &Session) {
        if action == Action::Quit {
            self.quit = true;
            return;
        }
        if matches!(self.screen, Screen::Replay(_)) {
            return;
        }
        if !self.usable() {
            return;
        }
        if action == Action::Help {
            self.help = !self.help;
            return;
        }
        if self.help {
            if action == Action::Back {
                self.help = false;
            }
            return;
        }
        if matches!(self.screen, Screen::Overview | Screen::Inspector(_)) {
            match action {
                Action::Section(tab) => {
                    self.inspector_tab = tab;
                    self.inspector_offset = 0;
                    self.journey = false;
                    self.clamp(session);
                    return;
                }
                Action::Journey => {
                    if let Some(id) = self.selected_id(session) {
                        let active = self.journey && matches!(self.screen, Screen::Inspector(_));
                        self.screen = Screen::Inspector(id);
                        self.journey = !active;
                        self.journey_stage = 5;
                        self.inspector_offset = 0;
                        self.notice.clear();
                    }
                    self.clamp(session);
                    return;
                }
                Action::Focus => {
                    if matches!(self.screen, Screen::Inspector(_)) {
                        self.screen = Screen::Overview;
                        self.journey = false;
                    } else if let Some(id) = self.selected_id(session) {
                        self.screen = Screen::Inspector(id);
                    }
                    self.clamp(session);
                    return;
                }
                Action::Back if self.journey => {
                    self.journey = false;
                    self.inspector_offset = 0;
                    self.clamp(session);
                    return;
                }
                _ => {}
            }
        }
        if self.journey && matches!(self.screen, Screen::Inspector(_)) {
            match action {
                Action::Up | Action::Down | Action::First | Action::Last => {
                    self.journey_stage = match action {
                        Action::Up => self.journey_stage.saturating_sub(1),
                        Action::Down => self
                            .journey_stage
                            .saturating_add(1)
                            .min(crate::journey_view::STAGES - 1),
                        Action::First => 0,
                        _ => crate::journey_view::STAGES - 1,
                    };
                    self.inspector_offset = 0;
                    self.clamp(session);
                    return;
                }
                Action::PageUp | Action::PageDown => {
                    let (_, rows) = crate::journey_view::detail_size(self.width, self.height);
                    self.inspector_offset = if action == Action::PageUp {
                        self.inspector_offset.saturating_sub(usize::from(rows))
                    } else {
                        self.inspector_offset.saturating_add(usize::from(rows))
                    };
                    self.clamp(session);
                    return;
                }
                _ => {}
            }
        }
        if matches!(
            action,
            Action::PageUp | Action::PageDown | Action::First | Action::Last
        ) {
            let move_to = |position: usize, rows: u16| match action {
                Action::PageUp => position.saturating_sub(usize::from(rows.max(1))),
                Action::PageDown => position.saturating_add(usize::from(rows.max(1))),
                Action::First => 0,
                Action::Last => usize::MAX,
                _ => position,
            };
            match self.screen {
                Screen::Overview => {
                    self.selected = self
                        .selected
                        .map(|i| move_to(i, self.height.saturating_sub(9)));
                }
                Screen::Hosts => {
                    self.host_selected = self
                        .host_selected
                        .map(|i| move_to(i, self.height.saturating_sub(17)));
                }
                Screen::Inspector(_) => {
                    let (_, rows) = crate::workspace_view::detail_size(self.width, self.height);
                    self.inspector_offset = move_to(self.inspector_offset, rows);
                }
                Screen::Replay(_) => {}
            }
            self.clamp(session);
            return;
        }
        match (self.screen, action) {
            (Screen::Overview, Action::Search) => {
                let mut cursor = crate::text_edit::Cursor::default();
                cursor.end(&self.query);
                self.search = Some((self.query.clone(), cursor));
            }
            (Screen::Inspector(id), Action::Replay) => {
                if let Some(request) = session.request(id) {
                    match netscope_core::ReplayDraft::from_request(id, request) {
                        Ok(draft) => {
                            self.replay = Some(crate::replay_view::ReplayEditor::new(draft));
                            self.screen = Screen::Replay(id);
                            self.notice.clear();
                        }
                        Err(reason) => self.notice = reason.into(),
                    }
                }
            }
            (Screen::Hosts, Action::Hosts | Action::Back) => self.screen = self.host_return,
            (_, Action::Hosts) => self.show_hosts(session),
            (Screen::Overview | Screen::Hosts, Action::AllRequests) => self.show_all(session),
            (Screen::Hosts, Action::Up) => {
                self.host_selected = self.host_selected.map(|i| i.saturating_sub(1))
            }
            (Screen::Hosts, Action::Down) => {
                self.host_selected = self
                    .host_selected
                    .map(|i| (i + 1).min(session.hosts().len().saturating_sub(1)))
            }
            (Screen::Hosts, Action::Open) => {
                if let Some(id) = self
                    .host_selected
                    .and_then(|i| session.host_order().get(i))
                    .copied()
                {
                    if self.host_filter != Some(id) {
                        if self.host_filter.is_none() && self.query.is_empty() {
                            self.all_requests_position =
                                Some((self.selected, self.overview_offset));
                        }
                        let selected = self.selected_id(session);
                        self.host_filter = Some(id);
                        self.selected = Some(0);
                        self.overview_offset = 0;
                        if !self.query.is_empty() {
                            self.rebuild_search(session, selected);
                        }
                    }
                    self.screen = Screen::Overview;
                    self.journey = false;
                }
            }
            (Screen::Overview, Action::Back) if self.host_filter.is_some() => {
                self.show_hosts(session)
            }
            (Screen::Overview, Action::Down) => {
                self.inspector_offset = 0;
                self.selected = self
                    .selected
                    .map(|i| (i + 1).min(self.request_order(session).len().saturating_sub(1)))
            }
            (Screen::Overview, Action::Up) => {
                self.inspector_offset = 0;
                self.selected = self.selected.map(|i| i.saturating_sub(1))
            }
            (Screen::Overview, Action::Open) => {
                if let Some(id) = self
                    .selected
                    .and_then(|i| self.request_order(session).get(i))
                    .copied()
                {
                    self.screen = Screen::Inspector(id);
                    self.inspector_offset = 0;
                }
            }
            (Screen::Inspector(_), Action::Back) => self.screen = Screen::Overview,
            (Screen::Inspector(_), Action::Up) => {
                self.inspector_offset = self.inspector_offset.saturating_sub(1)
            }
            (Screen::Inspector(_), Action::Down) => {
                self.inspector_offset = self.inspector_offset.saturating_add(1)
            }
            _ => {}
        }
        self.clamp(session);
    }
}
fn clamp_list(selected: &mut Option<usize>, offset: &mut usize, count: usize, rows: usize) {
    let rows = rows.max(1);
    if count == 0 {
        *selected = None;
        *offset = 0;
        return;
    }
    let position = selected.unwrap_or(0).min(count - 1);
    *selected = Some(position);
    if position < *offset {
        *offset = position;
    }
    if position >= *offset + rows {
        *offset = position + 1 - rows;
    }
    *offset = (*offset).min(count.saturating_sub(rows));
}

pub(super) fn status(request: &Request) -> String {
    if request.status == 0 {
        "No HTTP status recorded".into()
    } else {
        format!("{} {}", request.status, terminal_text(&request.status_text))
    }
}
pub(super) fn timing(value: Timing) -> String {
    match value {
        Timing::Missing => "Not recorded".into(),
        Timing::NotApplicable => "Not applicable (-1)".into(),
        Timing::Milliseconds(n) => format!("{n} ms"),
        Timing::ExporterNegative(n) => format!("Exporter value {n} · not a duration"),
    }
}
fn size(value: ReportedSize) -> String {
    match value {
        ReportedSize::Missing => "Not recorded".into(),
        ReportedSize::Unavailable => "Unavailable (-1)".into(),
        ReportedSize::Bytes(n) => format!("{n} bytes"),
    }
}
pub(super) fn pairs(lines: &mut Vec<String>, title: &str, values: &[NameValue]) {
    lines.push(title.into());
    if values.is_empty() {
        lines.push("  No captured fields".into());
    }
    for value in values {
        lines.push(format!(
            "  {}: {}",
            terminal_text(&value.name),
            terminal_text(&value.value)
        ));
    }
}
pub(super) fn cookies(lines: &mut Vec<String>, title: &str, values: &[Cookie]) {
    lines.push(title.into());
    if values.is_empty() {
        lines.push("  No captured cookies".into());
    }
    for cookie in values {
        lines.push(format!(
            "  {}: {}",
            terminal_text(&cookie.pair.name),
            terminal_text(&cookie.pair.value)
        ));
        for (name, value) in [
            ("path", &cookie.path),
            ("domain", &cookie.domain),
            ("expires", &cookie.expires),
        ] {
            if let Some(value) = value {
                lines.push(format!("    {name}: {}", terminal_text(value)));
            }
        }
        for (name, value) in [("httpOnly", cookie.http_only), ("secure", cookie.secure)] {
            if let Some(value) = value {
                lines.push(format!("    {name}: {value}"));
            }
        }
    }
}
/// Inspector content before width-aware wrapping; captured values are escaped.
pub fn inspector_lines(request: &Request, id: RequestId) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Request #{} · {} · {}",
            id.0,
            terminal_text(&request.method),
            status(request)
        ),
        format!("URL: {}", terminal_text(&request.url)),
        format!("Started: {}", terminal_text(&request.started_at)),
        format!("Reported total: {} ms", request.total_ms),
        String::new(),
        "REQUEST · exporter-reported".into(),
        format!("HTTP version: {}", reported_label(&request.request_version)),
        format!("Header size: {}", size(request.request_header_size)),
        format!("Body size: {}", size(request.request_body_size)),
    ];
    pairs(&mut lines, "Request headers", &request.headers);
    pairs(&mut lines, "Query fields", &request.query);
    cookies(&mut lines, "Request cookies", &request.cookies);
    lines.extend(request_body_lines(request));
    lines.extend([
        String::new(),
        "RESPONSE · exporter-reported".into(),
        format!("Status: {}", status(request)),
        format!(
            "HTTP version: {}",
            reported_label(&request.response_version)
        ),
        format!("Header size: {}", size(request.response_header_size)),
        format!("Body size: {}", size(request.response_body_size)),
        format!("Content size: {} bytes (reported)", request.content_size),
        format!("MIME type: {}", reported_label(&request.mime_type)),
    ]);
    pairs(&mut lines, "Response headers", &request.response_headers);
    cookies(&mut lines, "Response cookies", &request.response_cookies);
    if let Some(evidence) = netscope_core::websocket_evidence(request) {
        lines.extend([
            String::new(),
            "WEBSOCKET UPGRADE · recorded metadata".into(),
            evidence.summary.into(),
        ]);
        lines.extend(evidence.notes.into_iter().map(str::to_owned));
        lines.push("Header shape only · key/accept digest not verified".into());
        lines.push("Extensions and browser acceptance not verified".into());
        lines.push("HAR has no standard WebSocket frame history".into());
        lines.push("Body size is not a frame count".into());
        lines.push(String::new());
    }
    lines.extend(crate::sse_view::lines(request));
    lines.extend(response_body_lines(request));
    lines.extend([
        String::new(),
        "TIMINGS · exporter-reported".into(),
        format!("Blocked: {}", timing(request.timings.blocked)),
        format!("DNS: {}", timing(request.timings.dns)),
        format!("Connect: {}", timing(request.timings.connect)),
        format!("  SSL (within connect): {}", timing(request.timings.ssl)),
        format!("Send: {} ms", request.timings.send),
        format!("Waiting for response: {} ms", request.timings.wait),
        format!("Receive: {} ms", request.timings.receive),
    ]);
    if let Some(ip) = request.server_ip {
        lines.push(format!("Server IP (exporter-reported): {ip}"));
    }
    if let Some(connection) = &request.connection {
        lines.push(format!(
            "Connection (exporter-reported): {:?}",
            terminal_text(connection)
        ));
    }
    lines.push("WARNINGS".into());
    if request.warnings.is_empty() {
        lines.push("No import consistency warnings".into());
    }
    for warning in &request.warnings {
        lines.push(terminal_text(warning));
    }
    lines.extend(protocol_lines(request));
    lines
}
pub(super) fn request_body_lines(request: &Request) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("Request body".into());
    if matches!(request.request_body, RequestBody::NotRecorded) {
        lines.push("Request body not recorded".into());
    }
    if let RequestBody::Text { mime_type, text }
    | RequestBody::TextAndParams {
        mime_type, text, ..
    } = &request.request_body
    {
        lines.push(format!("MIME type: {}", terminal_text(mime_type)));
        if text.is_empty() {
            lines.push("Captured body is empty".into());
        }
        let mut end = text.len().min(16 * 1024);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        lines.push(terminal_text(&text[..end]));
        if end < text.len() {
            lines.push("Preview truncated at 16 KiB".into());
        }
    }
    if let RequestBody::Params { mime_type, params }
    | RequestBody::TextAndParams {
        mime_type, params, ..
    } = &request.request_body
    {
        lines.push(format!(
            "Captured parameters · MIME type: {}",
            terminal_text(mime_type)
        ));
        lines.push(
            if matches!(request.request_body, RequestBody::TextAndParams { .. }) {
                "Exporter text is retained separately; fields do not establish exact wire bytes"
            } else {
                "Exact multipart bytes not recorded"
            }
            .into(),
        );
        for param in params {
            lines.push(format!(
                "  {}: {}",
                terminal_text(&param.name),
                param
                    .value
                    .as_deref()
                    .map(terminal_text)
                    .unwrap_or_else(|| "Value not recorded".into())
            ));
            for (name, value) in [
                ("filename", &param.file_name),
                ("content type", &param.content_type),
            ] {
                if let Some(value) = value {
                    lines.push(format!("    {name}: {}", terminal_text(value)));
                }
            }
        }
    }
    lines
}

pub(super) fn response_body_label(request: &Request) -> String {
    match &request.body {
        Body::NotRecorded => "Response body not recorded".into(),
        Body::Text(s) if s.is_empty() => "Captured body is empty".into(),
        Body::Text(_) => "Exported text".into(),
        Body::DecodedText(s) if s.is_empty() => {
            "Decoded from base64 · Captured body is empty".into()
        }
        Body::DecodedText(_) => "Decoded from base64".into(),
        Body::Binary(_) => "Binary · decoded from base64 · hexadecimal preview".into(),
        Body::Undecoded { reason, .. } => {
            format!("Body could not be decoded: {}", terminal_text(reason))
        }
    }
}

pub(super) fn response_body_lines(request: &Request) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("Response body".into());
    lines.push(response_body_label(request));
    let preview = request.body.preview();
    if !matches!(request.body, Body::NotRecorded) {
        lines.push(preview.text);
    }
    if preview.truncated {
        lines.push("Preview truncated (16 KiB text / 256 binary bytes)".into());
    }
    lines
}

pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let columns = grapheme.width();
        if used + columns > width && !line.is_empty() {
            lines.push(line);
            line = String::new();
            used = 0;
        }
        line.push_str(grapheme);
        used += columns;
    }
    lines.push(line);
    lines
}
pub(super) fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut remaining = text;
    let mut lines = Vec::new();
    while remaining.width() > width {
        let mut end = 0;
        let mut columns = 0;
        for grapheme in remaining.graphemes(true) {
            if columns + grapheme.width() > width && end > 0 {
                break;
            }
            columns += grapheme.width();
            end += grapheme.len();
        }
        if let Some(space) = remaining[..end].rfind(' ')
            && !remaining[..space].trim().is_empty()
        {
            end = space + 1;
        }
        lines.push(remaining[..end].into());
        remaining = &remaining[end..];
    }
    lines.push(remaining.into());
    lines
}
pub(super) fn wrapped_inspector(
    session: &Session,
    request: &Request,
    id: RequestId,
    width: u16,
) -> Vec<String> {
    let mut lines = inspector_lines(request, id);
    lines.extend(redirect_lines(session, id));
    lines.extend(retry_lines(session, id));
    lines
        .iter()
        .flat_map(|line| wrap(line, usize::from(width).max(2)))
        .collect()
}
pub(super) fn clip(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    let mut result = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let columns = grapheme.width();
        if used + columns > width.saturating_sub(1) {
            break;
        }
        result.push_str(grapheme);
        used += columns;
    }
    result.push('…');
    result
}
fn offset(request: &Request, first: &Request) -> f64 {
    (request.instant.0 as f64 - first.instant.0 as f64) * 1000.0
        + (f64::from(request.instant.1) - f64::from(first.instant.1)) / 1_000_000.0
}
fn bar(request: &Request, first: &Request, span: f64) -> String {
    let start = (offset(request, first) / span * 15.0).clamp(0.0, 15.0) as usize;
    if request.total_ms == 0.0 {
        return format!("{}| 0 ms", " ".repeat(start));
    }
    let length = (request.total_ms / span * 16.0)
        .ceil()
        .clamp(1.0, (16 - start) as f64) as usize;
    format!("{}{}", " ".repeat(start), "━".repeat(length))
}
pub(super) fn line(frame: &mut Frame, area: Rect, text: String, style: Style) {
    frame.render_widget(Paragraph::new(text).style(style), area);
}
/// Draw one complete frame from borrowed capture data and deterministic state.
pub fn draw(frame: &mut Frame, session: &Session, app: &mut App, filename: &str) {
    let area = frame.area();
    app.resize(area.width, area.height, session);
    let base = crate::presentation::base();
    frame.render_widget(Block::default().style(base), area);
    if !app.usable() {
        frame.render_widget(
            Paragraph::new("Terminal too small: use at least 80 columns × 24 rows\nCtrl+c quits; q quits outside text editing")
                .wrap(ratatui::widgets::Wrap { trim: false }),
            area,
        );
        return;
    }
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);
    line(
        frame,
        header,
        format!(
            "NETSCOPE / {}\n{} · {} requests\n{}",
            match app.screen {
                Screen::Overview => "CAPTURE",
                Screen::Inspector(_) if app.journey => "JOURNEY",
                Screen::Inspector(_) => "INSPECTOR",
                Screen::Replay(_) => app
                    .replay
                    .as_ref()
                    .map_or("REPLAY", |editor| editor.title()),
                Screen::Hosts => "HOSTS",
            },
            clip(
                &terminal_text(filename),
                usize::from(area.width).saturating_sub(20)
            ),
            session.requests().len(),
            match app.screen {
                Screen::Overview if app.search.is_some() => {
                    app.search
                        .as_ref()
                        .map_or_else(String::new, |(text, cursor)| {
                            let before = terminal_text(&text[..cursor.byte]);
                            let after = terminal_text(&text[cursor.byte..]);
                            let room = usize::from(area.width).saturating_sub(14);
                            let mut tail = before.as_str();
                            while tail.width() > room {
                                tail = &tail[tail.graphemes(true).next().map_or(0, str::len)..];
                            }
                            format!(
                                "URL search: {tail}│{}",
                                clip(&after, room.saturating_sub(tail.width()))
                            )
                        })
                }
                Screen::Overview if !app.query.is_empty() => format!(
                    "{} / {} requests · {} · URL contains: {}",
                    app.request_order(session).len(),
                    session.requests().len(),
                    app.host_filter
                        .and_then(|id| session.host(id))
                        .map_or("All hosts".into(), |h| terminal_text(&h.name)),
                    terminal_text(&app.query)
                ),
                Screen::Overview => match app.host_filter.and_then(|id| session.host(id)) {
                    Some(host) => format!(
                        "Host: {} · {} / {} requests",
                        clip(
                            &terminal_text(&host.name),
                            usize::from(area.width).saturating_sub(40)
                        ),
                        app.request_order(session).len(),
                        session.requests().len()
                    ),
                    None =>
                        "Reported total (ms) · ↑↓ choose · Enter inspect · g explains timing".into(),
                },
                Screen::Inspector(_) if app.journey && app.notice.is_empty() =>
                    "Reported values · select a stage for its explanation".into(),
                Screen::Inspector(_) =>
                    if app.notice.is_empty() {
                        "Captured data · 1–6 sections · g journey · Tab changes focus".into()
                    } else {
                        app.notice.clone()
                    },
                Screen::Replay(_) =>
                    if app
                        .replay
                        .as_ref()
                        .is_some_and(|e| !e.draft().has_capture())
                    {
                        "Saved request · no capture".into()
                    } else {
                        "Capture unchanged".into()
                    },
                Screen::Hosts =>
                    "Which server? Choose a host, then Enter to see only its requests.".into(),
            }
        ),
        Style::default().fg(Color::Rgb(128, 225, 178)),
    );
    line(
        frame,
        footer,
        match app.screen {
            Screen::Overview if app.search.is_some() => {
                "Enter apply · Esc cancel · Ctrl+u clear · ←/→ edit · 1024 bytes max · Ctrl+c quit"
            }
            Screen::Hosts => "↑/↓ j/k move · Enter filter · Esc back · a all · ? help · q quit",
            Screen::Overview => {
                "↑/↓ move · Enter inspect · Tab focus · / search · h hosts · a all · ? · q quit"
            }
            Screen::Inspector(_) if app.journey => {
                "↑/↓ stage · PgUp/PgDn explanation · Home/End stages · g/Esc back · ? · q quit"
            }
            Screen::Inspector(_) => {
                "↑/↓ scroll · 1–6 tabs · g journey · Tab list · r replay · Esc back · ? · q quit"
            }
            Screen::Replay(_) => app
                .replay
                .as_ref()
                .map_or("Ctrl+c quit", |editor| editor.footer()),
        }
        .into(),
        Style::default().fg(Color::Gray),
    );
    if app.help {
        frame.render_widget(Paragraph::new("START WITH A QUESTION\n\nWhat happened?  ↑↓ chooses a request; Enter opens its details.\nRead Summary first, then 2 Headers, 3 Body or 4 Timing.\ng opens a journey with explanations of each timing stage.\n\nFIND YOUR WAY\nThe bright border receives keys. The filled row is selected.\nTab switches list / details. Esc returns without losing your place.\n↑↓ or j/k moves; PgUp/PgDn pages; Home/End jumps to the ends.\nh groups by host; / searches URL text; a clears both filters.\n5 Evidence shows limitations and candidates; 6 All keeps every field.\n\nTRY A NEW REQUEST\nr from details opens a separate draft. Edit, review, then Send.\nOnly Send starts traffic. A replay never replaces the capture.\n\n? or Esc closes help · q or Ctrl+c quits and restores the terminal").block(Block::default().borders(Borders::ALL).title(" Help ")),body);
        return;
    }
    match app.screen {
        Screen::Overview if area.width >= 100 => {
            crate::workspace_view::draw(frame, body, session, app)
        }
        Screen::Replay(_) => {
            if let Some(editor) = app.replay.as_mut() {
                editor.draw(frame, body, session);
            }
        }
        Screen::Hosts => crate::host_view::draw_hosts(frame, body, session, app),
        Screen::Overview => {
            let block =
                crate::workspace_view::block(" Requests · ↑↓ choose · Enter inspect ".into(), true);
            let inner = block.inner(body);
            frame.render_widget(block, body);
            if app.request_order(session).is_empty() {
                line(
                    frame,
                    inner,
                    if session.requests().is_empty() {
                        "No requests recorded"
                    } else {
                        "No requests match these filters · / edit search · a all requests"
                    }
                    .into(),
                    Style::default(),
                );
                return;
            }
            let [labels, rows, position] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(inner);
            let target_width = usize::from(rows.width).saturating_sub(56).max(8);
            line(
                frame,
                labels,
                format!(
                    "  {:<4} {:<8} {:<target_width$} {:<6} {:>11} {}",
                    "ID", "METHOD", "HOST / PATH", "Status", "Total (ms)", "Timeline"
                ),
                Style::default().fg(Color::Gray),
            );
            let first = session
                .chronological()
                .first()
                .and_then(|id| session.request(*id));
            let span = first
                .map(|first| {
                    session
                        .requests()
                        .iter()
                        .map(|r| offset(r, first) + r.total_ms)
                        .fold(1.0, f64::max)
                })
                .unwrap_or(1.0);
            for (row, id) in app
                .request_order(session)
                .iter()
                .enumerate()
                .skip(app.overview_offset)
                .take(usize::from(rows.height))
            {
                if let Some(request) = session.request(*id) {
                    let selected = app.selected == Some(row);
                    let method = clip(&terminal_text(&request.method), 8);
                    let rest = request
                        .url
                        .split_once("://")
                        .map(|(_, rest)| rest)
                        .unwrap_or("");
                    let path = rest
                        .find(['/', '?', '#'])
                        .map(|index| &rest[index..])
                        .unwrap_or("/");
                    let target = terminal_text(&format!("{}{path}", request.host));
                    let status = if request.status == 0 {
                        "none".into()
                    } else {
                        request.status.to_string()
                    };
                    let target = clip(&target, target_width);
                    let total = clip(&format!("{:.2}", request.total_ms), 8);
                    let text = format!(
                        "{} {:<4} {:<8} {}{} {:<6} {:>8} ms {}",
                        if selected { ">" } else { " " },
                        id.0,
                        method,
                        target,
                        " ".repeat(target_width.saturating_sub(target.width())),
                        status,
                        total,
                        first
                            .map(|first| bar(request, first, span))
                            .unwrap_or_default()
                    );
                    let style = if selected {
                        Style::default()
                            .fg(crate::presentation::MINT)
                            .bg(crate::presentation::SELECTED)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    line(
                        frame,
                        Rect::new(
                            rows.x,
                            rows.y + (row - app.overview_offset) as u16,
                            rows.width,
                            1,
                        ),
                        text,
                        style,
                    );
                }
            }
            let selected = app.selected.unwrap_or(0);
            let note = if app
                .request_order(session)
                .get(selected)
                .and_then(|id| session.request(*id))
                .is_some_and(|r| r.status == 0)
            {
                " · No HTTP status recorded"
            } else {
                ""
            };
            line(
                frame,
                position,
                format!(
                    "Row {} / {}{}",
                    selected + 1,
                    app.request_order(session).len(),
                    note
                ),
                Style::default().fg(Color::Gray),
            );
        }
        Screen::Inspector(_) => crate::workspace_view::draw(frame, body, session, app),
    }
}

/// Explain recorded redirect evidence separately from possible follow-up requests.
pub fn redirect_lines(session: &Session, id: RequestId) -> Vec<String> {
    use netscope_core::RedirectEvidence;
    let Some(evidence) = session.redirect(id) else {
        return vec![];
    };
    let mut lines = vec!["REDIRECT EVIDENCE".into()];
    match evidence {
        RedirectEvidence::NotApplicable => {
            lines.push("Not analyzed: only statuses 301, 302, 303, 307, 308".into())
        }
        RedirectEvidence::Unavailable(reason) => lines.push((*reason).into()),
        RedirectEvidence::Candidates { target, ids, more } => {
            lines.push(format!(
                "Resolved Location (without fragment): {}",
                terminal_text(target)
            ));
            lines.push(
                "Rule: same URL, compatible method, later start, 0..30 s after reported end".into(),
            );
            lines.push(
                "Reported end = recorded start + total duration; early follow-ups may be missed"
                    .into(),
            );
            lines.push(match ids.len() {
                0 => "No candidate under this rule; this does not prove the redirect was not followed",
                1 => "One candidate; URL, method and timing do not prove causation",
                _ => "Ambiguous: multiple candidates; no request chosen",
            }.into());
            for candidate in ids {
                if let Some(request) = session.request(*candidate) {
                    lines.push(format!(
                        "Candidate #{}: {} {}",
                        candidate.0,
                        terminal_text(&request.method),
                        terminal_text(&request.started_at)
                    ));
                }
            }
            if *more {
                lines.push("More candidates exist; showing the first 20".into());
            }
            lines.push("Esc returns to the list; candidate numbers are source IDs".into());
            lines.push("From the list, a clears a host filter to show other hosts".into());
        }
    }
    lines
}

/// Explain repeated-request evidence without recommending a new request.
pub fn retry_lines(session: &Session, id: RequestId) -> Vec<String> {
    use netscope_core::RetryEvidence;
    let (Some(evidence), Some(source)) = (session.retry(id), session.request(id)) else {
        return vec![];
    };
    if matches!(evidence, RetryEvidence::NotApplicable) {
        return vec![];
    }
    let mut lines = vec![
        "RETRY EVIDENCE".into(),
        format!(
            "Recorded source: {} · status {}",
            terminal_text(&source.method),
            source.status
        ),
    ];
    match evidence {
        RetryEvidence::NotApplicable => {}
        RetryEvidence::Unavailable(reason) => lines.push((*reason).into()),
        RetryEvidence::Candidates { ids, more } => {
            lines.push(
                "Rule: same URL/method and exact captured headers, query fields, cookies".into(),
            );
            lines.push("Both report body size 0 with no captured request body".into());
            lines.push("Later start, 0..30 s after reported end (start + total duration)".into());
            lines.push("Retry-After is not interpreted; longer delays may be missed".into());
            lines.push(
                match ids.len() {
                    0 => "No candidate under this rule; this does not prove there was no retry",
                    1 => "One possible retry; repetition does not prove intent or success",
                    _ => "Ambiguous: multiple possible retries; no request chosen",
                }
                .into(),
            );
            lines.push("Polling, a refresh, or another tab can produce the same evidence".into());
            for candidate in ids {
                if let Some(request) = session.request(*candidate) {
                    lines.push(format!(
                        "Candidate #{}: {} {}",
                        candidate.0,
                        terminal_text(&request.method),
                        terminal_text(&request.started_at)
                    ));
                }
            }
            if *more {
                lines.push("More candidates exist; showing the first 20".into());
            }
            lines.push("Esc returns to the list; candidate numbers are source IDs".into());
        }
    }
    lines
}

/// Keep exporter labels separate from general protocol explanations.
pub fn protocol_lines(request: &Request) -> Vec<String> {
    use netscope_core::RecordedProtocol;
    let sent = RecordedProtocol::from_label(&request.request_version);
    let received = RecordedProtocol::from_label(&request.response_version);
    let mut lines = vec![
        "PROTOCOL NOTES · exporter labels".into(),
        format!(
            "Request label: {:?} -> {}",
            reported_label(&request.request_version),
            sent.name()
        ),
        format!(
            "Response label: {:?} -> {}",
            reported_label(&request.response_version),
            received.name()
        ),
    ];
    lines.push(
        if sent == RecordedProtocol::Unrecognized || received == RecordedProtocol::Unrecognized {
            "Comparison unavailable: at least one label is unrecognized"
        } else if sent != received {
            "Labels disagree; no upgrade, downgrade, or cause is inferred"
        } else {
            "Labels agree on a recognized version; this is exporter evidence"
        }
        .into(),
    );
    lines.push("General protocol behaviour, not observed messages:".into());
    lines.push(sent.explanation().into());
    if received != sent {
        lines.push(received.explanation().into());
    }
    lines.push("These HAR fields provide no packet/frame trace or stream IDs".into());
    lines.push("No compression savings or connection reuse are established".into());
    lines
}

fn reported_label(value: &str) -> String {
    if value.is_empty() {
        "Not recorded (empty label)".into()
    } else {
        terminal_text(value)
    }
}

#[cfg(test)]
mod operation_id_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn final_id_can_be_used_once_then_send_stays_in_review()
    -> Result<(), Box<dyn std::error::Error>> {
        let session = netscope_import::import_bytes(
            include_bytes!("../../../tests/fixtures/har/single-request.har"),
            netscope_import::Limits::default(),
        )?;
        let mut app = App::new(&session);
        app.next_send_id = Some(u64::MAX);
        app.apply(Action::Open, &session);
        app.apply(Action::Replay, &session);
        let press =
            |app: &mut App, code| app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &session);
        for _ in 0..4 {
            press(&mut app, KeyCode::Down);
        }
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('s'));
        let id = app
            .replay
            .as_mut()
            .ok_or("editor")?
            .take_send()
            .ok_or("last send")?
            .0;
        assert_eq!(id, u64::MAX);
        assert_eq!(app.next_send_id, None);
        app.replay.as_mut().ok_or("editor")?.start_failed(id);
        for _ in 0..2 {
            press(&mut app, KeyCode::Char('s'));
        }
        let editor = app.replay.as_mut().ok_or("editor")?;
        assert!(editor.take_send().is_none());
        assert_eq!(editor.title(), "REVIEW");
        assert_eq!(editor.draft().url(), session.requests()[0].url);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30))?;
        terminal.draw(|frame| draw(frame, &session, &mut app, "synthetic.har"))?;
        let buffer = terminal.backend().buffer();
        let text: String = (0..30)
            .flat_map(|y| (0..100).map(move |x| buffer[(x, y)].symbol()))
            .collect();
        assert!(text.contains("Send IDs exhausted"));
        assert_eq!(app.next_send_id, None);
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        app.apply(Action::Replay, &session);
        for _ in 0..4 {
            press(&mut app, KeyCode::Down);
        }
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('s'));
        let editor = app.replay.as_mut().ok_or("editor")?;
        assert!(editor.take_send().is_none());
        assert_eq!(editor.title(), "REVIEW");
        Ok(())
    }
}
