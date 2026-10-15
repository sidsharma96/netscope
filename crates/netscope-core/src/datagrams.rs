//! Bounded recognition of recorded UDP DNS answers and STUN-shaped messages.
use crate::packets::Packet;
use std::net::{Ipv4Addr, Ipv6Addr};
fn number(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}
fn name(bytes: &[u8], at: usize) -> Result<(String, usize), &'static str> {
    let mut position = at;
    let mut consumed = None;
    let mut labels = vec![];
    let mut size = 0;
    let mut jumps = 0;
    // Labels are bounded by the 255-byte name size (at most 127 of them), so only
    // pointer jumps need their own count. Together the two limits end every loop.
    loop {
        let first = *bytes.get(position).ok_or("DNS name cut")?;
        if first == 0 {
            return Ok((labels.join("."), consumed.unwrap_or(position + 1)));
        }
        if first & 0xc0 == 0xc0 {
            let target = usize::from(number(bytes, position).ok_or("DNS pointer cut")? & 0x3fff);
            if target >= position {
                return Err("DNS pointer does not reference earlier data");
            }
            jumps += 1;
            if jumps > 127 {
                return Err("DNS name traversal limit");
            }
            consumed.get_or_insert(position + 2);
            position = target;
            continue;
        }
        if first & 0xc0 != 0 {
            return Err("DNS label type unsupported");
        }
        let end = position + 1 + usize::from(first);
        let label = bytes.get(position + 1..end).ok_or("DNS label cut")?;
        size += label.len() + 1;
        if size > 255 {
            return Err("DNS name exceeds bounded label size");
        }
        labels.push(
            label
                .iter()
                .map(|byte| {
                    if byte.is_ascii_graphic() && !matches!(byte, b'.' | b'\\') {
                        char::from(*byte).to_string()
                    } else {
                        format!("\\{byte:03}")
                    }
                })
                .collect::<String>(),
        );
        position = end;
    }
}
fn dns(bytes: &[u8]) -> Result<Vec<String>, &'static str> {
    if bytes.len() < 12 {
        return Err("DNS header cut");
    }
    let flags = number(bytes, 2).ok_or("DNS flags unavailable")?;
    let questions = number(bytes, 4).ok_or("DNS question count unavailable")?;
    let answers = number(bytes, 6).ok_or("DNS answer count unavailable")?;
    if questions > 16 || answers > 32 {
        return Err("DNS question/answer inspection limit");
    }
    if flags & 0x7800 != 0 {
        return Err("DNS opcode outside ordinary query subset");
    }
    let mut lines = vec![format!(
        "DNS candidate · {} · ID {} · rcode {} · truncated flag {}",
        if flags & 0x8000 != 0 {
            "response"
        } else {
            "query"
        },
        number(bytes, 0).ok_or("DNS ID unavailable")?,
        flags & 15,
        flags & 0x200 != 0
    )];
    let mut at = 12;
    for _ in 0..questions {
        let (value, next) = name(bytes, at)?;
        let kind = number(bytes, next).ok_or("DNS question cut")?;
        number(bytes, next + 2).ok_or("DNS question class cut")?;
        at = next + 4;
        lines.push(format!("Question {value} · type {kind}"));
    }
    for _ in 0..answers {
        let (owner, next) = name(bytes, at)?;
        let kind = number(bytes, next).ok_or("DNS answer type cut")?;
        let class = number(bytes, next + 2).ok_or("DNS answer class cut")?;
        let length = usize::from(number(bytes, next + 8).ok_or("DNS answer header cut")?);
        let start = next + 10;
        let data = bytes
            .get(start..start + length)
            .ok_or("DNS answer data cut")?;
        let value = if class == 1 && kind == 1 && length == 4 {
            Ipv4Addr::new(data[0], data[1], data[2], data[3]).to_string()
        } else if class == 1 && kind == 28 && length == 16 {
            Ipv6Addr::from(<[u8; 16]>::try_from(data).map_err(|_| "DNS IPv6 answer size")?)
                .to_string()
        } else {
            format!("type {kind}, class {class}, {length} raw bytes")
        };
        lines.push(format!("Answer {owner} → {value}"));
        at = start + length;
    }
    lines.push("Answer fields only · not proof of resolver choice or load balancing".into());
    Ok(lines)
}
/// Return scoped protocol candidates; no encrypted media or successful ICE is inferred.
pub fn inspect_datagram(packet: &Packet) -> Vec<String> {
    let Some(udp) = &packet.udp else {
        return vec![];
    };
    if !udp.complete {
        return vec!["UDP application inspection unavailable: datagram cut".into()];
    }
    let Some(bytes) = packet.bytes.get(udp.payload.clone()) else {
        return vec![];
    };
    if udp.source.port() == 53 || udp.destination.port() == 53 {
        return dns(bytes)
            .unwrap_or_else(|reason| vec![format!("DNS candidate unavailable: {reason}")]);
    }
    if bytes.len() >= 20
        && bytes[0] & 0xc0 == 0
        && bytes.get(4..8) == Some(&[0x21, 0x12, 0xa4, 0x42])
    {
        let length = usize::from(number(bytes, 2).unwrap_or_default());
        if length % 4 != 0 || length + 20 != bytes.len() {
            return vec!["STUN-shaped header with invalid/incomplete declared length".into()];
        }
        let mut at = 20;
        let mut count = 0;
        while at < bytes.len() {
            let Some(size) = number(bytes, at + 2) else {
                return vec!["STUN attribute header cut".into()];
            };
            at += 4 + usize::from(size).div_ceil(4) * 4;
            count += 1;
            if at > bytes.len() || count > 64 {
                return vec!["STUN attribute cut or inspection limit".into()];
            }
        }
        return vec![
            format!(
                "STUN candidate · type 0x{:04x} · {count} framed attributes",
                number(bytes, 0).unwrap_or_default()
            ),
            "Header/attribute framing only · not proof of ICE success or visible media".into(),
        ];
    }
    vec!["UDP application protocol outside DNS/STUN recognition subset".into()]
}
