//! Bounded directed TCP observations; missing and conflicting bytes stay explicit.
use crate::packets::PacketCapture;
use std::{collections::BTreeMap, net::SocketAddr};

/// Maximum directed epochs retained for analysis.
pub const FLOW_LIMIT: usize = 128;
/// Maximum sequence span of one directed epoch.
pub const STREAM_LIMIT: usize = 256 * 1024;
/// Maximum sum of retained sequence spans.
pub const TOTAL_STREAM_LIMIT: usize = 8 * 1024 * 1024;
/// Evidence available at a sequence position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteState {
    /// No observed payload byte supplies this position.
    Missing,
    /// All observed bytes at this position agree.
    Observed,
    /// Overlapping payload observations disagree.
    Conflict,
}
/// A directed epoch; opposite directions remain separate.
#[derive(Debug)]
pub struct Reassembled {
    /// Sender address and port.
    pub source: SocketAddr,
    /// Receiver address and port.
    pub destination: SocketAddr,
    /// Source record positions, in file order.
    pub packets: Vec<usize>,
    /// Sequence corresponding to position zero, including wrap-around.
    pub first_sequence: u32,
    /// Whether a SYN established the beginning of data sequence space.
    pub syn_observed: bool,
    /// Whether a FIN position is retained consistently.
    pub fin_observed: bool,
    /// Whether a reset was recorded.
    pub reset_observed: bool,
    /// Values are meaningful only where state is Observed.
    pub bytes: Vec<u8>,
    /// Presence/conflict information; no zero is substituted for a gap.
    pub states: Vec<ByteState>,
    /// Number of agreeing overlapped byte observations, not proof of network loss.
    pub repeated_bytes: usize,
    /// Count of input segments skipped for sequence budgets.
    pub skipped_segments: usize,
    /// Constraints and ambiguous end observations.
    pub notes: Vec<&'static str>,
}
impl Reassembled {
    /// The uniquely observed prefix; analysis never silently crosses a hole.
    pub fn contiguous_prefix(&self) -> &[u8] {
        let end = self
            .states
            .iter()
            .position(|s| *s != ByteState::Observed)
            .unwrap_or(self.states.len());
        &self.bytes[..end]
    }
    /// Whether the retained SYN-to-FIN span is unambiguous under this subset.
    pub fn observed_end(&self) -> bool {
        self.syn_observed
            && self.fin_observed
            && !self.reset_observed
            && self.skipped_segments == 0
            && self.notes.is_empty()
            && self.contiguous_prefix().len() == self.bytes.len()
    }
}
/// Bounded analysis with omissions visible.
#[derive(Debug)]
pub struct Reassembly {
    /// Directed epochs in first-observed order.
    pub flows: Vec<Reassembled>,
    /// TCP records excluded because the directed epoch budget was exhausted.
    pub unassigned_packets: usize,
}
struct Group {
    source: SocketAddr,
    destination: SocketAddr,
    anchor: u32,
    syn: bool,
    closed: bool,
    ids: Vec<usize>,
}
/// Group supported TCP records and preserve overlaps, gaps, wrap-around and cuts.
/// A changed SYN or a SYN after a close starts a new directed epoch.
pub fn reassemble(capture: &PacketCapture) -> Reassembly {
    let mut groups: Vec<Group> = vec![];
    let mut active = BTreeMap::new();
    let mut unassigned_packets = 0;
    for (id, packet) in capture.packets.iter().enumerate() {
        let Some(tcp) = &packet.tcp else { continue };
        let key = (tcp.source, tcp.destination);
        let syn = tcp.flags & 2 != 0;
        let sequence = tcp.sequence.wrapping_add(u32::from(syn));
        let existing = active.get(&key).copied();
        let new = existing.is_none_or(|i: usize| {
            syn && (!groups[i].syn || groups[i].anchor != sequence || groups[i].closed)
        });
        let index = if new {
            if groups.len() == FLOW_LIMIT {
                unassigned_packets += 1;
                active.remove(&key);
                continue;
            }
            let i = groups.len();
            groups.push(Group {
                source: tcp.source,
                destination: tcp.destination,
                anchor: sequence,
                syn,
                closed: false,
                ids: vec![],
            });
            active.insert(key, i);
            i
        } else if let Some(i) = existing {
            i
        } else {
            continue;
        };
        groups[index].ids.push(id);
        groups[index].closed |= tcp.flags & 5 != 0;
    }
    let mut total = 0;
    let mut flows = vec![];
    for group in groups {
        let mut minimum = 0i64;
        let mut maximum = 0i64;
        let mut ranges = vec![];
        let mut skipped = 0;
        let mut fin = None;
        let mut reset = false;
        let mut notes = vec![];
        for &id in &group.ids {
            let Some(tcp) = &capture.packets[id].tcp else {
                continue;
            };
            let sequence = tcp.sequence.wrapping_add(u32::from(tcp.flags & 2 != 0));
            let start = i64::from(sequence.wrapping_sub(group.anchor) as i32);
            // ACK-only packets do not reveal absent application byte contents.
            if tcp.declared_payload == 0 && tcp.flags & 5 == 0 {
                continue;
            }
            let Some(end) = i64::try_from(tcp.declared_payload)
                .ok()
                .and_then(|length| start.checked_add(length))
            else {
                skipped += 1;
                continue;
            };
            let low = minimum.min(start);
            let high = maximum.max(end);
            if (group.syn && start < 0)
                || high - low > STREAM_LIMIT as i64
                || total + (high - low) as usize > TOTAL_STREAM_LIMIT
            {
                skipped += 1;
                continue;
            }
            minimum = low;
            maximum = high;
            if tcp.flags & 1 != 0 {
                if fin.is_some_and(|old| old != end)
                    && !notes.contains(&"Conflicting FIN positions")
                {
                    notes.push("Conflicting FIN positions");
                }
                fin = Some(end);
            }
            reset |= tcp.flags & 4 != 0;
            ranges.push((id, start));
        }
        if fin.is_some_and(|end| end != maximum) {
            notes.push("Payload observations extend beyond recorded FIN");
        }
        let length = (maximum - minimum) as usize;
        total += length;
        let mut bytes = vec![0; length];
        let mut states = vec![ByteState::Missing; length];
        let mut repeated_bytes = 0;
        for (id, start) in ranges {
            let packet = &capture.packets[id];
            let Some(tcp) = &packet.tcp else { continue };
            if let Some(data) = packet.bytes.get(tcp.payload.clone()) {
                let begin = (start - minimum) as usize;
                for (position, byte) in data.iter().enumerate() {
                    let target = begin + position;
                    let Some(state) = states.get_mut(target) else {
                        break;
                    };
                    match *state {
                        ByteState::Missing => {
                            bytes[target] = *byte;
                            *state = ByteState::Observed;
                        }
                        ByteState::Observed if bytes[target] == *byte => repeated_bytes += 1,
                        ByteState::Observed => *state = ByteState::Conflict,
                        ByteState::Conflict => {}
                    }
                }
            }
        }
        flows.push(Reassembled {
            source: group.source,
            destination: group.destination,
            packets: group.ids,
            first_sequence: group.anchor.wrapping_add(minimum as u32),
            syn_observed: group.syn,
            fin_observed: fin.is_some(),
            reset_observed: reset,
            bytes,
            states,
            repeated_bytes,
            skipped_segments: skipped,
            notes,
        });
    }
    Reassembly {
        flows,
        unassigned_packets,
    }
}
