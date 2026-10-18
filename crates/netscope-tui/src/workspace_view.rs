//! Request selection and interpretation share a stable workspace.
use crate::{App, Screen, view};
use netscope_core::{Request, RequestId, Session, Timing, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub(super) const MINT: Color = Color::Rgb(128, 225, 178);
pub(super) const MUTED: Color = Color::Rgb(159, 181, 176);
const RULE: Color = Color::Rgb(60, 83, 77);
pub(super) const SELECTED: Color = Color::Rgb(32, 65, 54);

/// A section changes presentation, never the captured request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    /// Status, representation, and capture boundaries.
    Summary,
    /// Captured headers, query fields, and cookies.
    Headers,
    /// Request and response representations, including retained SSE.
    Body,
    /// Exporter-reported timing stages.
    Timing,
    /// Warnings, protocol labels, upgrades, and candidate relationships.
    Evidence,
    /// The complete original report, with all fields available.
    All,
}
impl InspectorTab {
    fn title(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Headers => "Headers",
            Self::Body => "Body",
            Self::Timing => "Timing",
            Self::Evidence => "Evidence",
            Self::All => "All",
        }
    }
}

pub(super) fn block(title: String, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(if focused { MINT } else { RULE }))
        .title_style(Style::default().fg(if focused { MINT } else { MUTED }))
}

// These dimensions also bound scrolling, so resizing cannot strand a line.
pub(super) fn detail_size(width: u16, height: u16) -> (u16, u16) {
    let pane = if width >= 100 {
        let [_, detail] = Layout::horizontal([Constraint::Percentage(40), Constraint::Min(1)])
            .areas(Rect::new(0, 0, width, height));
        detail.width
    } else {
        width
    };
    (pane.saturating_sub(2), height.saturating_sub(10))
}

fn timing_lines(r: &Request) -> Vec<String> {
    let mut lines = vec![
        "TIMINGS · exporter-reported".into(),
        format!("Reported total: {} ms", r.total_ms),
        format!("Blocked: {}", view::timing(r.timings.blocked)),
        format!("DNS: {}", view::timing(r.timings.dns)),
        format!("Connect: {}", view::timing(r.timings.connect)),
        format!("  SSL (within connect): {}", view::timing(r.timings.ssl)),
        format!("Send: {} ms", r.timings.send),
        format!("Waiting for response: {} ms", r.timings.wait),
        format!("Receive: {} ms", r.timings.receive),
        String::new(),
        "SSL is part of connect; do not add it twice.".into(),
        "Wait includes network travel, not just server work.".into(),
        "Missing or unavailable values are not zero.".into(),
    ];
    let stages = [
        ("DNS", r.timings.dns),
        ("Connect", r.timings.connect),
        ("Send", Timing::Milliseconds(r.timings.send)),
        ("Wait", Timing::Milliseconds(r.timings.wait)),
        ("Receive", Timing::Milliseconds(r.timings.receive)),
    ];
    let max = stages
        .iter()
        .filter_map(|(_, value)| match value {
            Timing::Milliseconds(n) => Some(*n),
            _ => None,
        })
        .fold(0.0_f64, f64::max);
    lines.extend([
        String::new(),
        "Relative reported durations · not packet order".into(),
    ]);
    for (name, value) in stages {
        if let Timing::Milliseconds(n) = value {
            let count = if max > 0.0 {
                (n / max * 16.0).ceil() as usize
            } else {
                0
            };
            lines.push(format!(
                "{name:<8} {:<16} {n} ms",
                "━".repeat(count.min(16))
            ));
        }
    }
    lines
}
fn upgrade_lines(r: &Request) -> Vec<String> {
    let Some(e) = netscope_core::websocket_evidence(r) else {
        return vec![];
    };
    let mut lines = vec![
        "WEBSOCKET UPGRADE · recorded metadata".into(),
        e.summary.into(),
    ];
    lines.extend(e.notes.into_iter().map(str::to_owned));
    lines.extend([
        "Header shape only · key/accept digest not verified".into(),
        "Extensions and browser acceptance not verified".into(),
        "HAR has no standard WebSocket frame history".into(),
        "Body size is not a frame count".into(),
    ]);
    lines
}
fn sections(session: &Session, r: &Request, id: RequestId, tab: InspectorTab) -> Vec<String> {
    match tab {
        InspectorTab::Summary => {
            let mut lines = vec![
                format!("Status: {}", view::status(r)),
                match r.status {
                    0 => "No response status was recorded.".into(),
                    100..=199 => "An interim response or protocol switch was recorded.".into(),
                    200..=299 => "The server reported success for this request.".into(),
                    300..=399 => "The response concerns redirection or cached content.".into(),
                    400..=499 => "The server reported a problem with the request.".into(),
                    500..=599 => "The server reported that it could not fulfil the request.".into(),
                    _ => "This status is outside the standard HTTP classes.".into(),
                },
                String::new(),
                "RECORDED MEASUREMENTS".into(),
                format!("Reported total: {} ms", r.total_ms),
                format!("Reported content: {} bytes", r.content_size),
                format!(
                    "Content type: {}",
                    if r.mime_type.is_empty() {
                        "Not recorded".into()
                    } else {
                        terminal_text(&r.mime_type)
                    }
                ),
                "RESPONSE CONTENT".into(),
            ];
            // The label states the representation without repeating a large body.
            lines.push(view::response_body_label(r));
            lines.extend([
                String::new(),
                "WHAT IS MISSING?".into(),
                "Body text may differ from transferred bytes.".into(),
                "g explains timing; 6 All shows every recorded field.".into(),
                format!("Import warnings: {} · see Evidence", r.warnings.len()),
            ]);
            if netscope_core::websocket_evidence(r).is_some() {
                lines.push("WebSocket upgrade metadata: see Evidence.".into());
            }
            if netscope_core::inspect_sse(r).is_some() {
                lines.push("SSE inspection: see Body.".into());
            }
            lines
        }
        InspectorTab::Headers => {
            let mut lines = vec!["REQUEST · exporter-reported".into()];
            view::pairs(&mut lines, "Request headers", &r.headers);
            view::pairs(&mut lines, "Query fields", &r.query);
            view::cookies(&mut lines, "Request cookies", &r.cookies);
            lines.extend([String::new(), "RESPONSE · exporter-reported".into()]);
            view::pairs(&mut lines, "Response headers", &r.response_headers);
            view::cookies(&mut lines, "Response cookies", &r.response_cookies);
            lines
        }
        InspectorTab::Body => {
            let mut lines = view::request_body_lines(r);
            lines.push(String::new());
            lines.extend(crate::sse_view::lines(r));
            lines.extend(view::response_body_lines(r));
            lines
        }
        InspectorTab::Timing => timing_lines(r),
        InspectorTab::Evidence => {
            let mut lines = vec!["WARNINGS".into()];
            if r.warnings.is_empty() {
                lines.push("No import consistency warnings".into());
            }
            lines.extend(r.warnings.iter().map(|w| terminal_text(w)));
            lines.extend(upgrade_lines(r));
            lines.extend(view::protocol_lines(r));
            lines.extend(view::redirect_lines(session, id));
            lines.extend(view::retry_lines(session, id));
            lines
        }
        InspectorTab::All => view::inspector_lines(r, id),
    }
}

