//! DNS name traversal limits use constructed messages, not recorded resolver traffic.
use netscope_core::{
    datagrams::inspect_datagram,
    packets::{Packet, UdpDatagram},
};
fn dns_packet(message: &[u8]) -> Packet {
    Packet {
        seconds: 0,
        nanos: 0,
        original_len: message.len() as u32,
        bytes: message.to_vec(),
        layers: vec![],
        tcp: None,
        udp: Some(UdpDatagram {
            source: std::net::SocketAddr::from(([192, 0, 2, 53], 53)),
            destination: std::net::SocketAddr::from(([192, 0, 2, 1], 5353)),
            payload: 0..message.len(),
            complete: true,
        }),
    }
}
fn header(questions: u16, answers: u16) -> Vec<u8> {
    let mut bytes = vec![0, 7, 0x81, 0x80];
    bytes.extend_from_slice(&questions.to_be_bytes());
    bytes.extend_from_slice(&answers.to_be_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes
}
#[test]
fn a_legal_name_with_more_than_64_labels_is_read() {
    // 100 one-letter labels encode to 201 bytes, inside the 255-byte limit.
    let mut message = header(1, 1);
    for _ in 0..100 {
        message.extend_from_slice(&[1, b'a']);
    }
    message.extend_from_slice(&[0, 0, 1, 0, 1]);
    // The answer owner is "b" followed by a pointer to the question name.
    message.extend_from_slice(&[
        1, b'b', 0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 203, 0, 113, 9,
    ]);
    let lines = inspect_datagram(&dns_packet(&message)).join("\n");
    let name = vec!["a"; 100].join(".");
    assert!(
        lines.contains(&format!("Question {name} · type 1")),
        "{lines}"
    );
    assert!(
        lines.contains(&format!("Answer b.{name} → 203.0.113.9")),
        "{lines}"
    );
}
#[test]
fn expanded_name_length_includes_the_terminal_zero() {
    for (last_label, accepted) in [(61, true), (62, false)] {
        let mut message = header(1, 0);
        for length in [63, 63, 63, last_label] {
            message.push(length);
            message.extend(vec![b'a'; usize::from(length)]);
        }
        message.extend_from_slice(&[0, 0, 1, 0, 1]);
        let lines = inspect_datagram(&dns_packet(&message));
        if accepted {
            assert!(lines.iter().any(|line| line.starts_with("Question ")));
        } else {
            assert_eq!(
                lines,
                ["DNS candidate unavailable: DNS name exceeds bounded label size"]
            );
        }
    }
    let mut root = header(1, 0);
    root.extend_from_slice(&[0, 0, 1, 0, 1]);
    assert!(inspect_datagram(&dns_packet(&root)).contains(&"Question  · type 1".into()));
}
#[test]
fn compressed_name_length_counts_the_expanded_suffix_and_terminal_zero() {
    for (prefix, accepted) in [(61, true), (62, false)] {
        let mut message = header(1, 1);
        for _ in 0..3 {
            message.push(63);
            message.extend([b'a'; 63]);
        }
        message.extend_from_slice(&[0, 0, 1, 0, 1]);
        message.push(prefix);
        message.extend(vec![b'b'; usize::from(prefix)]);
        message.extend_from_slice(&[0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 203, 0, 113, 9]);
        let lines = inspect_datagram(&dns_packet(&message));
        if accepted {
            assert!(lines.iter().any(|line| line.ends_with("→ 203.0.113.9")));
        } else {
            assert_eq!(
                lines,
                ["DNS candidate unavailable: DNS name exceeds bounded label size"]
            );
        }
    }
}
#[test]
fn pointer_loops_and_long_pointer_chains_stop() {
    // A label at 12 followed by a pointer back to 12 repeats forever without a limit.
    let mut looped = header(1, 0);
    looped.extend_from_slice(&[1, b'a', 0xc0, 12, 0, 1, 0, 1]);
    let lines = inspect_datagram(&dns_packet(&looped)).join("\n");
    assert!(lines.contains("DNS candidate unavailable"), "{lines}");
    // Store a root and 300 backward pointers inside an earlier answer's raw data.
    // The second answer actually enters at the far end of that chain.
    let mut message = header(0, 2);
    message.extend_from_slice(&[0, 0, 99, 0, 1, 0, 0, 0, 60]);
    message.extend_from_slice(&601u16.to_be_bytes());
    let mut previous = message.len() as u16;
    message.push(0);
    for _ in 0..300 {
        let here = message.len() as u16;
        message.extend_from_slice(&(0xc000 | previous).to_be_bytes());
        previous = here;
    }
    message.extend_from_slice(&(0xc000 | previous).to_be_bytes());
    message.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 203, 0, 113, 9]);
    let lines = inspect_datagram(&dns_packet(&message)).join("\n");
    assert_eq!(lines, "DNS candidate unavailable: DNS name traversal limit");
}
