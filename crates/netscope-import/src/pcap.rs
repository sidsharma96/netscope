//! Classic PCAP framing and a bounded Ethernet, loopback, Linux cooked and raw-IP layer subset.
use netscope_core::packets::{Packet, PacketCapture, TcpSegment, UdpDatagram};
use std::{
    fmt,
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
};

/// Maximum accepted file bytes, before allocation grows beyond the budget.
pub const PCAP_FILE_LIMIT: usize = 32 * 1024 * 1024;
/// Maximum records accepted from one file.
pub const PCAP_PACKET_LIMIT: usize = 20_000;
/// Maximum snap length and stored bytes per record.
pub const PCAP_RECORD_LIMIT: usize = 256 * 1024;
/// A framing failure with a byte offset and no captured values in its reason.
#[derive(Debug)]
pub struct PcapError {
    /// Offset in the file where validation failed.
    pub offset: usize,
    /// Fixed explanation of the unsupported or malformed structure.
    pub reason: &'static str,
}
impl fmt::Display for PcapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PCAP byte {}: {}", self.offset, self.reason)
    }
}
impl std::error::Error for PcapError {}
fn error(offset: usize, reason: &'static str) -> PcapError {
    PcapError { offset, reason }
}
fn be16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn be32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn word(bytes: &[u8], offset: usize, little: bool) -> Result<u32, PcapError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| error(offset, "Truncated numeric field"))?
        .try_into()
        .map_err(|_| error(offset, "Truncated numeric field"))?;
    Ok(if little {
        u32::from_le_bytes(value)
    } else {
        u32::from_be_bytes(value)
    })
}
/// Load with a byte cap; opening a packet file performs no network I/O.
/// # Errors
/// Rejects file I/O, excessive input and malformed PCAP framing.
pub fn import_pcap_file(path: &Path) -> Result<PacketCapture, PcapError> {
    let file = std::fs::File::open(path).map_err(|_| error(0, "Could not open file"))?;
    let mut bytes = Vec::new();
    file.take((PCAP_FILE_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| error(0, "Could not read file"))?;
    import_pcap(&bytes)
}
/// Parse classic PCAP 2.4 in either byte order and either timestamp resolution.
/// # Errors
/// Rejects malformed container framing; unsupported packet layers remain inspectable.
pub fn import_pcap(bytes: &[u8]) -> Result<PacketCapture, PcapError> {
    if bytes.len() > PCAP_FILE_LIMIT {
        return Err(error(0, "File exceeds 32 MiB"));
    }
    let (little, nanoseconds) = match bytes.get(..4) {
        Some([0xd4, 0xc3, 0xb2, 0xa1]) => (true, false),
        Some([0xa1, 0xb2, 0xc3, 0xd4]) => (false, false),
        Some([0x4d, 0x3c, 0xb2, 0xa1]) => (true, true),
        Some([0xa1, 0xb2, 0x3c, 0x4d]) => (false, true),
        Some([0x0a, 0x0d, 0x0d, 0x0a]) => {
            return Err(error(
                0,
                "PCAPNG is outside the supported classic PCAP subset",
            ));
        }
        _ => return Err(error(0, "Expected classic PCAP magic")),
    };
    if bytes.len() < 24 {
        return Err(error(0, "Truncated global header"));
    }
    if word(bytes, 4, little)? != if little { 0x0004_0002 } else { 0x0002_0004 } {
        return Err(error(4, "Expected PCAP version 2.4"));
    }
    let snaplen = word(bytes, 16, little)?;
    if snaplen == 0 || snaplen as usize > PCAP_RECORD_LIMIT {
        return Err(error(16, "Snap length must be 1 through 256 KiB"));
    }
    let link_type = word(bytes, 20, little)?;
    if link_type >> 16 != 0 {
        return Err(error(
            20,
            "Additional link/FCS information is outside this subset",
        ));
    }
    let mut capture = PacketCapture {
        link_type,
        snaplen,
        nanoseconds,
        packets: vec![],
        notes: vec![],
    };
    let mut offset = 24;
    while offset < bytes.len() {
        if capture.packets.len() == PCAP_PACKET_LIMIT {
            capture
                .notes
                .push("More than 20000 packet records; only the first 20000 were read");
            break;
        }
        // Stopping a capture abruptly can leave a half-written last record.
        let truncated = |capture: &mut PacketCapture, at, reason| {
            if capture.packets.is_empty() {
                return Err(error(at, reason));
            }
            capture
                .notes
                .push("Final packet record truncated; earlier records kept");
            Ok(())
        };
        if bytes.len() - offset < 16 {
            truncated(&mut capture, offset, "Truncated record header")?;
            break;
        }
        let seconds = word(bytes, offset, little)?;
        let fraction = word(bytes, offset + 4, little)?;
        if fraction
            >= if nanoseconds {
                1_000_000_000
            } else {
                1_000_000
            }
        {
            return Err(error(
                offset + 4,
                "Timestamp fraction outside its resolution",
            ));
        }
        let retained = word(bytes, offset + 8, little)? as usize;
        let original_len = word(bytes, offset + 12, little)?;
        if retained > snaplen as usize || retained > original_len as usize {
            return Err(error(
                offset + 8,
                "Captured length exceeds snap length or original length",
            ));
        }
        let start = offset + 16;
        let Some(end) = start
            .checked_add(retained)
            .filter(|end| *end <= bytes.len())
        else {
            truncated(&mut capture, start, "Truncated packet record")?;
            break;
        };
        let mut packet = Packet {
            seconds,
            nanos: if nanoseconds {
                fraction
            } else {
                fraction * 1000
            },
            original_len,
            bytes: bytes[start..end].to_vec(),
            layers: vec![],
            tcp: None,
            udp: None,
        };
        if retained < original_len as usize {
            packet
                .layers
                .push("Capture cut: fewer bytes retained than original packet".into());
        }
        match layers(&mut packet, link_type) {
            Ok(()) => {}
            Err(reason) => packet.layers.push(reason.into()),
        }
        capture.packets.push(packet);
        offset = end;
    }
    Ok(capture)
}
fn layers(packet: &mut Packet, link: u32) -> Result<(), &'static str> {
    let b = &packet.bytes;
    let (mut offset, mut kind) = match link {
        1 => {
            if b.len() < 14 {
                return Err("Ethernet header unavailable: fewer than 14 bytes");
            }
            let mac = |part: &[u8]| {
                part.iter()
                    .map(|n| format!("{n:02x}"))
                    .collect::<Vec<_>>()
                    .join(":")
            };
            packet
                .layers
                .push(format!("Ethernet {} → {}", mac(&b[6..12]), mac(&b[..6])));
            (14, be16(b, 12).ok_or("Ethernet type unavailable")?)
        }
        101 => (
            0,
            match b.first().map(|n| n >> 4) {
                Some(4) => 0x0800,
                Some(6) => 0x86dd,
                _ => return Err("Raw IP version unsupported or unavailable"),
            },
        ),
        // BSD loopback (DLT_NULL): a 4-byte address family in the capturing host's byte order.
        0 => {
            let family = b.get(..4).ok_or("Loopback header unavailable")?;
            let value = if family[..2] == [0, 0] {
                u32::from_be_bytes([family[0], family[1], family[2], family[3]])
            } else {
                u32::from_le_bytes([family[0], family[1], family[2], family[3]])
            };
            packet
                .layers
                .push(format!("Loopback address family {value}"));
            let kind = match value {
                2 => 0x0800,
                // AF_INET6 differs by system: Linux, NetBSD/OpenBSD, FreeBSD, macOS.
                10 | 24 | 28 | 30 => 0x86dd,
                _ => return Err("Loopback address family unsupported; bytes retained"),
            };
            (4, kind)
        }
        // Linux cooked capture v1: 16 bytes, EtherType last.
        113 => {
            let kind = be16(b, 14).ok_or("Linux cooked (SLL) header unavailable")?;
            packet.layers.push("Linux cooked capture (SLL)".into());
            (16, kind)
        }
        // Linux cooked capture v2: 20 bytes, EtherType first.
        276 => {
            if b.len() < 20 {
                return Err("Linux cooked (SLL2) header unavailable");
            }
            let kind = be16(b, 0).ok_or("Linux cooked (SLL2) header unavailable")?;
            packet.layers.push(format!(
                "Linux cooked capture (SLL2) · interface {}",
                be32(b, 4).ok_or("Linux cooked (SLL2) header unavailable")?
            ));
            (20, kind)
        }
        228 => (0, 0x0800),
        229 => (0, 0x86dd),
        _ => return Err("Link type unsupported; packet bytes retained"),
    };
    for _ in 0..2 {
        if !matches!(kind, 0x8100 | 0x88a8) {
            break;
        }
        let tci = be16(b, offset).ok_or("Truncated VLAN tag")?;
        packet.layers.push(format!("VLAN ID {}", tci & 0x0fff));
        kind = be16(b, offset + 2).ok_or("Truncated VLAN type")?;
        offset += 4;
    }
    let (source, destination, protocol, payload, end, declared_end) = match kind {
        0x0800 => {
            let h = b
                .get(offset..offset + 20)
                .ok_or("IPv4 base header unavailable")?;
            let length = usize::from(h[0] & 15) * 4;
            let mut total = usize::from(be16(h, 2).ok_or("IPv4 length unavailable")?);
            // Segmentation offload can record a total length of 0 for a segment the
            // card has not split yet; the frame's original length is then the evidence.
            let frame = packet.original_len as usize;
            let offload = total == 0 && frame > offset + length;
            if offload {
                total = frame - offset;
            }
            if h[0] >> 4 != 4 || length < 20 || total < length {
                return Err("Invalid IPv4 version or lengths");
            }
            if b.get(offset..offset + length).is_none() {
                return Err("IPv4 options cut by capture");
            }
            let src = IpAddr::V4(Ipv4Addr::new(h[12], h[13], h[14], h[15]));
            let dst = IpAddr::V4(Ipv4Addr::new(h[16], h[17], h[18], h[19]));
            packet.layers.push(format!(
                "IPv4 {src} → {dst} · {length}-byte header · TTL {}",
                h[8]
            ));
            if offload {
                packet.layers.push(
                    "IPv4 total length 0 (segmentation offload capture); frame length used".into(),
                );
            }
            let fragment = be16(h, 6).ok_or("IPv4 fragment field unavailable")?;
            if fragment & 0x3fff != 0 {
                return Err("IPv4 fragmented payload: transport/reassembly unavailable");
            }
            (
                src,
                dst,
                h[9],
                offset + length,
                b.len().min(offset + total),
                offset + total,
            )
        }
        0x86dd => {
            let h = b
                .get(offset..offset + 40)
                .ok_or("IPv6 base header unavailable")?;
            if h[0] >> 4 != 6 {
                return Err("Invalid IPv6 version");
            }
            let length = usize::from(be16(h, 4).ok_or("IPv6 length unavailable")?);
            let src = IpAddr::V6(Ipv6Addr::from(
                <[u8; 16]>::try_from(&h[8..24]).map_err(|_| "IPv6 address unavailable")?,
            ));
            let dst = IpAddr::V6(Ipv6Addr::from(
                <[u8; 16]>::try_from(&h[24..40]).map_err(|_| "IPv6 address unavailable")?,
            ));
            packet
                .layers
                .push(format!("IPv6 {src} → {dst} · hop limit {}", h[7]));
            if length == 0 {
                return Err("IPv6 empty/jumbo payload outside transport subset");
            }
            (
                src,
                dst,
                h[6],
                offset + 40,
                b.len().min(offset + 40 + length),
                offset + 40 + length,
            )
        }
        _ => return Err("Ethernet payload type unsupported; bytes retained"),
    };
    let body = b
        .get(payload..end)
        .ok_or("IP payload cut before transport header")?;
    match protocol {
        6 => {
            if body.len() < 20 {
                return Err("TCP header unavailable or cut");
            }
            let length = usize::from(body[12] >> 4) * 4;
            if length < 20 || payload + length > declared_end {
                return Err("Invalid TCP header length");
            }
            if length > body.len() {
                return Err("TCP options cut by capture");
            }
            let src = SocketAddr::new(source, be16(body, 0).ok_or("TCP port unavailable")?);
            let dst = SocketAddr::new(destination, be16(body, 2).ok_or("TCP port unavailable")?);
            packet.layers.push(format!(
                "TCP {src} → {dst} · flags 0x{:02x} · {length}-byte header",
                body[13]
            ));
            packet
                .layers
                .push("Checksums and receiver acceptance not verified".into());
            packet.tcp = Some(TcpSegment {
                source: src,
                destination: dst,
                sequence: be32(body, 4).ok_or("TCP sequence unavailable")?,
                acknowledgement: be32(body, 8).ok_or("TCP ACK unavailable")?,
                flags: body[13],
                window: be16(body, 14).ok_or("TCP window unavailable")?,
                payload: payload + length..end,
                declared_payload: declared_end - payload - length,
            });
        }
        17 => {
            if body.len() < 8 {
                return Err("UDP header unavailable or cut");
            }
            let length = usize::from(be16(body, 4).ok_or("UDP length unavailable")?);
            if length < 8 || payload + length > declared_end {
                return Err("Invalid UDP length");
            }
            let src = SocketAddr::new(source, be16(body, 0).ok_or("UDP port unavailable")?);
            let dst = SocketAddr::new(destination, be16(body, 2).ok_or("UDP port unavailable")?);
            packet
                .layers
                .push(format!("UDP {src} → {dst} · declared {length} bytes"));
            packet
                .layers
                .push("Checksums and receiver acceptance not verified".into());
            packet.udp = Some(UdpDatagram {
                source: src,
                destination: dst,
                payload: payload + 8..end.min(payload + length),
                complete: end >= payload + length,
            });
        }
        _ => packet.layers.push(format!(
            "IP protocol/next-header {protocol} outside TCP/UDP subset"
        )),
    }
    Ok(())
}