pub(super) fn detail_lines(
    session: &Session,
    r: &Request,
    id: RequestId,
    tab: InspectorTab,
    width: u16,
) -> Vec<String> {
    if tab == InspectorTab::All {
        return view::wrapped_inspector(session, r, id, width);
    }
    let mut lines = vec![
        match tab {
            InspectorTab::Summary => "WHAT HAPPENED?",
            InspectorTab::Headers => "Headers describe the message; repeated fields stay separate.",
            InspectorTab::Body => "Bodies carry content. A missing body is not an empty body.",
            InspectorTab::Timing => "Where was time reported? Press g to explore each stage.",
            InspectorTab::Evidence => "What can this capture support? Candidates are not proof.",
            InspectorTab::All => "Complete captured report",
        }
        .into(),
        String::new(),
    ];
    lines.extend(sections(session, r, id, tab));
    lines
        .iter()
        .flat_map(|s| view::wrap_words(s, usize::from(width).max(2)))
        .collect()
}

fn draw_list(frame: &mut Frame, area: Rect, session: &Session, app: &App) {
    let b = block(
        " Requests · Enter opens details ".into(),
        app.screen == Screen::Overview,
    );
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let [labels, rows, position] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    view::line(
        frame,
        labels,
        " ID   Method Status Path".into(),
        Style::default().fg(MUTED),
    );
    let order = app.request_order(session);
    for (index, id) in order
        .iter()
        .enumerate()
        .skip(app.overview_offset)
        .take(usize::from(rows.height))
    {
        let Some(r) = session.request(*id) else {
            continue;
        };
        let selected = app.selected == Some(index);
        let rest = r
            .url
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&r.url);
        let path = rest
            .find(['/', '?', '#'])
            .map(|i| &rest[i..])
            .unwrap_or("/");
        let prefix = format!("{} {:<4} ", if selected { ">" } else { " " }, id.0,);
        let method = format!("{:<6} ", view::clip(&terminal_text(&r.method), 6));
        let status = format!(
            "{:<4} ",
            if r.status == 0 {
                "none".into()
            } else {
                r.status.to_string()
            }
        );
        let room = usize::from(rows.width)
            .saturating_sub(prefix.width() + method.width() + status.width());
        let spans = vec![
            Span::styled(prefix, Style::default().fg(MUTED)),
            Span::styled(method, Style::default().fg(MINT)),
            Span::styled(
                status,
                Style::default().fg(if r.status >= 400 {
                    Color::Rgb(242, 186, 113)
                } else {
                    MINT
                }),
            ),
            Span::raw(view::clip(&terminal_text(path), room)),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(if selected {
                Style::default().bg(SELECTED).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }),
            Rect::new(
                rows.x,
                rows.y + (index - app.overview_offset) as u16,
                rows.width,
                1,
            ),
        );
    }
    view::line(
        frame,
        position,
        if order.is_empty() {
            if session.requests().is_empty() {
                "No requests recorded".into()
            } else {
                "No matches · / change search · a clear filters".into()
            }
        } else {
            format!(
                "{} / {} · ↑↓ choose · Enter inspect",
                app.selected.unwrap_or(0) + 1,
                order.len()
            )
        },
        Style::default().fg(MUTED),
    );
}

