//! Fixed wire bytes, malformed boundaries and deterministic packet/stream observations.
use netscope_core::{
    reassembly::{ByteState, reassemble},
    wire_http::inspect_http,
};
use netscope_import::pcap::{PCAP_PACKET_LIMIT, import_pcap};
const SAMPLE: &[u8] = include_bytes!("../../../tests/fixtures/pcap/streams.pcap");

#[test]
fn classic_orders_layers_and_source_ranges_are_checked() -> Result<(), Box<dyn std::error::Error>> {
    let capture = import_pcap(SAMPLE)?;
    assert_eq!(capture.packets.len(), 24);
    assert_eq!(capture.packets[0].nanos, 123456000);
    let first = capture.packets[0].tcp.as_ref().ok_or("Missing TCP")?;
    assert_eq!(first.sequence, 100);
    assert_eq!(first.flags, 2);
    assert_eq!(first.payload.len(), 0);
    assert!(capture.packets[22].tcp.is_none());
    assert!(
        capture.packets[22]
            .layers
            .iter()
            .any(|s| s.contains("fragmented"))
    );
    assert!(
        capture.packets[21]
            .layers
            .iter()
            .any(|s| s.contains("VLAN ID 7"))
    );
    let big = import_pcap(include_bytes!("../../../tests/fixtures/pcap/big-nano.pcap"))?;
    assert!(big.nanoseconds);
    assert_eq!(big.packets[0].nanos, 123456789);
    assert!(
        big.packets[0]
            .tcp
            .as_ref()
            .is_some_and(|s| s.source.is_ipv6())
    );
    let raw = import_pcap(include_bytes!("../../../tests/fixtures/pcap/raw-ip.pcap"))?;
    assert!(raw.packets[0].tcp.is_some());
    let unknown = import_pcap(include_bytes!(
        "../../../tests/fixtures/pcap/unknown-link.pcap"
    ))?;
    assert_eq!(unknown.packets[0].bytes, b"opaque");
    assert!(unknown.packets[0].layers[0].contains("unsupported"));
    Ok(())
}
#[test]
fn file_framing_and_all_prefix_cuts_fail_safely() -> Result<(), Box<dyn std::error::Error>> {
    for cut in 0..SAMPLE.len() {
        let _ = import_pcap(&SAMPLE[..cut]);
    }
    for (offset, value) in [
        (4, 0u32),
        (16, 0),
        (16, 262145),
        (20, 0x10001),
        (28, 1_000_000),
        (32, 262145),
        (36, 0),
    ] {
        let mut bytes = SAMPLE.to_vec();
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(import_pcap(&bytes).is_err(), "offset {offset}");
    }
    assert!(import_pcap(&[0x0a, 0x0d, 0x0d, 0x0a]).is_err());
    let mut many = SAMPLE[..24].to_vec();
    for _ in 0..=PCAP_PACKET_LIMIT {
        many.extend_from_slice(&[0; 16]);
    }
    let kept = import_pcap(&many)?;
    assert_eq!(kept.packets.len(), PCAP_PACKET_LIMIT);
    assert_eq!(
        kept.notes,
        ["More than 20000 packet records; only the first 20000 were read"]
    );
    let mut generator = 7u32;
    for length in 0..512 {
        let mut bytes = vec![0; length];
        for b in &mut bytes {
            generator = generator.wrapping_mul(1664525).wrapping_add(1013904223);
            *b = (generator >> 24) as u8;
        }
        let _ = import_pcap(&bytes);
    }
    Ok(())
}
#[test]
fn sequence_reassembly_keeps_gaps_conflicts_cuts_wrap_and_chunk_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let capture = import_pcap(SAMPLE)?;
    let result = reassemble(&capture);
    assert_eq!(result.flows.len(), 6);
    let request = &result.flows[0];
    assert_eq!(request.repeated_bytes, 13);
    assert!(request.observed_end());
    assert!(
        request
            .contiguous_prefix()
            .starts_with(b"GET /books HTTP/1.1\r\n")
    );
    let response = &result.flows[1];
    let parsed = inspect_http(response.contiguous_prefix(), response.observed_end());
    assert_eq!(parsed.messages.len(), 1);
    let message = &parsed.messages[0];
    assert!(message.framed);
    assert_eq!(message.chunks.len(), 3);
    assert_eq!(message.trailers[0].0, "X-End");
    assert!(message.sdp.is_some_and(|s| s.contains("not proof")));
    assert!(message.body.starts_with(b"v=0\r\n"));
    let gap = &result.flows[2];
    assert!(gap.states.contains(&ByteState::Missing));
    assert_eq!(gap.contiguous_prefix(), b"GET /gap HTTP/1.1\r\n");
    assert!(
        inspect_http(gap.contiguous_prefix(), false)
            .messages
            .is_empty()
    );
    assert_eq!(
        result.flows[3].states,
        [ByteState::Observed, ByteState::Conflict]
    );
    assert_eq!(result.flows[4].contiguous_prefix(), b"abc");
    assert_eq!(result.flows[4].states.len(), 6);
    assert_eq!(result.flows[5].contiguous_prefix(), b"abc");
    assert!(result.flows[5].observed_end());
    let dns = netscope_core::datagrams::inspect_datagram(&capture.packets[20]).join("\n");
    assert!(
        dns.contains("203.0.113.10") && dns.contains("203.0.113.11") && dns.contains("2001:db8::8")
    );
    assert!(dns.contains("not proof"));
    assert!(
        netscope_core::datagrams::inspect_datagram(&capture.packets[21])
            .join("\n")
            .contains("STUN candidate")
    );
    Ok(())
}
#[test]
fn layer_length_mutations_and_unknown_ipv6_headers_never_read_past_records()
-> Result<(), Box<dyn std::error::Error>> {
    // First Ethernet/IPv4/TCP record begins at file offset 40.
    for (position, value) in [(54, 0x44), (54, 0x4f), (86, 0x10), (86, 0xf0), (63, 250)] {
        let mut bytes = SAMPLE.to_vec();
        bytes[position] = value;
        let capture = import_pcap(&bytes)?;
        assert!(capture.packets[0].tcp.is_none());
    }
    let mut bytes = include_bytes!("../../../tests/fixtures/pcap/big-nano.pcap").to_vec();
    bytes[40 + 14 + 6] = 0;
    let capture = import_pcap(&bytes)?;
    assert!(capture.packets[0].tcp.is_none());
    Ok(())
}

