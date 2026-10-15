//! A stage selection changes the explanation, never the captured measurements.
use crate::{view, workspace_view};
use netscope_core::{Request, RequestId, Timing, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) const STAGES: usize = 7;
const NAMES: [&str; STAGES] = [
    "Blocked", "DNS", "Connect", "└ TLS", "Send", "Wait", "Receive",
];
const TITLES: [&str; STAGES] = [
    "Before sending",
    "Find an address",
    "Make a connection",
    "Set up encryption",
    "Send the request",
    "Wait for the response",
    "Receive the response",
];
const MUTED: Color = Color::Rgb(159, 181, 176);
const AMBER: Color = Color::Rgb(242, 186, 113);
const SELECTED: Color = Color::Rgb(32, 65, 54);

fn values(r: &Request) -> [Timing; STAGES] {
    [
        r.timings.blocked,
        r.timings.dns,
        r.timings.connect,
        r.timings.ssl,
        Timing::Milliseconds(r.timings.send),
        Timing::Milliseconds(r.timings.wait),
        Timing::Milliseconds(r.timings.receive),
    ]
}

fn layout(area: Rect) -> [Rect; 4] {
    // Extra terminal space should not stretch a small explanation into a poster.
    let width = area.width.min(140);
    let area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y,
        width,
        area.height.min(30),
    );
    let [exchange, accounting, panes] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .areas(area);
    let [stages, _, explanation] = Layout::horizontal([
        Constraint::Percentage(46),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(panes);
    [exchange, accounting, stages, explanation]
}

pub(super) fn detail_size(width: u16, height: u16) -> (u16, u16) {
    let [_, _, _, detail] = layout(Rect::new(0, 0, width, height.saturating_sub(4)));
    (
        detail.width.saturating_sub(2),
        detail.height.saturating_sub(2),
    )
}

fn explanation(r: &Request, stage: usize) -> Vec<String> {
    let stage = stage.min(STAGES - 1);
    let value = values(r)[stage];
    let meaning = match stage {
        0 => {
            "Time before the request could start. This field alone does not say what the client was waiting for."
        }
        1 => "DNS translates a host name into a network address.",
        2 => {
            "The client prepares a network connection. HAR includes reported TLS setup time within this interval."
        }
        3 => {
            "TLS sets up encryption for HTTPS. HAR calls this timing field ssl; it is part of Connect, not an extra interval to add."
        }
        4 => {
            "The client sends the request. This duration does not describe individual packets or their boundaries."
        }
        5 => {
            "After sending the request, the client waits for the response to begin. Network travel contributes as well as time before the server responds."
        }
        _ => {
            "The client receives the response. A response can contain several events; this duration cannot supply their individual arrival times."
        }
    };
    let reading = match value {
        Timing::Missing => {
            "The exporter did not retain a value here. That does not tell us whether the operation happened."
        }
        Timing::Unavailable => {
            "The exporter retained -1: no duration is available. It does not mean the operation took zero time."
        }
        Timing::ExporterNegative(_) => {
            "This negative exporter value is preserved as metadata, not a duration. It has no duration bar and is excluded from the known-stage sum."
        }
        Timing::Milliseconds(0.0) => {
            "A zero duration was reported. It may reflect the exporter's precision; it is different from a missing field. Its bar has no length."
        }
        Timing::Milliseconds(_) if stage == 5 => {
            "This is not a measurement of server processing alone. The capture cannot divide it into server work and network travel."
        }
        Timing::Milliseconds(_) if stage == 3 => {
            "Do not add this value again when summing Connect. The duration does not retain TLS messages or certificate results."
        }
        Timing::Milliseconds(_) => {
            "This value is reported by the exporter. The bar compares its duration with the other reported values; it is not a packet timeline."
        }
    };
    let mut lines = vec![
        format!("Reported: {}", view::timing(value)),
        String::new(),
        "WHAT THIS STAGE MEANS".into(),
        meaning.into(),
        String::new(),
        "READING THIS CAPTURE".into(),
        reading.into(),
    ];
    if stage == 3
        && r.url
            .get(..7)
            .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
    {
        lines.extend([
            String::new(),
            "This URL uses plain HTTP. An absent SSL field is not a TLS failure.".into(),
        ]);
    }
    if stage == 3
        && let Timing::Milliseconds(ssl) = r.timings.ssl
    {
        match r.timings.connect {
            Timing::Milliseconds(connect) if ssl > connect => lines.extend([
                String::new(), "The reported SSL value exceeds Connect. These values conflict; see Evidence.".into(),
            ]),
            Timing::Milliseconds(_) => {}
            _ => lines.extend([String::new(), "Connect has no duration here, so this capture cannot establish the reported nesting.".into()]),
        }
    }
    lines
}

pub(super) fn detail_lines(r: &Request, stage: usize, width: u16) -> Vec<String> {
    explanation(r, stage)
        .iter()
        .flat_map(|line| view::wrap_words(line, usize::from(width).max(2)))
        .collect()
}

fn accounting(r: &Request) -> (String, String, bool) {
    // TLS is already included in Connect. Unknown fields contribute no invented duration.
    let main = values(r)
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != 3)
        .map(|(_, value)| value);
    let sum = main
        .filter_map(|v| match v {
            Timing::Milliseconds(n) => Some(n),
            _ => None,
        })
        .try_fold(0.0_f64, |sum, n| {
            let next = sum + n;
            next.is_finite().then_some(next)
        });
    let Some(sum) = sum else {
        return (
            format!("Reported total: {} ms", r.total_ms),
            "Stage sum exceeds the numeric range; original values remain available.".into(),
            true,
        );
    };
    let different = (r.total_ms - sum).abs() > 0.01;
    let note = if different {
        format!(
            "{:.2} ms difference · cause not recorded · no time assigned to missing stages",
            (r.total_ms - sum).abs()
        )
    } else {
        "Sum matches total within 0.01 ms; missing values still remain unknown.".into()
    };
    let sum_label = if sum != 0.0 && !(0.01..1e9).contains(&sum) {
        format!("≈{sum:.3e}")
    } else if sum.fract() != 0.0 {
        format!("≈{sum:.2}")
    } else {
        format!("{sum}")
    };
    (
        format!(
            "Reported total: {} ms    Known stage sum: {sum_label} ms · excludes TLS",
            r.total_ms
        ),
        note,
        different,
    )
}