pub(super) fn draw(frame: &mut Frame, area: Rect, session: &Session, app: &App) {
    if app.journey && matches!(app.screen, Screen::Inspector(_)) {
        if let Screen::Inspector(id) = app.screen
            && let Some(request) = session.request(id)
        {
            crate::journey_view::draw(
                frame,
                area,
                request,
                id,
                app.journey_stage,
                app.inspector_offset,
            );
        }
        return;
    }
    let detail = if area.width >= 100 {
        let [list, detail] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Min(1)]).areas(area);
        draw_list(frame, list, session, app);
        detail
    } else {
        area
    };
    let id = if let Screen::Inspector(id) = app.screen {
        Some(id)
    } else {
        app.selected
            .and_then(|i| app.request_order(session).get(i))
            .copied()
    };
    let focused = matches!(app.screen, Screen::Inspector(_));
    let b = block(
        format!(
            " {} · {} ",
            if focused { "Inspector" } else { "Preview" },
            app.inspector_tab.title()
        ),
        focused,
    );
    let inner = b.inner(detail);
    frame.render_widget(b, detail);
    let Some((id, r)) = id.and_then(|id| session.request(id).map(|r| (id, r))) else {
        view::line(
            frame,
            inner,
            "Select a recorded request to inspect it.".into(),
            Style::default().fg(MUTED),
        );
        return;
    };
    let [heading, tabs, content] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .areas(inner);
    view::line(
        frame,
        heading,
        format!(
            "#{} {} · {}\nURL: {}",
            id.0,
            terminal_text(&r.method),
            view::status(r),
            terminal_text(&r.url)
        ),
        Style::default().fg(MINT),
    );
    {
        // Two rows preserve full words in the narrow split pane.
        let labels = ["Summary", "Headers", "Body", "Timing", "Evidence", "All"];
        let rows = labels
            .chunks(3)
            .enumerate()
            .map(|(row, labels)| {
                Line::from(
                    labels
                        .iter()
                        .enumerate()
                        .map(|(column, label)| {
                            let i = row * 3 + column;
                            Span::styled(
                                format!("{} {label}  ", i + 1),
                                if i == app.inspector_tab as usize {
                                    Style::default()
                                        .fg(MINT)
                                        .bg(SELECTED)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(MUTED)
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(rows), tabs);
    }
    let lines = detail_lines(session, r, id, app.inspector_tab, content.width);
    let total = lines.len();
    let offset = if focused { app.inspector_offset } else { 0 };
    let content_lines = lines
        .into_iter()
        .skip(offset)
        .take(usize::from(content.height))
        .map(crate::presentation::report_line)
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(content_lines), content);
    if total > usize::from(content.height) {
        let label = format!(
            " {}–{} / {} · {} ",
            offset + 1,
            (offset + usize::from(content.height)).min(total),
            total,
            if focused {
                "↑↓ scroll"
            } else {
                "Tab to scroll"
            }
        );
        view::line(
            frame,
            Rect::new(
                detail.x + 1,
                detail.bottom().saturating_sub(1),
                detail.width.saturating_sub(2),
                1,
            ),
            label,
            Style::default().fg(MUTED),
        );
    }
}