#[test]
fn datagram_cuts_dns_pointer_errors_and_stun_lengths_remain_scoped()
-> Result<(), Box<dyn std::error::Error>> {
    let mut capture = import_pcap(SAMPLE)?;
    let dns = &mut capture.packets[20];
    let start = dns.udp.as_ref().ok_or("Missing DNS UDP")?.payload.start;
    dns.bytes[start + 12..start + 14].copy_from_slice(&[0xc0, 0x0c]);
    assert!(
        netscope_core::datagrams::inspect_datagram(dns)
            .join("\n")
            .contains("pointer")
    );
    let stun = &mut capture.packets[21];
    let start = stun.udp.as_ref().ok_or("Missing STUN UDP")?.payload.start;
    stun.bytes[start + 2..start + 4].copy_from_slice(&4u16.to_be_bytes());
    assert!(
        netscope_core::datagrams::inspect_datagram(stun)
            .join("\n")
            .contains("invalid/incomplete")
    );
    let original = import_pcap(SAMPLE)?;
    let packet = &original.packets[20];
    let cut = packet.bytes.len() - 1;
    let mut file = SAMPLE[..24].to_vec();
    file.extend_from_slice(&[0; 8]);
    file.extend_from_slice(&(cut as u32).to_le_bytes());
    file.extend_from_slice(&(packet.bytes.len() as u32).to_le_bytes());
    file.extend_from_slice(&packet.bytes[..cut]);
    let cut_capture = import_pcap(&file)?;
    assert!(
        netscope_core::datagrams::inspect_datagram(&cut_capture.packets[0])
            .join("\n")
            .contains("datagram cut")
    );
    Ok(())
}

