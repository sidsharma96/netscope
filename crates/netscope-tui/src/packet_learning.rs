//! Explanations read recorded fields without changing packet analysis.
use crate::{
    presentation::{MINT, MUTED, SELECTED},
    view,
    workspace_view::block,
};
use crossterm::event::KeyCode;
use netscope_core::{packets::Packet, terminal_text};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::Paragraph,
};

#[derive(Debug, Default)]
pub(crate) struct Learning {
    topic: usize,
    help: bool,
    offset: usize,
    page: usize,
}
impl Learning {
    pub(crate) fn key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('h') => {
                self.help = !self.help;
                self.offset = 0;
            }
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
        let explanation = if self.help {
            explanation
        } else {
            packet_summary(packet, self.topic)
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
            Paragraph::new("[1 Explain]  2 Recorded report · ↑↓ field · h detailed help")
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
            Paragraph::new(diagram.into_iter().map(Line::from).collect::<Vec<_>>()),
            inner,
        );
        let [left, right] =
            Layout::horizontal([Constraint::Length(29), Constraint::Min(1)]).areas(body);
        let left = Rect {
            height: left.height.min(6),
            ..left
        };
        let b = block(" FIELDS · ↑↓ selects ".into(), true);
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
        let b = block(
            if self.help {
                " FIELD HELP · h closes "
            } else {
                " SELECTED FIELD · h explains more "
            }
            .into(),
            false,
        );
        let inner = b.inner(right);
        let width = usize::from(inner.width).max(2);
        let lines = if self.help {
            explanation_lines(&explanation, width)
        } else {
            explanation
                .iter()
                .enumerate()
                .flat_map(|(i, s)| {
                    s.lines()
                        .flat_map(|line| view::wrap_words(line, width))
                        .chain((i > 0 && i + 1 < explanation.len()).then(String::new))
                })
                .collect()
        };
        let right = if self.help {
            right
        } else {
            Rect {
                height: right.height.min((lines.len() + 2) as u16),
                ..right
            }
        };
        let inner = b.inner(right);
        frame.render_widget(b, right);
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

fn packet_summary(packet: &Packet, topic: usize) -> Vec<String> {
    let Some(tcp) = &packet.tcp else {
        return vec![
            "NO SUPPORTED TCP HEADER".into(),
            if packet.udp.is_some() {
                "This packet carries UDP. It has no TCP sequence, ACK or flags.".into()
            } else {
                "The TCP header is absent, unsupported or incomplete. Missing fields are not zero values.".into()
            },
            "Press 2 for recorded layers and bytes; h explains more.".into(),
        ];
    };
    match topic {
        0 => {
            let mut lines = vec!["CONTROL FLAGS".into()];
            lines.extend(FLAGS.iter().filter(|(mask,_,_)| tcp.flags & mask != 0).map(|(_,name,meaning)| format!("{name} · {meaning}")));
            if tcp.flags == 0 { lines.push("No control flags are set.".into()); }
            lines.push("Flags are on/off instructions. They do not prove the peer accepted this packet. Press h for all definitions.".into()); lines
        },
        1 => { let start = tcp.sequence.wrapping_add(u32::from(tcp.flags & 2 != 0)); let end = start.wrapping_add(tcp.declared_payload as u32);
            vec!["WHERE DO THESE BYTES BELONG?".into(), format!("Data starts at {start}; ends before {end}. {} bytes declared, {} retained.", tcp.declared_payload, tcp.payload.len()), "Sender ── numbered bytes ──→ Peer".into(), "Sequence numbers locate bytes in this direction. SYN uses one position before data; FIN uses one after. Press h for wrap-around and capture limits.".into()]
        },
        2 => vec!["WHAT IS BEING ACKNOWLEDGED?".into(), if tcp.flags & 16 == 0 { format!("ACK flag is off. Field {} is inactive; it reports no receipt.", tcp.acknowledgement) } else {format!("ACK {}: this sender expects that position next from its peer.", tcp.acknowledgement)}, "Sender ←── peer's data ───── Peer
Sender ── ACK for that data → Peer".into(), "Direction illustration, not a delivery trace. ACK reports TCP receipt, not application processing.".into()],
        _ => vec!["READ FROM THE OUTSIDE IN".into(), format!("{} bytes stored. TCP data occupies packet offsets [{}..{}).",packet.bytes.len(),tcp.payload.start,tcp.payload.end), "A layer carries the next layer inside it. Packet offsets locate stored bytes; sequence numbers locate bytes in a stream.".into(), "Press 2 for recorded layers and hex; h explains the boundaries.".into()],
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
    fn ack_and_combined_flags_do_not_invent_receiver_progress() {
        assert!(ack_explanation(2, 900).contains("not an active acknowledgement"));
        assert!(ack_explanation(16, 0).contains("next sequence position"));
        assert_eq!(flag_names(0x18), "ACK + PSH");
        assert_eq!(flag_names(0), "none set");
        assert_eq!(flag_names(255).split(" + ").count(), 8);
    }
}
