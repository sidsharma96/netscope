# Deterministic packet illustrations

Generated on October 13, 2026 by [build_packet_fixtures.py](../../../scripts/build_packet_fixtures.py). These are constructed examples, not captures from browsers or an observed network. Fixed seconds/fractions, documentation addresses, payloads and control fields make regression expectations independent of clocks and external services. SHA256SUMS records their checked-in bytes; rebuilding must reproduce them exactly.

| File | Evidence exercised |
| --- | --- |
| streams.pcap | Little-endian/microsecond Ethernet; 24 records, six directed TCP epochs, out-of-order GET/chunked response, duplicate payload, gap, conflicting overlap, capture cut, wrapping sequence, UDP DNS A/AAAA and IPv6 VLAN/STUN candidate, IPv4 fragment, opaque ARP payload |
| big-nano.pcap | Big-endian/nanosecond IPv6/TCP |
| raw-ip.pcap | Link type 101 with IPv4/TCP |
| unknown-link.pcap | Unsupported type 147 with retained opaque bytes |

streams.pcap #0..8 contain two directed SYN/data/FIN observations. The response's wire chunks retain an extension and a trailer; application bytes are an illustrative SDP-like description. #9..11 contain a known sequence gap. #12..14 disagree about a payload position. #15..16 retain only three of six declared payload bytes. #17..19 cross the 32-bit sequence boundary. #20 is a DNS response with two IPv4 and one IPv6 answer, not proof of load balancing. #21 has a framed STUN-shaped header, not evidence of ICE success. #22 is deliberately outside fragment handling and #23 outside ARP decoding.

The generator writes valid IP/TCP/UDP checksum arithmetic for its supported examples, but Netscope does not validate checksums or claim these synthetic flag/ACK sequences were accepted by any receiver. An independent macOS tcpdump 4.99.1/libpcap 1.10.1 read recognized supported fields, checksum values, DNS answers and link types. Unsupported/truncated synthetic content remains labelled; this cross-check is not a real network observation or whole-protocol validation.

PCAPNG, extension chains, IP fragments, capture interfaces and encryption keys are not hidden in these fixtures. Derived malformed variants belong in tests rather than edits to the frozen examples.