#[test]
fn remaining_orders_resolutions_vlan_and_variable_headers_are_supported()
-> Result<(), Box<dyn std::error::Error>> {
    let mut little_nano = SAMPLE.to_vec();
    little_nano[..4].copy_from_slice(&[0x4d, 0x3c, 0xb2, 0xa1]);
    assert_eq!(import_pcap(&little_nano)?.packets[0].nanos, 123456);
    let mut big_micro = include_bytes!("../../../tests/fixtures/pcap/big-nano.pcap").to_vec();
    big_micro[..4].copy_from_slice(&[0xa1, 0xb2, 0xc3, 0xd4]);
    big_micro[28..32].copy_from_slice(&654321u32.to_be_bytes());
    assert_eq!(import_pcap(&big_micro)?.packets[0].nanos, 654321000);
    let mut raw = include_bytes!("../../../tests/fixtures/pcap/raw-ip.pcap").to_vec();
    raw[20..24].copy_from_slice(&228u32.to_le_bytes());
    assert!(import_pcap(&raw)?.packets[0].tcp.is_some());
    fn single(frame: &[u8]) -> Vec<u8> {
        let mut file = SAMPLE[..24].to_vec();
        file.extend_from_slice(&[0; 8]);
        file.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        file.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        file.extend_from_slice(frame);
        file
    }
    let original = import_pcap(SAMPLE)?;
    let frame = &original.packets[0].bytes;
    let mut tagged = frame[..12].to_vec();
    tagged.extend_from_slice(&[0x88, 0xa8, 0, 3, 0x81, 0, 0, 4, 8, 0]);
    tagged.extend_from_slice(&frame[14..]);
    let capture = import_pcap(&single(&tagged))?;
    assert!(capture.packets[0].tcp.is_some());
    assert_eq!(
        capture.packets[0]
            .layers
            .iter()
            .filter(|s| s.contains("VLAN ID"))
            .count(),
        2
    );
    let mut options = frame[..34].to_vec();
    options[14] = 0x46;
    options[16..18].copy_from_slice(&48u16.to_be_bytes());
    options.extend_from_slice(&[1, 1, 1, 1]);
    options.extend_from_slice(&frame[34..54]);
    options[38 + 12] = 0x60;
    options.extend_from_slice(&[1, 1, 1, 1]);
    let capture = import_pcap(&single(&options))?;
    let tcp = capture.packets[0]
        .tcp
        .as_ref()
        .ok_or("Variable TCP header unavailable")?;
    assert_eq!(tcp.payload.start, 62);
    assert_eq!(tcp.declared_payload, 0);
    assert!(
        capture.packets[0]
            .layers
            .iter()
            .any(|s| s.contains("24-byte header"))
    );
    Ok(())
}
#[test]
fn loopback_and_linux_cooked_link_types_decode_ip() -> Result<(), Box<dyn std::error::Error>> {
    for (file, payloads) in [
        (
            include_bytes!("../../../tests/fixtures/pcap/loopback-null.pcap").as_slice(),
            [b"loop4".as_slice(), b"loop6"].as_slice(),
        ),
        (
            include_bytes!("../../../tests/fixtures/pcap/loopback-null-be.pcap"),
            &[b"loop4", b"loop6"],
        ),
        (
            include_bytes!("../../../tests/fixtures/pcap/linux-sll.pcap"),
            &[b"cooked"],
        ),
        (
            include_bytes!("../../../tests/fixtures/pcap/linux-sll2.pcap"),
            &[b"cooked2"],
        ),
    ] {
        let capture = import_pcap(file)?;
        assert_eq!(capture.packets.len(), payloads.len());
        for (packet, payload) in capture.packets.iter().zip(payloads) {
            let tcp = packet.tcp.as_ref().ok_or("Missing TCP")?;
            assert_eq!(&packet.bytes[tcp.payload.clone()], *payload);
            assert_eq!(tcp.destination.port(), 80);
        }
    }
    // An unknown loopback family and a cut cooked header stay labelled, not decoded.
    let mut odd = include_bytes!("../../../tests/fixtures/pcap/loopback-null.pcap").to_vec();
    odd[40] = 99;
    assert!(import_pcap(&odd)?.packets[0].tcp.is_none());
    let mut cut = include_bytes!("../../../tests/fixtures/pcap/linux-sll.pcap").to_vec();
    cut.truncate(24 + 16 + 10);
    cut[32..36].copy_from_slice(&10u32.to_le_bytes());
    cut[36..40].copy_from_slice(&10u32.to_le_bytes());
    assert!(import_pcap(&cut)?.packets[0].tcp.is_none());
    Ok(())
}
#[test]
fn ipv4_total_length_zero_from_offload_uses_the_frame_length()
-> Result<(), Box<dyn std::error::Error>> {
    let mut raw = include_bytes!("../../../tests/fixtures/pcap/raw-ip.pcap").to_vec();
    // The IPv4 header starts after the 24-byte file and 16-byte record headers.
    raw[42..44].copy_from_slice(&[0, 0]);
    let capture = import_pcap(&raw)?;
    let packet = &capture.packets[0];
    let tcp = packet.tcp.as_ref().ok_or("Missing TCP")?;
    assert_eq!(&packet.bytes[tcp.payload.clone()], b"raw");
    assert_eq!(tcp.declared_payload, 3);
    assert!(packet.layers.iter().any(|s| s.contains("offload")));
    // A header-only frame with total length 0 is still invalid.
    let mut bare = raw[..40 + 20].to_vec();
    bare[32..36].copy_from_slice(&20u32.to_le_bytes());
    bare[36..40].copy_from_slice(&20u32.to_le_bytes());
    assert!(import_pcap(&bare)?.packets[0].tcp.is_none());
    Ok(())
}
#[test]
fn a_truncated_final_record_keeps_the_complete_records_before_it()
-> Result<(), Box<dyn std::error::Error>> {
    let whole = import_pcap(SAMPLE)?;
    assert!(whole.notes.is_empty());
    // Cut inside the last record's data, then inside its 16-byte header.
    let last = SAMPLE.len() - whole.packets[23].bytes.len();
    for cut in [SAMPLE.len() - 1, last - 16 + 7] {
        let capture = import_pcap(&SAMPLE[..cut])?;
        assert_eq!(capture.packets.len(), 23);
        assert_eq!(
            capture.notes,
            ["Final packet record truncated; earlier records kept"]
        );
    }
    // With no complete record there is nothing to keep, so the file still fails.
    assert!(import_pcap(&SAMPLE[..24 + 20]).is_err());
    Ok(())
}
