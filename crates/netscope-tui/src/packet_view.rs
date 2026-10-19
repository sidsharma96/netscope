//! Offline packet/stream navigation and safe rendering, with no replay action.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use netscope_core::{
    packets::PacketCapture,
    reassembly::{ByteState, Reassembly, reassemble},
    terminal_text,
    wire_http::inspect_http,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

#[derive(Debug, Clone, Copy)]
enum Screen {
    Packets,
    Packet(usize),
    Streams,
    Stream(usize),
}
/// Packet navigation holds derived state while the recording remains borrowed.
#[derive(Debug)]
pub struct PacketApp {
    screen: Screen,
    selected: usize,
    packet_selected: usize,
    stream_selected: usize,
    offset: usize,
    analysis: Reassembly,
    help: bool,
    quit: bool,
    page: usize,
    small: bool,
    recorded: bool,
    learning: crate::packet_learning::Learning,
    stream: crate::stream_view::StreamInspector,
}
impl PacketApp {
    /// Derive bounded stream observations once without performing network I/O.
    pub fn new(capture: &PacketCapture) -> Self {
        Self {
            screen: Screen::Packets,
            selected: 0,
            packet_selected: 0,
            stream_selected: 0,
            offset: 0,
            analysis: reassemble(capture),
            help: false,
            quit: false,
            page: 16,
            small: false,
            recorded: false,
            learning: crate::packet_learning::Learning::default(),
            stream: crate::stream_view::StreamInspector::default(),
        }
    }
    /// Whether an explicit quit key was accepted.
    pub fn quit(&self) -> bool {
        self.quit
    }
    /// Apply one input while preserving list positions when inspectors close.
    pub fn key(&mut self, key: KeyEvent, capture: &PacketCapture) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Char('q')
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            self.quit = true;
            return;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        if self.small {
            return;
        }
        if key.code == KeyCode::Char('?') {
            self.help = !self.help;
            return;
        }
        if key.code == KeyCode::Esc {
            if self.help {
                self.help = false;
                return;
            }
            self.screen = match self.screen {
                Screen::Packet(_) => Screen::Packets,
                Screen::Stream(_) => Screen::Streams,
                _ => Screen::Packets,
            };
            self.selected = match self.screen {
                Screen::Streams => self.stream_selected,
                _ => self.packet_selected,
            };
            self.offset = 0;
            return;
        }
        if self.help || self.small {
            return;
        }
        if key.code == KeyCode::Char('s') {
            self.screen = Screen::Streams;
            self.selected = self.stream_selected;
            self.offset = 0;
            return;
        }
        if key.code == KeyCode::Char('p') {
            self.screen = Screen::Packets;
            self.selected = self.packet_selected;
            self.offset = 0;
            return;
        }
        if matches!(self.screen, Screen::Packet(_) | Screen::Stream(_)) {
            match key.code {
                KeyCode::Char('1') => {
                    self.recorded = false;
                    return;
                }
                KeyCode::Char('2') => {
                    self.recorded = true;
                    self.offset = 0;
                    return;
                }
                _ if !self.recorded => {
                    if matches!(self.screen, Screen::Stream(_)) {
                        self.stream.key(key.code);
                    } else {
                        self.learning.key(key.code);
                    }
                    return;
                }
                _ => {}
            }
        }
        let count = match self.screen {
            Screen::Packets => capture.packets.len(),
            Screen::Streams => self.analysis.flows.len(),
            _ => 0,
        };
        let movement = match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some((false, 1)),
            KeyCode::Down | KeyCode::Char('j') => Some((true, 1)),
            KeyCode::PageUp => Some((false, self.page)),
            KeyCode::PageDown => Some((true, self.page)),
            _ => None,
        };
        if let Some((forward, amount)) = movement {
            if matches!(self.screen, Screen::Packets | Screen::Streams) {
                self.selected = if forward {
                    self.selected
                        .saturating_add(amount)
                        .min(count.saturating_sub(1))
                } else {
                    self.selected.saturating_sub(amount)
                };
            } else {
                self.offset = if forward {
                    self.offset.saturating_add(amount)
                } else {
                    self.offset.saturating_sub(amount)
                };
            }
        }
        match key.code {
            KeyCode::Home => {
                if matches!(self.screen, Screen::Packets | Screen::Streams) {
                    self.selected = 0;
                }
                self.offset = 0;
            }
            KeyCode::End => {
                if count > 0 {
                    self.selected = count - 1;
                } else {
                    self.offset = usize::MAX;
                }
            }
            KeyCode::Enter if count > 0 => {
                self.recorded = false;
                self.learning = crate::packet_learning::Learning::default();
                self.screen = match self.screen {
                    Screen::Packets => Screen::Packet(self.selected),
                    Screen::Streams => Screen::Stream(self.selected),
                    other => other,
                };
                if let Screen::Stream(id) = self.screen
                    && let Some(flow) = self.analysis.flows.get(id)
                {
                    self.stream = crate::stream_view::StreamInspector::new(flow);
                }
                self.offset = 0;
            }
            _ => {}
        }
        match self.screen {
            Screen::Packets | Screen::Packet(_) => self.packet_selected = self.selected,
            Screen::Streams | Screen::Stream(_) => self.stream_selected = self.selected,
        }
    }
}
fn preview(bytes: &[u8], limit: usize) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            let mut end = text.len().min(limit);
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            let mut value = terminal_text(&text[..end]);
            if end < text.len() {
                value.push_str(" … [display shortened]");
            }
            value
        }
        Err(_) => {
            let mut value = format!("[non-UTF-8 · hex] {}", hex(bytes));
            if bytes.len() > 256 {
                value.push_str(" … [display shortened]");
            }
            value
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(256)
        .map(|n| format!("{n:02x} "))
        .collect()
}
fn packet_lines(capture: &PacketCapture, id: usize) -> Vec<String> {
    let Some(packet) = capture.packets.get(id) else {
        return vec!["Packet ID unavailable".into()];
    };
    // Print only the digits the file recorded; microsecond files hold no nanoseconds.
    let fraction = if capture.nanoseconds {
        format!("{:09}", packet.nanos)
    } else {
        format!("{:06}", packet.nanos / 1000)
    };
    let mut lines = vec![
        format!(
            "Packet #{id} · recorded timestamp {}.{fraction}",
            packet.seconds
        ),
        format!(
            "Retained {} bytes · original {} bytes",
            packet.bytes.len(),
            packet.original_len
        ),
        "File record order is not a receiver transcript".into(),
    ];
    lines.extend(packet.layers.iter().map(|s| terminal_text(s)));
    if let Some(tcp) = &packet.tcp {
        lines.push(format!(
            "Sequence {} · ACK {} · window {} (unscaled)",
            tcp.sequence, tcp.acknowledgement, tcp.window
        ));
        lines.push(format!(
            "TCP payload range {:?} · {} retained / {} declared bytes",
            tcp.payload,
            tcp.payload.len(),
            tcp.declared_payload
        ));
    }
    lines.extend(
        netscope_core::datagrams::inspect_datagram(packet)
            .iter()
            .map(|s| terminal_text(s)),
    );
    lines.push("Stored packet hex · first 256 bytes; not decrypted".into());
    lines.push(hex(&packet.bytes));
    if packet.bytes.len() > 256 {
        lines.push("[hex display shortened; stored bytes unchanged]".into());
    }
    lines
}
fn stream_lines(analysis: &Reassembly, id: usize) -> Vec<String> {
    let Some(flow) = analysis.flows.get(id) else {
        return vec!["Directed stream ID unavailable".into()];
    };
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
    let mut lines = vec![
        format!(
            "Directed epoch #{id} · {} → {}",
            flow.source, flow.destination
        ),
        format!(
            "{} source records · first sequence {}",
            flow.packets.len(),
            flow.first_sequence
        ),
        format!(
            "SYN {} · FIN {} · reset {}",
            flow.syn_observed, flow.fin_observed, flow.reset_observed
        ),
        format!(
            "{} sequence positions · {missing} missing · {conflicts} conflicting",
            flow.states.len()
        ),
        format!(
            "{} identical overlapping byte observations",
            flow.repeated_bytes
        ),
        "Repeated observations are retransmission candidates or capture duplicates".into(),
        format!(
            "{} segments excluded by sequence/memory limits",
            flow.skipped_segments
        ),
        "Checksums, receiver acceptance and opposite-direction pairing unverified".into(),
    ];
    if !flow.syn_observed {
        lines.push("Start unknown: capture has no SYN for this directed epoch".into());
    }
    if flow.ignored_probes > 0 {
        lines.push(format!(
            "{} one-byte keep-alive probes ignored at the last observed byte",
            flow.ignored_probes
        ));
    }
    lines.extend(flow.notes.iter().map(|s| s.to_string()));
    lines.push("Sequence ranges · offsets from first retained sequence".into());
    let mut start = 0;
    let mut runs = 0;
    while start < flow.states.len() && runs < 64 {
        let state = flow.states[start];
        let end = flow.states[start..]
            .iter()
            .position(|s| *s != state)
            .map_or(flow.states.len(), |n| start + n);
        lines.push(format!("  [{start}..{end}) {state:?}"));
        start = end;
        runs += 1;
    }
    if start < flow.states.len() {
        lines.push("[range display shortened after 64 runs]".into());
    }
    let prefix = flow.contiguous_prefix();
    lines.push(format!(
        "HTTP/1.1 framing candidates · {} uniquely observed prefix bytes",
        prefix.len()
    ));
    lines.push("Response request method is not paired; framing is conditional".into());
    let http = inspect_http(prefix, flow.observed_end());
    for message in http.messages {
        lines.push(format!(
            "At {}: {}",
            message.start,
            terminal_text(&message.start_line)
        ));
        lines.push(message.framing.into());
        lines.push(format!(
            "Examined end {} · framing terminator present {}",
            message.end, message.framed
        ));
        for (name, value) in &message.headers {
            lines.push(format!("  {name}: {}", preview(value, 512)));
        }
        for chunk in message.chunks.iter().take(64) {
            lines.push(format!(
                "  Chunk line {:?}: {} · data {:?}",
                chunk.line,
                preview(&prefix[chunk.line.clone()], 256),
                chunk.data
            ));
        }
        if message.chunks.len() > 64 {
            lines.push("[chunk display shortened after 64 entries]".into());
        }
        for (name, value) in message.trailers {
            lines.push(format!("  Trailer {name}: {}", preview(&value, 512)));
        }
        if let Some(sdp) = message.sdp {
            lines.push(sdp.into());
        }
        lines.push(
            "Application byte preview · framing removed, content encoding not decoded".into(),
        );
        lines.push(preview(&message.body, 4096));
    }
    if let Some(reason) = http.stopped {
        lines.push(format!("HTTP inspection stopped: {reason}"));
    }
    lines.push("Other/encrypted application protocols remain raw bytes".into());
    lines.push(format!("Prefix hex: {}", hex(prefix)));
    lines
}
/// Draw with the same small-terminal recovery boundary as HAR inspection.
pub fn draw_packets(
    frame: &mut Frame,
    capture: &PacketCapture,
    app: &mut PacketApp,
    filename: &str,
) {
    let area = frame.area();
    frame.render_widget(Block::default().style(crate::presentation::base()), area);
    app.small = area.width < 80 || area.height < 24;
    if app.small {
        frame.render_widget(
            Paragraph::new("NETSCOPE · resize to at least 80×24 · q / Ctrl+c quit"),
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
    let learning_area = body;
    let [guide, body] = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).areas(body);
    app.page = usize::from(body.height.saturating_sub(2)).max(1);
    let (question, explanation) = match app.screen {
        Screen::Packets | Screen::Packet(_) => (
            "What travelled between these addresses?",
            "A packet is one recorded piece of network traffic. Enter shows its layers.",
        ),
        Screen::Streams | Screen::Stream(_) => (
            "What bytes can be put back in order?",
            "A stream here groups TCP bytes in one direction and connection lifetime.",
        ),
    };
    let direction = match app.screen {
        Screen::Packets | Screen::Packet(_) => capture.packets.get(app.selected).and_then(|p| {
            p.tcp
                .as_ref()
                .map(|t| format!("{} ── TCP → {}", t.source, t.destination))
                .or_else(|| {
                    p.udp
                        .as_ref()
                        .map(|u| format!("{} ── UDP → {}", u.source, u.destination))
                })
        }),
        Screen::Streams | Screen::Stream(_) => app
            .analysis
            .flows
            .get(app.selected)
            .map(|f| format!("{} ── recorded direction → {}", f.source, f.destination)),
    };
    let guide_lines = vec![
        ratatui::text::Line::styled(
            question,
            ratatui::style::Style::default().fg(crate::presentation::MINT),
        ),
        ratatui::text::Line::from(explanation),
        ratatui::text::Line::styled(
            crate::view::clip(
                &direction.unwrap_or_else(|| "No transport addresses available".into()),
                usize::from(guide.width),
            ),
            ratatui::style::Style::default().fg(crate::presentation::MINT),
        ),
        ratatui::text::Line::styled(
            "Recorded direction only · gaps and encryption can limit what is readable",
            ratatui::style::Style::default().fg(crate::presentation::MUTED),
        ),
    ];
    frame.render_widget(Paragraph::new(guide_lines), guide);
    let title = match app.screen {
        Screen::Packets => "PACKETS",
        Screen::Packet(_) => "PACKET LAYERS",
        Screen::Streams => "TCP STREAMS",
        Screen::Stream(_) => "STREAM EVIDENCE",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "NETSCOPE / {title}\n{}\n{} packets · link {} · {}",
            terminal_text(filename),
            capture.packets.len(),
            capture.link_type,
            // A file-level note, such as unread records, outranks the reminder.
            capture
                .notes
                .first()
                .copied()
                .unwrap_or("offline; capture unchanged")
        )),
        header,
    );
    if !app.help && !app.recorded && matches!(app.screen, Screen::Packet(_) | Screen::Stream(_)) {
        frame.render_widget(ratatui::widgets::Clear, learning_area);
        frame.render_widget(
            Block::default().style(crate::presentation::base()),
            learning_area,
        );
        match app.screen {
            Screen::Packet(id) => {
                if let Some(packet) = capture.packets.get(id) {
                    app.learning.packet(frame, learning_area, packet, id);
                }
            }
            Screen::Stream(id) => {
                if let Some(flow) = app.analysis.flows.get(id) {
                    app.stream.draw(frame, learning_area, flow, id);
                }
            }
            _ => {}
        }
        frame.render_widget(
            Paragraph::new(if matches!(app.screen, Screen::Stream(_)) {
                "↑↓ range · b text/hex · h help · [ ] bytes · 2 report · Esc list · q quit"
            } else {
                "↑↓ topic · PgUp/PgDn text · 2 recorded report · Esc list · ? help · q quit"
            }),
            footer,
        );
        return;
    }
    let block = crate::workspace_view::block(
        match app.screen {
            Screen::Packets => " Packets · ↑↓ select · Enter inspect ",
            Screen::Streams => " One-way streams · ↑↓ select · Enter inspect ",
            Screen::Packet(_) => " Packet layers · 1 Explain · [2 Recorded] · ↑↓ scroll ",
            Screen::Stream(_) => " Reassembled evidence · 1 Inspect · [2 Recorded] · ↑↓ scroll ",
        }
        .into(),
        true,
    );
    let inner = block.inner(body);
    frame.render_widget(block, body);
    let lines = if app.help {
        vec![
            "↑/↓ or j/k move · PageUp/PageDown page · Home/End bounds".into(),
            "Enter inspects · Esc returns and preserves list selection".into(),
            "p packets · s directed TCP epochs · ? help · q / Ctrl+c quit".into(),
            "Inspectors: 1 Inspect/Explain · 2 Recorded report · Esc returns".into(),
            "Packet: ↑↓ topics. Stream: ↑↓ ranges, Home/End first/last".into(),
            "Stream: b text/hex, [ ] bytes, h guidance; PgUp/PgDn pages".into(),
            "PCAP clocks are recorded metadata, not guaranteed event-delivery times".into(),
            "Gaps/conflicts stop HTTP prefix inspection; encryption stays unavailable".into(),
        ]
    } else {
        match app.screen {
            Screen::Packets => {
                let top = app.selected.saturating_sub(app.page - 1);
                if capture.packets.is_empty() {
                    vec!["No packet records".into()]
                } else {
                    capture
                        .packets
                        .iter()
                        .enumerate()
                        .skip(top)
                        .take(app.page)
                        .map(|(i, p)| {
                            format!(
                                "{}#{i} · {}/{} bytes · {}",
                                if i == app.selected { "> " } else { "  " },
                                p.bytes.len(),
                                p.original_len,
                                if let Some(tcp) = &p.tcp {
                                    format!(
                                        "TCP {} → {} · flags 0x{:02x}",
                                        tcp.source, tcp.destination, tcp.flags
                                    )
                                } else if let Some(udp) = &p.udp {
                                    format!("UDP {} → {}", udp.source, udp.destination)
                                } else {
                                    terminal_text(
                                        p.layers.last().map_or("No parsed layer", String::as_str),
                                    )
                                }
                            )
                        })
                        .collect()
                }
            }
            Screen::Streams => {
                let top = app.selected.saturating_sub(app.page.saturating_sub(2));
                let mut lines = vec![format!(
                    "{} directed epochs · {} unassigned TCP records",
                    app.analysis.flows.len(),
                    app.analysis.unassigned_packets
                )];
                lines.extend(
                    app.analysis
                        .flows
                        .iter()
                        .enumerate()
                        .skip(top)
                        .take(app.page.saturating_sub(1))
                        .map(|(i, f)| {
                            format!(
                                "{}#{i} {} → {} · {} positions",
                                if i == app.selected { "> " } else { "  " },
                                f.source,
                                f.destination,
                                f.states.len()
                            )
                        }),
                );
                lines
            }
            Screen::Packet(id) => packet_lines(capture, id),
            Screen::Stream(id) => stream_lines(&app.analysis, id),
        }
    };
    let list = matches!(app.screen, Screen::Packets | Screen::Streams);
    let lines: Vec<_> = if list {
        lines
            .iter()
            .map(|line| crate::view::clip(line, usize::from(inner.width)))
            .collect()
    } else {
        lines
    };
    let wrapped: Vec<_> = lines
        .iter()
        .flat_map(|s| crate::view::wrap(s, usize::from(inner.width)))
        .collect();
    let inspector = matches!(app.screen, Screen::Packet(_) | Screen::Stream(_));
    if inspector {
        app.offset = app
            .offset
            .min(wrapped.len().saturating_sub(usize::from(inner.height)));
    } else {
        app.offset = 0;
    }
    frame.render_widget(
        Paragraph::new(
            wrapped
                .into_iter()
                .skip(app.offset)
                .take(usize::from(inner.height))
                .map(crate::presentation::report_line)
                .collect::<Vec<_>>(),
        ),
        inner,
    );
    frame.render_widget(
        Paragraph::new(
            "↑/↓ move · Enter inspect · Esc back · p packets · s streams · ? help · q quit",
        ),
        footer,
    );
}
/// Plain-text summary used when output is redirected.
pub fn packet_summary(capture: &PacketCapture) -> String {
    let analysis = reassemble(capture);
    let mut lines = vec![
        format!(
            "{} packets · link {} · snaplen {} · timestamp {}",
            capture.packets.len(),
            capture.link_type,
            capture.snaplen,
            if capture.nanoseconds {
                "nanoseconds"
            } else {
                "microseconds"
            }
        ),
        "Offline packet evidence; checksums and receiver acceptance not verified".into(),
    ];
    lines.extend(capture.notes.iter().map(|s| s.to_string()));
    for id in 0..capture.packets.len() {
        lines.extend(packet_lines(capture, id));
    }
    lines.push(format!(
        "{} directed TCP epochs · {} unassigned records",
        analysis.flows.len(),
        analysis.unassigned_packets
    ));
    for id in 0..analysis.flows.len() {
        lines.extend(stream_lines(&analysis, id));
    }
    lines.join("\n")
}