fn short_value(value: Timing) -> String {
    match value {
        Timing::Missing => "not recorded".into(),
        Timing::Unavailable => "unavailable".into(),
        Timing::Milliseconds(n) => format!("{n} ms"),
        Timing::ExporterNegative(n) => format!("{n} *"),
    }
}

fn bar_scale(r: &Request) -> f64 {
    values(r)
        .into_iter()
        .filter_map(|v| match v {
            Timing::Milliseconds(n) => Some(n),
            _ => None,
        })
        .fold(r.total_ms, f64::max)
}

fn draw_stages(frame: &mut Frame, area: Rect, r: &Request, stage: usize) {
    let block = workspace_view::block(" Reported stages · ↑/↓ ".into(), true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    view::line(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        if inner.width < 42 {
            "   Stage         Reported    Bar"
        } else {
            "   Stage         Reported       Relative"
        }
        .into(),
        Style::default().fg(MUTED),
    );
    let max = bar_scale(r);
    let bar_width = usize::from(inner.width.saturating_sub(27)).min(16);
    for (i, value) in values(r).into_iter().enumerate() {
        let label = format!(
            "{} {:<10} {:>12} ",
            if i == stage { ">" } else { " " },
            NAMES[i],
            view::clip(&short_value(value), 12)
        );
        let bar = match value {
            Timing::Milliseconds(n) if n > 0.0 && max > 0.0 => {
                "━".repeat(((n / max * bar_width as f64).ceil() as usize).min(bar_width))
            }
            Timing::Milliseconds(_) => String::new(),
            Timing::ExporterNegative(_) => String::new(),
            _ => "—".into(),
        };
        let base = if stage == i {
            Style::default().bg(SELECTED).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let line = Line::from(vec![
            Span::raw(label),
            Span::styled(bar, Style::default().fg(workspace_view::MINT)),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(base),
            Rect::new(inner.x, inner.y + i as u16 + 1, inner.width, 1),
        );
    }
    if inner.height > 9 {
        view::line(frame, Rect::new(inner.x, inner.y + 9, inner.width, inner.height - 9), "Bars compare durations.\nTLS is included in Connect.\n— no duration · * exporter value".into(), Style::default().fg(MUTED));
    }
}

pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    r: &Request,
    id: RequestId,
    stage: usize,
    offset: usize,
) {
    let stage = stage.min(STAGES - 1);
    let [exchange, totals, stages, detail] = layout(area);
    let title = format!(
        " Request #{} · {} · {} ",
        id.0,
        terminal_text(&r.method),
        view::status(r)
    );
    let block = workspace_view::block(title, false);
    let inner = block.inner(exchange);
    frame.render_widget(block, exchange);
    view::line(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        view::clip(&terminal_text(&r.url), usize::from(inner.width)),
        Style::default().fg(MUTED),
    );
    let host = view::clip(
        &terminal_text(&r.host),
        usize::from(inner.width).saturating_sub(20),
    );
    let padding = usize::from(inner.width)
        .saturating_sub(6 + unicode_width::UnicodeWidthStr::width(host.as_str()));
    view::line(
        frame,
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
        format!("Client{}{host}", " ".repeat(padding)),
        Style::default(),
    );
    let [_, flow, _] = Layout::horizontal([
        Constraint::Length(7),
        Constraint::Min(1),
        Constraint::Length(7),
    ])
    .areas(Rect::new(inner.x, inner.y + 2, inner.width, 2));
    let request = format!(" {} ", view::clip(&terminal_text(&r.method), 10));
    let response = if r.status == 0 {
        " no status retained ".into()
    } else {
        format!(" {} response ", r.status)
    };
    let arrows = [(request, false), (response, true)]
        .into_iter()
        .map(|(label, back)| {
            let count = usize::from(flow.width)
                .saturating_sub(unicode_width::UnicodeWidthStr::width(label.as_str()) + 1);
            let text = if back {
                format!(
                    "◀{}{}{}",
                    "─".repeat(count / 2),
                    label,
                    "─".repeat(count - count / 2)
                )
            } else {
                format!(
                    "{}{}{}▶",
                    "─".repeat(count / 2),
                    label,
                    "─".repeat(count - count / 2)
                )
            };
            Line::styled(
                text,
                Style::default().fg(if back && r.status >= 400 {
                    AMBER
                } else {
                    workspace_view::MINT
                }),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(arrows), flow);
    view::line(
        frame,
        Rect::new(inner.x, inner.y + 4, inner.width, 1),
        "Direction diagram · not a packet trace".into(),
        Style::default().fg(MUTED),
    );
    let (summary, note, different) = accounting(r);
    let [numbers, note_area, scale_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(totals);
    view::line(
        frame,
        numbers,
        summary,
        Style::default().fg(workspace_view::MINT),
    );
    view::line(
        frame,
        note_area,
        view::clip(&note, usize::from(note_area.width)),
        Style::default().fg(if different { AMBER } else { MUTED }),
    );
    let scale = bar_scale(r);
    let scale = if scale >= 1e9 {
        format!("{scale:.3e}")
    } else {
        format!("{scale}")
    };
    view::line(
        frame,
        scale_area,
        format!("Bar scale: 0–{scale} ms · TLS is part of Connect"),
        Style::default().fg(MUTED),
    );
    draw_stages(frame, stages, r, stage);
    let content = detail_lines(r, stage, detail.width.saturating_sub(2));
    let rows = usize::from(detail.height.saturating_sub(2));
    let offset = offset.min(content.len().saturating_sub(rows));
    let mut title = format!(" {} ", TITLES[stage]);
    if content.len() > rows {
        title = format!(" {} · PgUp/PgDn ", TITLES[stage]);
    }
    let block = workspace_view::block(title, false);
    let inner = block.inner(detail);
    frame.render_widget(block, detail);
    let lines = content
        .into_iter()
        .skip(offset)
        .take(rows)
        .map(|line| {
            let style = if line.starts_with("Reported:") {
                Style::default().fg(workspace_view::MINT)
            } else if line == "WHAT THIS STAGE MEANS" || line == "READING THIS CAPTURE" {
                Style::default().fg(MUTED)
            } else {
                Style::default()
            };
            Line::styled(line, style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}
