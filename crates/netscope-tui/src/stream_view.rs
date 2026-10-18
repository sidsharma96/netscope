//! Select recorded byte ranges without filling holes or choosing conflicting values.
use crate::{
    presentation::{AMBER, MINT, MUTED, SELECTED},
    view,
    workspace_view::block,
};
use crossterm::event::KeyCode;
use netscope_core::reassembly::{ByteState, Reassembled};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use std::ops::Range;

#[derive(Debug, Default)]
pub(crate) struct StreamInspector {
    ranges: Vec<Range<usize>>,
    selected: usize,
    hex: bool,
    text_offset: usize,
    byte_offset: usize,
    text_page: usize,
    byte_page: usize,
}
impl StreamInspector {
    pub(crate) fn new(flow: &Reassembled) -> Self {
        let mut ranges = Vec::new();
        let mut start = 0;
        for end in 1..=flow.states.len() {
            if end == flow.states.len() || flow.states[end] != flow.states[start] {
                ranges.push(start..end);
                start = end;
            }
        }
        let selected = ranges
            .iter()
            .position(|r| flow.states[r.start] != ByteState::Observed)
            .unwrap_or(0);
        Self {
            ranges,
            selected,
            ..Self::default()
        }
    }
    pub(crate) fn key(&mut self, key: KeyCode) {
        let old = self.selected;
        match key {
            KeyCode::Up | KeyCode::Left | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j') => {
                self.selected = self
                    .selected
                    .saturating_add(1)
                    .min(self.ranges.len().saturating_sub(1))
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.ranges.len().saturating_sub(1),
            KeyCode::Char('b') => {
                self.hex = !self.hex;
                self.byte_offset = 0;
            }
            KeyCode::PageDown => {
                self.text_offset = self.text_offset.saturating_add(self.text_page.max(1))
            }
            KeyCode::PageUp => {
                self.text_offset = self.text_offset.saturating_sub(self.text_page.max(1))
            }
            KeyCode::Char(']') => {
                self.byte_offset = self.byte_offset.saturating_add(self.byte_page.max(1))
            }
            KeyCode::Char('[') => {
                self.byte_offset = self.byte_offset.saturating_sub(self.byte_page.max(1))
            }
            _ => {}
        }
        if old != self.selected {
            self.text_offset = 0;
            self.byte_offset = 0;
        }
    }
    pub(crate) fn draw(&mut self, frame: &mut Frame, area: Rect, flow: &Reassembled, id: usize) {
        let [tabs, identity, map, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .areas(area);
        view::line(
            frame,
            tabs,
            "[1 Inspect]  2 Recorded report · offline capture".into(),
            Style::default().fg(MINT),
        );
        let missing = flow
            .states
            .iter()
            .filter(|s| **s == ByteState::Missing)
            .count();
        let conflicts = flow
            .states
            .iter()
            .filter(|s| **s == ByteState::Conflict)
            .count();
        frame.render_widget(Paragraph::new(vec![Line::from(format!("Stream #{id} · {} → {} · one direction", flow.source, flow.destination)), Line::styled(format!("{} captured · {missing} missing · {conflicts} conflicting · readable start {} bytes", flow.states.len()-missing-conflicts, flow.contiguous_prefix().len()), Style::default().fg(MUTED))]), identity);
        let b = block(" BYTE COVERAGE · position, not time ".into(), false);
        let inner = b.inner(map);
        frame.render_widget(b, map);
        let selected = self.ranges.get(self.selected).cloned().unwrap_or(0..0);
        let cells = coverage(&flow.states, usize::from(inner.width), &selected);
        let width = usize::from(inner.width);
        let end = format!("{}", flow.states.len());
        let axis = format!(
            "0{}{}",
            " ".repeat(width.saturating_sub(end.len() + 1)),
            end
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(axis, Style::default().fg(MUTED)),
                Line::from(cells),
                Line::from(
                    "= captured  ? missing  ! conflict · highlighted cells touch selected range",
                ),
            ]),
            inner,
        );
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(body);
        let [table, bytes] = Layout::vertical([
            Constraint::Length((left.height / 2).max(5)),
            Constraint::Min(1),
        ])
        .areas(left);
        self.draw_ranges(frame, table, flow);
        self.draw_bytes(frame, bytes, flow, &selected);
        self.draw_explanation(frame, right, flow, &selected);
    }
    fn draw_ranges(&self, frame: &mut Frame, area: Rect, flow: &Reassembled) {
        let b = block(
            format!(
                " RANGES · {} / {} · ↑↓ ",
                if self.ranges.is_empty() {
                    0
                } else {
                    self.selected + 1
                },
                self.ranges.len()
            ),
            true,
        );
        let inner = b.inner(area);
        frame.render_widget(b, area);
        let page = usize::from(inner.height.saturating_sub(1)).max(1);
        let top = self.selected.saturating_sub(page - 1);
        let mut lines = vec![Line::styled(
            "  Offsets         Bytes  Evidence",
            Style::default().fg(MUTED),
        )];
        for (i, r) in self.ranges.iter().enumerate().skip(top).take(page) {
            lines.push(Line::styled(
                format!(
                    "{} {:<14} {:>6}  {}",
                    if i == self.selected { ">" } else { " " },
                    format!("{}–{}", r.start, r.end - 1),
                    r.len(),
                    label(flow.states[r.start])
                ),
                if i == self.selected {
                    Style::default().fg(MINT).bg(SELECTED)
                } else {
                    Style::default()
                },
            ));
        }
        if self.ranges.is_empty() {
            lines.push(Line::from("No payload positions retained."));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }
    fn draw_bytes(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        flow: &Reassembled,
        range: &Range<usize>,
    ) {
        let b = block(
            format!(
                " BYTES · {} · b toggles · [ ] page ",
                if self.hex { "hex" } else { "text" }
            ),
            false,
        );
        let inner = b.inner(area);
        frame.render_widget(b, area);
        let rows = byte_rows(
            flow,
            range,
            usize::from(inner.width).saturating_sub(8).max(4),
            self.hex,
        );
        self.byte_page = usize::from(inner.height).max(1);
        self.byte_offset = self
            .byte_offset
            .min(rows.len().saturating_sub(self.byte_page));
        let lines = rows
            .iter()
            .skip(self.byte_offset)
            .take(self.byte_page)
            .map(|r| {
                let text = r
                    .clone()
                    .map(|i| byte_text(flow.states[i], flow.bytes[i], self.hex))
                    .collect::<Vec<_>>()
                    .join(if self.hex { " " } else { "" });
                Line::from(format!("{:>6}  {text}", r.start))
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), inner);
    }
    fn draw_explanation(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        flow: &Reassembled,
        range: &Range<usize>,
    ) {
        let b = block(" UNDERSTAND THIS RANGE ".into(), false);
        let inner = b.inner(area);
        frame.render_widget(b, area);
        let paragraphs = explanation(flow, range);
        let lines = paragraphs
            .iter()
            .flat_map(|s| {
                view::wrap_words(s, usize::from(inner.width).max(1))
                    .into_iter()
                    .chain(
                        (!s.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()))
                            .then(String::new),
                    )
            })
            .collect::<Vec<_>>();
        self.text_page = usize::from(inner.height).max(1);
        self.text_offset = self
            .text_offset
            .min(lines.len().saturating_sub(self.text_page));
        frame.render_widget(
            Paragraph::new(
                lines
                    .iter()
                    .skip(self.text_offset)
                    .take(self.text_page)
                    .cloned()
                    .map(crate::presentation::report_line)
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
        if lines.len() > self.text_page {
            view::line(
                frame,
                Rect::new(
                    area.x + 1,
                    area.bottom() - 1,
                    area.width.saturating_sub(2),
                    1,
                ),
                format!(
                    " {}–{} / {} · PgUp/PgDn ",
                    self.text_offset + 1,
                    (self.text_offset + self.text_page).min(lines.len()),
                    lines.len()
                ),
                Style::default().fg(MUTED),
            );
        }
    }
}
fn byte_rows(
    flow: &Reassembled,
    range: &Range<usize>,
    width: usize,
    hex: bool,
) -> Vec<Range<usize>> {
    let mut rows = Vec::new();
    let mut start = range.start;
    let mut used = 0;
    for i in range.clone() {
        let size = byte_text(flow.states[i], flow.bytes[i], hex).len();
        let separator = usize::from(hex && i > start);
        if used + separator + size > width && i > start {
            rows.push(start..i);
            start = i;
            used = 0;
        }
        used += size + usize::from(hex && i > start);
    }
    if start < range.end {
        rows.push(start..range.end);
    }
    rows
}
fn label(state: ByteState) -> &'static str {
    match state {
        ByteState::Observed => "= Captured",
        ByteState::Missing => "? Missing",
        ByteState::Conflict => "! Conflict",
    }
}
fn byte_text(state: ByteState, byte: u8, hex: bool) -> String {
    match state {
        ByteState::Missing => if hex { "??" } else { "?" }.into(),
        ByteState::Conflict => if hex { "!!" } else { "!" }.into(),
        ByteState::Observed if hex => format!("{byte:02x}"),
        ByteState::Observed => match byte {
            b'\r' => "\\r".into(),
            b'\n' => "\\n".into(),
            b'\t' => "\\t".into(),
            b'\\' => "\\\\".into(),
            b'?' | b'!' => format!("\\x{byte:02x}"),
            32..=126 => char::from(byte).to_string(),
            _ => format!("\\x{byte:02x}"),
        },
    }
}
fn coverage(states: &[ByteState], width: usize, selected: &Range<usize>) -> Vec<Span<'static>> {
    let cells = if states.is_empty() { 0 } else { width };
    (0..cells)
        .map(|i| {
            let start = i * states.len() / cells;
            let end = ((i + 1) * states.len() / cells).max(start + 1);
            let slice = &states[start..end];
            let (symbol, color) = if slice.contains(&ByteState::Conflict) {
                ("!", AMBER)
            } else if slice.contains(&ByteState::Missing) {
                ("?", AMBER)
            } else {
                ("=", MINT)
            };
            let mut style = Style::default().fg(color);
            if start < selected.end && end > selected.start {
                style = style.bg(SELECTED);
            }
            Span::styled(symbol, style)
        })
        .collect()
}
fn explanation(flow: &Reassembled, range: &Range<usize>) -> Vec<String> {
    let Some(state) = flow.states.get(range.start) else {
        return vec!["NO PAYLOAD POSITIONS".into(),"This group contains no retained application byte positions. Control packets can exist without application data.".into(),"Press 2 for recorded flags, analysis limits and other evidence.".into()];
    };
    let (title, meaning, consequence) = match state {
        ByteState::Missing => (
            "MISSING FROM THE CAPTURE",
            "No captured data supplies these positions. This alone does not prove network loss.",
            "HTTP reading stops before this gap. Joining the two sides would invent adjacent bytes.",
        ),
        ByteState::Conflict => (
            "RECORDED COPIES DISAGREE",
            "At each of these positions, captured copies contain different values. Netscope does not choose which copy the receiver accepted.",
            "HTTP inspection cannot cross a conflict. The preview marks uncertainty instead of showing one disputed value as fact.",
        ),
        ByteState::Observed if range.start == 0 => (
            "THE BEGINNING IS READABLE",
            "Captured copies agree. These bytes can be read in order from this grouping's start.",
            "This is the prefix available to HTTP inspection. It can still be incomplete, encrypted, or another protocol.",
        ),
        ByteState::Observed => (
            "LATER BYTES ARE AVAILABLE",
            "These bytes agree, but an earlier gap or conflict separates them from the readable beginning.",
            "Inspect these bytes separately. They cannot be joined directly to the beginning while ignoring the uncertain positions.",
        ),
    };
    vec![title.into(),meaning.into(),"WHAT THIS CHANGES".into(),consequence.into(),format!("Offsets {}–{} include both ends: {} bytes.",range.start,range.end-1,range.len()),format!("Readable start: {} bytes. Press 2 for HTTP findings and the full recorded report.",flow.contiguous_prefix().len()),"READING THE PREVIEW".into(),"? means missing; ! means conflict. In hex mode these become ?? and !!. Text shows ASCII bytes; \\r and \\n are line endings. Other bytes use \\xNN escapes, including literal ? and !. Each escape represents one byte.".into(),"WHAT IS AN OFFSET?".into(),format!("An offset counts positions from zero, not time or packets. Here zero corresponds to TCP sequence {}. Sequence numbers wrap after 4294967295.",flow.first_sequence),if flow.syn_observed{"A SYN start signal was observed. That does not prove the handshake succeeded.".into()}else{"No SYN start signal was observed. Offset zero may be partway through the connection.".into()},format!("{} segments excluded by analysis limits. Coverage describes the retained span, not a verified complete connection.",flow.skipped_segments),"Each map cell groups neighbouring positions. Conflict takes priority over missing, then captured. Highlighted cells touch this range; the table gives its exact boundaries.".into()]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flow(states: Vec<ByteState>) -> Reassembled {
        Reassembled {
            source: ([192, 0, 2, 1], 1234).into(),
            destination: ([192, 0, 2, 2], 80).into(),
            packets: vec![],
            first_sequence: u32::MAX,
            syn_observed: false,
            fin_observed: false,
            reset_observed: false,
            bytes: vec![0; states.len()],
            states,
            repeated_bytes: 0,
            ignored_probes: 0,
            skipped_segments: 0,
            notes: vec![],
        }
    }
    #[test]
    fn map_never_hides_uncertainty_and_marks_selected_overlap() {
        let mut states = vec![ByteState::Observed; 100];
        states[1] = ByteState::Missing;
        states[99] = ByteState::Conflict;
        let map = coverage(&states, 4, &(1..2));
        assert_eq!(
            map.iter().map(|s| s.content.as_ref()).collect::<String>(),
            "?==!"
        );
        assert_eq!(map[0].style.bg, Some(SELECTED));
        assert_eq!(map[1].style.bg, None);
        assert_eq!(coverage(&[ByteState::Missing], 80, &(0..1)).len(), 80);
        assert!(coverage(&[], 80, &(0..0)).is_empty());
        assert!(coverage(&states, 0, &(0..1)).is_empty());
    }
    #[test]
    fn every_range_is_reachable_and_selection_resets_only_local_pages() {
        let flow = flow(
            (0..130)
                .map(|i| {
                    if i % 2 == 0 {
                        ByteState::Observed
                    } else {
                        ByteState::Missing
                    }
                })
                .collect(),
        );
        let mut ui = StreamInspector::new(&flow);
        assert_eq!(ui.selected, 1);
        assert_eq!(ui.ranges.len(), 130);
        ui.key(KeyCode::End);
        assert_eq!(ui.selected, 129);
        ui.key(KeyCode::Down);
        assert_eq!(ui.selected, 129);
        ui.key(KeyCode::PageDown);
        ui.key(KeyCode::Char(']'));
        ui.key(KeyCode::Char('b'));
        ui.key(KeyCode::Up);
        assert_eq!((ui.selected, ui.text_offset, ui.byte_offset), (128, 0, 0));
        assert!(ui.hex);
        ui.key(KeyCode::Home);
        ui.key(KeyCode::Up);
        assert_eq!(ui.selected, 0);
        let mut empty = StreamInspector::new(&self::flow(vec![]));
        empty.key(KeyCode::End);
        assert_eq!(empty.selected, 0);
    }
    #[test]
    fn previews_never_expose_placeholder_or_disputed_values() {
        assert_eq!(byte_text(ByteState::Missing, 0, true), "??");
        assert_eq!(byte_text(ByteState::Conflict, b'A', true), "!!");
        assert_eq!(byte_text(ByteState::Observed, 0, true), "00");
        assert_eq!(byte_text(ByteState::Observed, 27, false), "\\x1b");
        assert_eq!(byte_text(ByteState::Observed, b'?', false), "\\x3f");
        assert_eq!(byte_text(ByteState::Observed, b'!', false), "\\x21");
        let mut f = flow(vec![ByteState::Observed; 6]);
        f.bytes = b"a\r\n?\0z".to_vec();
        assert_eq!(byte_rows(&f, &(0..6), 5, false), vec![0..3, 3..4, 4..6]);
        assert_eq!(byte_rows(&f, &(0..6), 5, true), vec![0..2, 2..4, 4..6]);
    }
}
