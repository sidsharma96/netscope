//! Sequence-space and budget rules use fixed observations, not live TCP.
use netscope_core::{
    packets::{Packet, PacketCapture, TcpSegment},
    reassembly::{FLOW_LIMIT, STREAM_LIMIT, TOTAL_STREAM_LIMIT, reassemble},
};
fn segment(port: u16, sequence: u32, flags: u8, body: &[u8]) -> Packet {
    Packet {
        seconds: 0,
        nanos: 0,
        original_len: body.len() as u32,
        bytes: body.to_vec(),
        layers: vec![],
        udp: None,
        tcp: Some(TcpSegment {
            source: std::net::SocketAddr::from(([192, 0, 2, 1], port)),
            destination: std::net::SocketAddr::from(([198, 51, 100, 1], 80)),
            sequence,
            acknowledgement: 0,
            flags,
            window: 1,
            payload: 0..body.len(),
            declared_payload: body.len(),
        }),
    }
}
fn capture(packets: Vec<Packet>) -> PacketCapture {
    PacketCapture {
        link_type: 1,
        snaplen: 65535,
        nanoseconds: false,
        packets,
    }
}
#[test]
fn tuple_reuse_resets_directions_and_budget_does_not_merge_new_epochs_into_old() {
    let mut packets = vec![
        segment(1, 10, 2, b""),
        segment(1, 11, 0x18, b"old"),
        segment(1, 14, 1, b""),
        segment(1, 20, 2, b""),
        segment(1, 21, 0x18, b"new"),
    ];
    assert_eq!(reassemble(&capture(packets)).flows.len(), 2);
    packets = vec![segment(1, 10, 2, b""), segment(1, 11, 0x18, b"old")];
    for port in 2..=FLOW_LIMIT as u16 {
        packets.push(segment(port, 10, 2, b""));
    }
    packets.extend([segment(1, 500, 2, b""), segment(1, 501, 0x18, b"new")]);
    let result = reassemble(&capture(packets));
    assert_eq!(result.flows.len(), FLOW_LIMIT);
    assert_eq!(result.unassigned_packets, 2);
    assert_eq!(result.flows[0].contiguous_prefix(), b"old");
}
#[test]
fn sequence_span_and_total_memory_budgets_remain_bounded() {
    let mut packets = vec![
        segment(1, 10, 2, b""),
        segment(1, 11, 0x18, b"x"),
        segment(1, 0x70000000, 0x18, b"bad"),
    ];
    let limited = reassemble(&capture(packets));
    assert_eq!(limited.flows[0].skipped_segments, 1);
    assert_eq!(limited.flows[0].bytes, b"x");
    packets = vec![];
    for port in 1..=40 {
        packets.push(segment(port, 0, 2, b""));
        packets.push(segment(port, STREAM_LIMIT as u32, 0x18, b"x"));
    }
    let result = reassemble(&capture(packets));
    assert_eq!(
        result.flows.iter().map(|s| s.bytes.len()).sum::<usize>(),
        TOTAL_STREAM_LIMIT
    );
    assert!(result.flows.iter().any(|s| s.skipped_segments > 0));
}
#[test]
fn without_syn_out_of_order_prefix_is_retained_but_not_claimed_as_connection_start() {
    let result = reassemble(&capture(vec![
        segment(1, 103, 0x18, b"def"),
        segment(1, 100, 0x18, b"abc"),
        segment(1, 106, 1, b""),
    ]));
    assert_eq!(result.flows[0].contiguous_prefix(), b"abcdef");
    assert_eq!(result.flows[0].first_sequence, 100);
    assert!(!result.flows[0].observed_end());
}
