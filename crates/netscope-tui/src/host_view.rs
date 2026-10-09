use crate::{
    App,
    view::{clip, line, wrap},
};
use netscope_core::{Session, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

fn duration(sum: Option<f64>) -> String {
    match sum {
        None => "Overflow".into(),
        Some(value) if value >= 1_000_000.0 => format!("{value:.2e}"),
        Some(value) => format!("{value:.2}"),
    }
}
pub(crate) fn draw_hosts(frame: &mut Frame, area: Rect, session: &Session, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Hosts · alphabetical ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if session.hosts().is_empty() {
        line(frame, inner, "No hosts recorded".into(), Style::default());
        return;
    }
    let [labels, rows, position, details] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(8),
    ])
    .areas(inner);
    let name_width = usize::from(rows.width).saturating_sub(43);
    line(
        frame,
        labels,
        format!(
            "  {:<name_width$} {:>6} {:>5} {:>5} {:>9} {:>10}",
            "HOST", "Count", "4xx", "5xx", "No status", "Sum (ms)"
        ),
        Style::default().fg(Color::Gray),
    );
    for (row, id) in session
        .host_order()
        .iter()
        .enumerate()
        .skip(app.host_offset)
        .take(usize::from(rows.height))
    {
        if let (Some(host), Some(summary)) = (session.host(*id), session.host_summary(*id)) {
            let name = clip(&terminal_text(&host.name), name_width);
            let selected = app.host_selected == Some(row);
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
                    rows.y + (row - app.host_offset) as u16,
                    rows.width,
                    1,
                ),
                format!(
                    "{} {}{} {:>6} {:>5} {:>5} {:>9} {:>10}",
                    if selected { ">" } else { " " },
                    name,
                    " ".repeat(
                        name_width
                            .saturating_sub(unicode_width::UnicodeWidthStr::width(name.as_str()))
                    ),
                    summary.request_ids.len(),
                    summary.statuses.client_error,
                    summary.statuses.server_error,
                    summary.statuses.not_recorded,
                    duration(summary.summed_duration_ms)
                ),
                style,
            );
        }
    }
    line(
        frame,
        position,
        format!(
            "Host {} / {}",
            app.host_selected.unwrap_or(0) + 1,
            session.hosts().len()
        ),
        Style::default().fg(Color::Gray),
    );
    if let Some(id) = app.host_selected.and_then(|i| session.host_order().get(i))
        && let (Some(host), Some(summary)) = (session.host(*id), session.host_summary(*id))
    {
        let name_lines = wrap(
            &format!("Host: {}", terminal_text(&host.name)),
            usize::from(details.width),
        );
        let truncated = name_lines.len() > 4;
        let mut content: Vec<String> = name_lines.into_iter().take(4).collect();
        if truncated && let Some(last) = content.last_mut() {
            *last = "Host name shortened here; inspect a request for its full URL".into();
        }
        let counts = &summary.statuses;
        content.extend([
            format!(
                "Statuses: 1xx {} · 2xx {} · 3xx {}",
                counts.informational, counts.successful, counts.redirection
            ),
            format!(
                "          4xx {} · 5xx {} · no status {}",
                counts.client_error, counts.server_error, counts.not_recorded
            ),
            format!(
                "Requests with import warnings: {}",
                summary.warning_requests
            ),
            "Reported durations are summed; requests can overlap.".into(),
        ]);
        frame.render_widget(
            Paragraph::new(content.into_iter().map(Line::from).collect::<Vec<_>>()),
            details,
        );
    }
}
