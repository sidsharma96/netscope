//! Explanations read recorded fields without changing packet analysis.
use crate::{
    presentation::{MINT, MUTED, SELECTED},
    view,
    workspace_view::block,
};
use crossterm::event::KeyCode;
use netscope_core::{
    packets::Packet,
    reassembly::{ByteState, Reassembled},
    terminal_text,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Debug, Default)]
pub(crate) struct Learning {
    topic: usize,
    offset: usize,
    page: usize,
}
impl Learning {
    pub(crate) fn key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.topic = self.topic.saturating_sub(1);
                self.offset = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.topic = (self.topic + 1).min(3);
                self.offset = 0;
            }
            KeyCode::Home => {
                self.topic = 0;
                self.offset = 0;
            }
            KeyCode::End => {
                self.topic = 3;
                self.offset = 0;
            }
            KeyCode::PageUp => self.offset = self.offset.saturating_sub(self.page.max(1)),
            KeyCode::PageDown => self.offset = self.offset.saturating_add(self.page.max(1)),
            _ => {}
        }
    }
    pub(crate) fn packet(&mut self, frame: &mut Frame, area: Rect, packet: &Packet, id: usize) {
        let (diagram, labels, explanation) = if let Some(tcp) = &packet.tcp {
            let start = tcp.sequence.wrapping_add(u32::from(tcp.flags & 2 != 0));
            let end = start.wrapping_add(tcp.declared_payload as u32);
            let ack = if tcp.flags & 16 != 0 {
                format!(
                    "ACK {} · peer's next expected position",
                    tcp.acknowledgement
                )
            } else {
                format!(
                    "ACK field {} · inactive (ACK flag off)",
                    tcp.acknowledgement
                )
            };
            let diagram = vec![
                format!("Packet #{id} · {} ── TCP → {}", tcp.source, tcp.destination),
                format!(
                    "Sequence {} · {} retained / {} declared data bytes",
                    tcp.sequence,
                    tcp.payload.len(),
                    tcp.declared_payload
                ),
                format!("Flags: {} · raw 0x{:02x}", flag_names(tcp.flags), tcp.flags),
                ack,
            ];
            let labels = vec![
                "Flags · control bits".into(),
                "Sequence · this direction".into(),
                "ACK · other direction".into(),
                "Layers and stored bytes".into(),
            ];
            let explanation = match self.topic {
                0 => flag_explanation(tcp.flags),
                1 => vec!["WHERE DO THESE BYTES BELONG?".into(),
                    "TCP numbers bytes separately in each direction. A sequence number is a byte position, not a packet count.".into(),
                    format!("This header says Sequence {}. Its declared data starts at {start} and ends just before {end}.", tcp.sequence),
                    if tcp.declared_payload == 0 { "No application bytes are declared in this segment.".into() } else { format!("{start} ├── {} declared bytes ──┤ {end} (end excluded)", tcp.declared_payload) },
                    "SYN consumes one position before data. FIN consumes one after data. Neither becomes an application byte.".into(),
                    "Numbers wrap from 4294967295 to 0. An end smaller than its start can cross that boundary.".into(),
                    "Declared bytes may exceed the bytes retained in this capture. The diagram does not fill missing data.".into()],
                2 => vec!["WHAT IS BEING ACKNOWLEDGED?".into(),
                    ack_explanation(tcp.flags, tcp.acknowledgement),
                    "Sequence describes data going from this sender to its peer. ACK describes progress in the opposite direction.".into(),
                    "Illustration only · ACK flag set".into(),
                    "Peer ── data [100..103) abc ──→ Sender\nPeer ←────────── ACK 103 ───── Sender".into(),
                    "ACK 103 means position 103 is expected next.".into(),
                    "An ACK reports TCP receipt, not that an application read or acted on those bytes. It cannot recover unseen payload.".into(),
                    format!("Window field: {} (raw, unscaled). Netscope does not parse window-scale options, so this is not a calculated byte capacity.", tcp.window)],
                _ => vec!["READ FROM THE OUTSIDE IN".into(),
                    "A layer adds addressing or control information around the data it carries. The path below is illustrative; recorded layers follow it.".into(),
                    "Link → IP → TCP → application bytes".into(),
                    format!("Stored payload slice: [{}..{}). These are offsets inside this packet's stored bytes, not TCP sequence numbers.", tcp.payload.start, tcp.payload.end),
                    "A range includes its start and excludes its end. Subtract start from end to count its bytes.".into(),
                    packet.layers.iter().map(|s| terminal_text(s)).collect::<Vec<_>>().join("\n"),
                    "Press 2 for the complete recorded report and hex bytes. Checksums and receiver acceptance are not verified.".into()],
            };
            (diagram, labels, explanation)
        } else {
            (vec![format!("Packet #{id} · {} stored / {} original bytes", packet.bytes.len(), packet.original_len),
                "No supported TCP header is available for this packet.".into()],
             vec!["Recorded layers".into(), "Why no TCP fields?".into(), "Stored bytes".into(), "Capture limits".into()],
             match self.topic {
                0 => vec!["RECORDED LAYERS".into(), "Each supported layer describes the data inside it. These are the reader's recorded layer findings:".into(), packet.layers.iter().map(|s| terminal_text(s)).collect::<Vec<_>>().join("\n")],
                1 => vec!["WHY NO TCP FIELDS?".into(), if packet.udp.is_some() { "This packet carries UDP. UDP has no TCP sequence number, ACK field or TCP flags.".into() } else { "This may be another protocol, an unsupported layer, or an incomplete header. Missing TCP fields do not mean their values were zero.".into() }],
                2 => vec!["WHAT BYTES ARE STORED?".into(), format!("This record retains {} of its {} reported original bytes.", packet.bytes.len(), packet.original_len), "Press 2 for the byte report and hex preview. Hexadecimal writes each byte with two base-sixteen digits; it is a representation, not decryption.".into()],
                _ => vec!["WHAT IS STILL UNKNOWN?".into(), "A recorded packet does not prove receiver acceptance. The reader does not verify checksums, reassemble IP fragments or decrypt encrypted application data.".into(), "Unsupported bytes remain stored. Press 2 for the full report, including any parsed datagram evidence.".into()],
             })
        };
        self.draw(frame, area, diagram, labels, explanation);
    }
    pub(crate) fn stream(&mut self, frame: &mut Frame, area: Rect, flow: &Reassembled, id: usize) {
        let missing = flow
            .states
            .iter()
            .filter(|s| **s == ByteState::Missing)
            .count();
        let conflict = flow
            .states
            .iter()
            .filter(|s| **s == ByteState::Conflict)
            .count();
        let prefix = flow.contiguous_prefix().len();
        let diagram = vec![
            format!(
                "Stream #{id} · {} → {} · one direction",
                flow.source, flow.destination
            ),
            format!(
                "0 [{}] {} positions",
                coverage(&flow.states, 40),
                flow.states.len()
            ),
            "= observed · ? missing · ! conflict · each cell groups positions".into(),
            format!(
                "{} observed · {missing} missing · {conflict} conflicting · prefix {prefix}",
                flow.states.len() - missing - conflict
            ),
        ];
        let labels = vec![
            "Coverage · what is known?".into(),
            "Ranges · exact positions".into(),
            "Beginning and end".into(),
            "What can HTTP read?".into(),
        ];
        let explanation = match self.topic {
            0 => vec!["READ THE MAP FROM LEFT TO RIGHT".into(),
                "This is byte position, not time. One cell groups neighbouring positions; ! takes priority over ?, then =. A gap cannot hide inside an = cell; the counts remain exact.".into(),
                "Observed means recorded copies agree. Missing means no retained payload supplies that position. Conflict means copies disagree.".into(),
                format!("The first {prefix} positions are uniquely observed. A parser can read this prefix without crossing a hole or choosing between conflicting bytes."),
                format!("{} agreeing repeated byte observations. A repeat can be retransmission or duplicate capture; the cause is not established.", flow.repeated_bytes),
                format!("{} segments were skipped by analysis limits. The map describes only the retained span, not the entire connection.", flow.skipped_segments)],
            1 => {
                let mut lines = vec!["HOW TO READ [START..END)".into(),
                    "The start is included; the end is excluded. [3..6) contains positions 3, 4 and 5: three bytes.".into(),
                    format!("Position 0 corresponds to TCP sequence {}. Add an offset with 32-bit wrap-around to find its sequence number.", flow.first_sequence),
                    "EXACT RETAINED RANGES".into()];
                lines.extend(ranges(&flow.states)); lines
            }
            2 => vec!["DID WE SEE THE WHOLE DIRECTION?".into(),
                format!("SYN recorded: {} · FIN recorded: {} · reset recorded: {}", flow.syn_observed, flow.fin_observed, flow.reset_observed),
                "SYN starts sequence numbering; FIN ends data in this direction; RST signals an abrupt reset.".into(),
                if flow.syn_observed { "A SYN supplied the beginning of this grouping. That alone does not prove a successful handshake.".into() } else { "Start unknown: there is no SYN for this grouping. Position 0 is an observation anchor, not necessarily the connection's first byte.".into() },
                "An epoch is a separate group of observations in one direction. Netscope does not pair both directions into a verified connection history.".into(),
                "A FIN alone does not prove a complete capture. Gaps, conflicts, skipped segments and ambiguous end observations still matter.".into()],
            _ => vec!["WHY STOP AT A HOLE?".into(),
                format!("Uniquely observed prefix: {prefix} bytes. HTTP inspection stops before the first missing or conflicting position."),
                "Illustration: abc???ghi must not become abcghi. Removing the gap would invent adjacent bytes.".into(),
                "Even a gap-free prefix may be encrypted, incomplete, or another protocol. Coverage is not proof of a complete HTTP message.".into(),
                "Press 2 to read the existing HTTP framing evidence, headers, chunks, trailers and byte previews.".into(),
                "Responses are not paired with request methods. Message interpretation remains conditional; encrypted content stays unreadable.".into()],
        };
        self.draw(frame, area, diagram, labels, explanation);
    }
    fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        diagram: Vec<String>,
        labels: Vec<String>,
        explanation: Vec<String>,
    ) {
        let area = Rect::new(
            area.x + area.width.saturating_sub(140) / 2,
            area.y,
            area.width.min(140),
            area.height.min(30),
        );
        let [tabs, summary, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(6),
            Constraint::Min(1),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new("[1 Explain]  2 Recorded report · ↑↓ topic · PgUp/PgDn explanation")
                .style(Style::default().fg(MINT)),
            tabs,
        );
        let b = block(
            " Recorded values · diagrams are not a delivery trace ".into(),
            false,
        );
        let inner = b.inner(summary);
        frame.render_widget(b, summary);
        frame.render_widget(
            Paragraph::new(
                diagram
                    .into_iter()
                    .map(|text| {
                        if text.starts_with("0 [") {
                            Line::from(
                                text.chars()
                                    .map(|c| {
                                        Span::styled(
                                            c.to_string(),
                                            match c {
                                                '?' | '!' => Style::default()
                                                    .fg(crate::presentation::AMBER)
                                                    .add_modifier(Modifier::BOLD),
                                                '=' => Style::default().fg(MINT),
                                                _ => Style::default(),
                                            },
                                        )
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            Line::from(text)
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
        let [left, right] =
            Layout::horizontal([Constraint::Length(29), Constraint::Min(1)]).areas(body);
        let b = block(" Choose a topic ".into(), true);
        let inner = b.inner(left);
        frame.render_widget(b, left);
        frame.render_widget(
            Paragraph::new(
                labels
                    .into_iter()
                    .enumerate()
                    .map(|(i, text)| {
                        Line::styled(
                            format!("{} {text}", if i == self.topic { ">" } else { " " }),
                            if i == self.topic {
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
            ),
            inner,
        );
        let b = block(" What this means ".into(), false);
        let inner = b.inner(right);
        frame.render_widget(b, right);
        let lines = explanation_lines(&explanation, usize::from(inner.width).max(2));
        self.page = usize::from(inner.height).max(1);
        self.offset = self.offset.min(lines.len().saturating_sub(self.page));
        let position = format!(
            " {}–{} / {} · PgDn for more ",
            self.offset + 1,
            (self.offset + self.page).min(lines.len()),
            lines.len()
        );
        frame.render_widget(
            Paragraph::new(
                lines
                    .iter()
                    .skip(self.offset)
                    .take(self.page)
                    .cloned()
                    .map(crate::presentation::report_line)
                    .collect::<Vec<_>>(),
            ),
            inner,
        );
        if lines.len() > self.page {
            view::line(
                frame,
                Rect::new(right.x + 1, right.bottom() - 1, right.width - 2, 1),
                position,
                Style::default().fg(MUTED),
            );
        }
    }
}

fn explanation_lines(paragraphs: &[String], width: usize) -> Vec<String> {
    paragraphs
        .iter()
        .flat_map(|text| {
            text.lines()
                .flat_map(|line| view::wrap_words(line, width))
                .chain(std::iter::once(String::new()))
        })
        .collect()
}

fn ack_explanation(flags: u8, value: u32) -> String {
    if flags & 16 == 0 {
        format!(
            "ACK flag is off. The raw field is {value}, but it is not an active acknowledgement. Do not treat it as receipt of byte {value}."
        )
    } else {
        format!(
            "ACK flag is on. ACK {value} reports the next sequence position expected from the peer; earlier contiguous sequence space is acknowledged. The value is not a count of packets."
        )
    }
}
const FLAGS: [(u8, &str, &str); 8] = [
    (
        2,
        "SYN",
        "Start sequence numbering; consumes one sequence position.",
    ),
    (
        16,
        "ACK",
        "The acknowledgement field is active; it describes the other direction.",
    ),
    (
        8,
        "PSH",
        "Push data onward to the receiving application; not an HTTP message boundary.",
    ),
    (
        1,
        "FIN",
        "No more data from this sender; consumes one sequence position.",
    ),
    (
        4,
        "RST",
        "Reset the connection abruptly; this does not explain the cause.",
    ),
    (
        32,
        "URG",
        "The urgent-pointer field is active; Netscope does not decode it.",
    ),
    (
        64,
        "ECE",
        "Used in explicit congestion notification; interpretation depends on SYN and negotiation.",
    ),
    (
        128,
        "CWR",
        "Used to signal congestion-window reduction in explicit congestion notification.",
    ),
];
fn flag_names(flags: u8) -> String {
    let names = FLAGS
        .iter()
        .filter(|(mask, _, _)| flags & mask != 0)
        .map(|(_, name, _)| *name)
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none set".into()
    } else {
        names.join(" + ")
    }
}
fn flag_explanation(flags: u8) -> Vec<String> {
    let mut lines = vec!["WHAT IS THIS SEGMENT SAYING?".into(), "Flags are independent on/off bits in the TCP header. Several can be set together. Hexadecimal (0x) is a compact way to write those bits.".into()];
    for (mask, name, meaning) in FLAGS {
        lines.push(format!(
            "{name} {} · when set: {meaning}",
            if flags & mask != 0 { "[on]" } else { "[off]" }
        ));
    }
    lines.push("These are recorded signals, not proof that the peer accepted them.".into());
    lines
}
fn coverage(states: &[ByteState], width: usize) -> String {
    if states.is_empty() {
        return "no payload positions".into();
    }
    let cells = width.max(1).min(states.len());
    (0..cells)
        .map(|i| {
            let slice = &states[i * states.len() / cells..(i + 1) * states.len() / cells];
            if slice.contains(&ByteState::Conflict) {
                '!'
            } else if slice.contains(&ByteState::Missing) {
                '?'
            } else {
                '='
            }
        })
        .collect()
}
fn ranges(states: &[ByteState]) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < states.len() && result.len() < 64 {
        let state = states[start];
        let end = states[start..]
            .iter()
            .position(|s| *s != state)
            .map_or(states.len(), |n| start + n);
        result.push(format!(
            "[{start}..{end}) · {} bytes · {state:?}",
            end - start
        ));
        start = end;
    }
    if start < states.len() {
        result.push("Range display stops after 64 runs; retained bytes are unchanged.".into());
    }
    if states.is_empty() {
        result.push("No payload positions retained.".into());
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagrams_and_layer_lists_keep_their_explicit_line_breaks() {
        assert_eq!(
            explanation_lines(&["Peer → Sender\nPeer ← Sender".into()], 80),
            vec!["Peer → Sender", "Peer ← Sender", ""]
        );
    }
    #[test]
    fn coverage_preserves_small_holes_and_conflicts_and_handles_empty_spans() {
        let mut states = vec![ByteState::Observed; 100];
        states[1] = ByteState::Missing;
        states[99] = ByteState::Conflict;
        assert_eq!(coverage(&states, 4), "?==!");
        assert_eq!(coverage(&[], 4), "no payload positions");
        assert_eq!(coverage(&[ByteState::Observed], 40), "=");
        assert_eq!(coverage(&states, 0), "!");
        assert_eq!(ranges(&states)[1], "[1..2) · 1 bytes · Missing");
    }
    #[test]
    fn ack_and_combined_flags_do_not_invent_receiver_progress() {
        assert!(ack_explanation(2, 900).contains("not an active acknowledgement"));
        assert!(ack_explanation(16, 0).contains("next sequence position"));
        assert_eq!(flag_names(0x18), "ACK + PSH");
        assert_eq!(flag_names(0), "none set");
        assert_eq!(flag_names(255).split(" + ").count(), 8);
    }
}
