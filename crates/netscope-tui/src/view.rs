use netscope_core::{
    Body, Cookie, HostId, NameValue, ReportedSize, Request, RequestBody, RequestId, Session,
    Timing, terminal_text,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A keyboard action, independent of terminal event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move up one request or inspector line.
    Up,
    /// Move down one request or inspector line.
    Down,
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
    /// Open a separate GET replay draft.
    Replay,
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
    /// Current view.
    pub screen: Screen,
    /// Selected chronological row; none for an empty capture.
    pub selected: Option<usize>,
    /// First visible overview row.
    pub overview_offset: usize,
    /// First visible wrapped inspector line.
    pub inspector_offset: usize,
    /// Whether help is visible.
    pub help: bool,
    /// Whether quitting was requested.
    pub quit: bool,
    /// The active URL-host filter, or all requests when absent.
    pub host_filter: Option<HostId>,
    /// Selected row in the alphabetical host list.
    pub host_selected: Option<usize>,
    /// First visible host row.
    pub host_offset: usize,
    /// Local editor, present only while a replay draft is open.
    pub replay: Option<crate::replay_view::ReplayEditor>,
    notice: String,
    all_requests_position: Option<(Option<usize>, usize)>,
    host_return: Screen,
    width: u16,
    height: u16,
}
impl App {
    /// Initialise selections from the session rather than inventing empty rows.
    pub fn new(session: &Session) -> Self {
        Self {
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
            all_requests_position: None,
            host_return: Screen::Overview,
            width: 100,
            height: 30,
        }
    }
    /// Borrow the chronological source IDs visible through the current filter.
    pub fn request_order<'a>(&self, session: &'a Session) -> &'a [RequestId] {
        match self.host_filter {
            None => session.chronological(),
            Some(id) => session
                .host_summary(id)
                .map(|summary| summary.request_ids.as_slice())
                .unwrap_or(&[]),
        }
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
            let len = wrapped_inspector(session, request, id, self.width.saturating_sub(2)).len();
            self.inspector_offset = self
                .inspector_offset
                .min(len.saturating_sub(usize::from(self.height.saturating_sub(6))));
        }
    }
    fn show_hosts(&mut self, session: &Session) {
        self.host_return = self.screen;
        if let Some(host) = self.host_filter {
            self.host_selected = session.host_order().iter().position(|id| *id == host);
        }
        self.screen = Screen::Hosts;
    }
    fn show_all(&mut self) {
        self.host_filter = None;
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
        if let Screen::Replay(id) = self.screen {
            if let Some(editor) = self.replay.as_mut() {
                if !editor.editing() && key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
                    self.quit = true;
                    return;
                }
                if self.width < 80 || self.height < 24 {
                    return;
                }
                if editor.key(key) {
                    self.replay = None;
                    self.screen = Screen::Inspector(id);
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
        match (self.screen, action) {
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
            (Screen::Overview | Screen::Hosts, Action::AllRequests) => self.show_all(),
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
                        if self.host_filter.is_none() {
                            self.all_requests_position =
                                Some((self.selected, self.overview_offset));
                        }
                        self.host_filter = Some(id);
                        self.selected = Some(0);
                        self.overview_offset = 0;
                    }
                    self.screen = Screen::Overview;
                }
            }
            (Screen::Overview, Action::Back) if self.host_filter.is_some() => {
                self.show_hosts(session)
            }
            (Screen::Overview, Action::Down) => {
                self.selected = self
                    .selected
                    .map(|i| (i + 1).min(self.request_order(session).len().saturating_sub(1)))
            }
            (Screen::Overview, Action::Up) => {
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

fn status(request: &Request) -> String {
    if request.status == 0 {
        "No HTTP status recorded".into()
    } else {
        format!("{} {}", request.status, terminal_text(&request.status_text))
    }
}
fn timing(value: Timing) -> String {
    match value {
        Timing::Missing => "Not recorded".into(),
        Timing::Unavailable => "Unavailable (-1)".into(),
        Timing::Milliseconds(n) => format!("{n} ms"),
    }
}
fn size(value: ReportedSize) -> String {
    match value {
        ReportedSize::Missing => "Not recorded".into(),
        ReportedSize::Unavailable => "Unavailable (-1)".into(),
        ReportedSize::Bytes(n) => format!("{n} bytes"),
    }
}
fn pairs(lines: &mut Vec<String>, title: &str, values: &[NameValue]) {
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
fn cookies(lines: &mut Vec<String>, title: &str, values: &[Cookie]) {
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
        format!("HTTP version: {}", terminal_text(&request.request_version)),
        format!("Header size: {}", size(request.request_header_size)),
        format!("Body size: {}", size(request.request_body_size)),
    ];
    pairs(&mut lines, "Request headers", &request.headers);
    pairs(&mut lines, "Query fields", &request.query);
    cookies(&mut lines, "Request cookies", &request.cookies);
    lines.push("Request body".into());
    match &request.request_body {
        RequestBody::NotRecorded => lines.push("Request body not recorded".into()),
        RequestBody::Text { mime_type, text } => {
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
        RequestBody::Params { mime_type, params } => {
            lines.push(format!(
                "Captured parameters · MIME type: {}",
                terminal_text(mime_type)
            ));
            lines.push("Exact multipart bytes not recorded".into());
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
    }
    lines.extend([
        String::new(),
        "RESPONSE · exporter-reported".into(),
        format!("Status: {}", status(request)),
        format!("HTTP version: {}", terminal_text(&request.response_version)),
        format!("Header size: {}", size(request.response_header_size)),
        format!("Body size: {}", size(request.response_body_size)),
        format!("Content size: {} bytes (reported)", request.content_size),
        format!("MIME type: {}", terminal_text(&request.mime_type)),
    ]);
    pairs(&mut lines, "Response headers", &request.response_headers);
    cookies(&mut lines, "Response cookies", &request.response_cookies);
    lines.push("Response body".into());
    lines.push(match &request.body {
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
    });
    let preview = request.body.preview();
    if !matches!(request.body, Body::NotRecorded) {
        lines.push(preview.text);
    }
    if preview.truncated {
        lines.push("Preview truncated (16 KiB text / 256 binary bytes)".into());
    }
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
fn wrapped_inspector(
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
    let base = Style::default()
        .fg(Color::Rgb(226, 234, 230))
        .bg(Color::Rgb(12, 17, 17));
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
                Screen::Inspector(_) => "INSPECTOR",
                Screen::Replay(_) => "REPLAY DRAFT",
                Screen::Hosts => "HOSTS",
            },
            clip(
                &terminal_text(filename),
                usize::from(area.width).saturating_sub(20)
            ),
            session.requests().len(),
            match app.screen {
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
                    None => "Chronological order · Reported total (ms) and recorded start".into(),
                },
                Screen::Inspector(_) =>
                    if app.notice.is_empty() {
                        "Captured data · Up/Down scroll".into()
                    } else {
                        app.notice.clone()
                    },
                Screen::Replay(_) => "Local draft only · no network traffic".into(),
                Screen::Hosts => "Grouped by URL host · derived counts and duration sums".into(),
            }
        ),
        Style::default().fg(Color::Rgb(128, 225, 178)),
    );
    line(
        frame,
        footer,
        match app.screen {
            Screen::Hosts => "↑/↓ j/k move · Enter filter · Esc back · a all · ? help · q quit",
            Screen::Overview => {
                "↑/↓ j/k move · Enter inspect · h hosts · a all · Esc back · ? help · q quit"
            }
            Screen::Inspector(_) => {
                "↑/↓ j/k scroll · r replay · h hosts · Esc back · ? help · q quit"
            }
            Screen::Replay(_) => "↑/↓ move · Enter choose/save · Esc back · Ctrl+c quit",
        }
        .into(),
        Style::default().fg(Color::Gray),
    );
    if app.help {
        frame.render_widget(Paragraph::new("KEYS\n\nUp / k      previous row or inspector line\nDown / j    next row or inspector line\nEnter       inspect request / filter by selected host\nh           open Hosts / return from Hosts\na           all requests (from lists)\nEsc         return to previous view / close help\nr           replay draft (from inspector)\n?           toggle help\nq           quit and restore terminal\n\nBars show reported starts and totals, not inferred connections.\nSSL is contained within connect.\nMissing capture data remains missing.").block(Block::default().borders(Borders::ALL).title(" Help ")),body);
        return;
    }
    match app.screen {
        Screen::Replay(_) => {
            if let Some(editor) = app.replay.as_mut() {
                editor.draw(frame, body);
            }
        }
        Screen::Hosts => crate::host_view::draw_hosts(frame, body, session, app),
        Screen::Overview => {
            let block = Block::default().borders(Borders::ALL).title(" Requests ");
            let inner = block.inner(body);
            frame.render_widget(block, body);
            if app.request_order(session).is_empty() {
                line(
                    frame,
                    inner,
                    "No requests recorded".into(),
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
                            .fg(Color::Black)
                            .bg(Color::Rgb(128, 225, 178))
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
        Screen::Inspector(id) => {
            if let Some(request) = session.request(id) {
                let block = Block::default().borders(Borders::ALL).title(format!(
                    " Request #{} · line {} ",
                    id.0,
                    app.inspector_offset + 1
                ));
                let inner = block.inner(body);
                frame.render_widget(block, body);
                let content = wrapped_inspector(session, request, id, inner.width)
                    .into_iter()
                    .skip(app.inspector_offset)
                    .take(usize::from(inner.height))
                    .map(Line::from)
                    .collect::<Vec<_>>();
                frame.render_widget(Paragraph::new(content), inner);
            }
        }
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
            terminal_text(&request.request_version),
            sent.name()
        ),
        format!(
            "Response label: {:?} -> {}",
            terminal_text(&request.response_version),
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
