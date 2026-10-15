# Which bytes were actually recorded?

A book request appears in the browser's export, but now you want to see the packets beneath it. A packet recording answers a different question. Instead of an exporter describing an HTTP response, it stores pieces of traffic seen at a capture point. Some pieces may be cut short. Some protocols may be outside the inspector's supported formats. Understanding the recording begins with finding exactly which bytes exist.

[PCAP](../GLOSSARY.md#pcap) is a file format for packet records. Its file header tells the reader how to interpret later records, including their [link type](../GLOSSARY.md#link-type) and timestamp representation. Each record reports both an original packet length and a captured length. If the original length is 120 bytes but only 80 are stored, the missing forty bytes are not zeros. They are unavailable. Netscope's classic PCAP reader preserves that distinction; PCAPNG, a different container format, is explicitly outside this first subset.

## A boundary belongs to a particular layer

[Ethernet](../GLOSSARY.md#ethernet) is a common format for frames, the pieces transmitted on a local-network link. IP supplies addresses for routing packets, while [TCP](../GLOSSARY.md#tcp) carries an ordered stream between endpoints. With those layers present, an Ethernet frame contains an IP packet, whose payload contains a TCP segment, whose payload contains application bytes. A payload is the part carried by a layer after its own header. A header contains instructions or metadata needed to interpret that layer. The same byte can therefore be application data when viewed from TCP and part of TCP's payload when viewed from IP.

The following illustration shows containment, not a measured allocation layout. Read the nested boxes from outside inward. Each outer layer supplies a reason to interpret the next range.

```text
PCAP record: [timestamp | captured/original sizes | stored frame]
                                                   |
Ethernet:    [link header | IP packet..........................]
                           |
IP:          [IP header | TCP segment.........................]
                         |
TCP:         [TCP header | application bytes..................]
```

Not every recording starts with Ethernet, not every IP packet contains TCP, and not every TCP payload contains plaintext HTTP. The link type matters before the first packet byte is interpreted. Besides Ethernet, Netscope supports BSD loopback (what macOS writes for `lo0`), the Linux "cooked" headers that `tcpdump -i any` writes, and selected raw-IP link types. Unknown types retain their bytes and an explanation instead of receiving invented Ethernet addresses.

[IPv4](../GLOSSARY.md#ip) and TCP headers can have options, so their lengths are not always the minimum. A length field must be checked against the enclosing packet and the retained bytes before advancing. Ethernet tags add another boundary. IPv6 can lead through extension headers; this first reader stops before unsupported chains. An IP fragment is not automatically a complete TCP segment, so fragments remain outside transport analysis. See the [IPv4](https://www.rfc-editor.org/rfc/rfc791) and [IPv6](https://www.rfc-editor.org/rfc/rfc8200) specifications for their layer rules.

## Two bytes need an agreed order

A port number is larger than a single byte can represent. A byte has eight bits, each either zero or one, and can represent 256 different values. Two bytes can represent 65,536 values. The order matters: bytes `01 bb` represent 443 when the first byte supplies the high part, but reversing them produces a different number.

Rust names an unsigned eight-bit integer `u8` and an unsigned sixteen-bit integer `u16`. Unsigned means no negative values. A fixed array contains a known number of values. The expression below creates a two-byte array and interprets it in [big-endian](../GLOSSARY.md#byte-order) order, with the high byte first.

```rust
let bytes = [0x01, 0xbb];
let port = u16::from_be_bytes(bytes);
assert_eq!(port, 443);
```

`0x` introduces hexadecimal, a base-sixteen way to write a number; `bb` represents 187. The result is `1 × 256 + 187`. In `u16::from_be_bytes`, `::` selects a function associated with the `u16` type. The function receives the bytes and returns a number. It does not inspect the machine's native memory representation to guess a format.

IP and transport fields use network byte order. A classic PCAP file can use a different byte order for its container fields, identified by a fixed marker often called magic bytes. One reader therefore needs separate conversion rules for the file header and the packet inside it. Converting all numbers using the host computer's preferred order would work accidentally on some inputs and fail on others. The [PCAP format description](https://www.ietf.org/archive/id/draft-ietf-opsawg-pcap-06.html) is a versioned work-in-progress reference for that container distinction.

## Check availability before indexing

A file containing one byte cannot supply a two-byte field. Rust's `get` method lets a reader ask for a slice, a borrowed view of existing elements, and receive an optional result. The range `..2` means from the beginning up to, but excluding, element two.

This small teaching function demonstrates the rule. It is not a complete packet parser. The input `&[u8]` is a borrowed byte slice; the returned `Option<u16>` is either a present number or absence.

```rust
fn read_port(bytes: &[u8]) -> Option<u16> {
    match bytes.get(..2) {
        Some(pair) => Some(u16::from_be_bytes([pair[0], pair[1]])),
        None => None,
    }
}
assert_eq!(read_port(&[0x01, 0xbb]), Some(443));
assert_eq!(read_port(&[0x01]), None);
```

The `Some` branch runs only when the two-element slice exists, so its two indexes are valid. `None` describes missing input rather than a port numbered zero. The function borrows the bytes; it neither copies the whole recording nor extends its lifetime. This connects the ownership and missing-values lessons to a visible parsing consequence.

Netscope owns each record's byte vector and stores checked ranges identifying payloads within it. A later view borrows a range while the packet owner remains alive. Storing positions avoids a structure trying to own a byte vector and simultaneously hold a reference into itself. Reinterpreting file bytes as a struct with tightly placed fields would not solve bounds, variable lengths, or byte order. Memory mapping, where the operating system exposes a file through a memory region, could change how bytes are owned, but would still need those checks and a lifetime design.

## A timestamp still has a source

The record timestamp belongs to the capture process. It is not automatically the time the application received a message or an accurate server clock. Converting microsecond fractions into nanosecond units does not add precision to the measurement. Netscope stores both as nanoseconds but prints only the digits the file recorded: six fractional digits for a microsecond file, nine for a nanosecond file. Record timestamps are wall-clock readings, so they can also go backwards when the capturing machine's clock is stepped. Netscope retains record order rather than sorting bytes into a receiver transcript.

Try the 120-byte packet with an 80-byte capture again. If a supported TCP header fits but declares more payload than exists, which part is usable? The checked header and retained prefix can remain visible; the missing bytes cannot be recovered by increasing an offset. The [local packet walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams) supplies examples, and the reader [`import_pcap`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L69) implements the bounds behind them.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Captured length and original length

**Level:** Newcomer


A PCAP record says the original packet was 120 bytes but only 80 were captured. What does Netscope do with the missing 40?

<details>
<summary>Answer</summary>

It treats them as unavailable, never as zeros. The capture tool stored only the first 80 bytes, often because of a snap length setting. Netscope keeps the headers and payload prefix that fit, and adds the note "Capture cut: fewer bytes retained than original packet". Later, TCP reassembly marks the uncaptured positions as missing, so no analysis reads past them.

**Learn more:** [Which bytes were actually recorded?](../chapters/29-reading-packet-bytes.md#which-bytes-were-actually-recorded) · [Check availability before indexing](../chapters/29-reading-packet-bytes.md#check-availability-before-indexing) · [`import_pcap`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L69)

</details>

### What can the PCAP importer read?

**Level:** Practitioner


Which container formats, link types and network and transport layers does the PCAP importer support? What happens to a packet whose layers it cannot parse?

<details>
<summary>Answer</summary>

Only classic PCAP version 2.4 is supported, in either byte order and with microsecond or nanosecond timestamps. A PCAPNG file is rejected with an explicit message. The supported link types are 0 (BSD loopback, as macOS `lo0` writes), 1 (Ethernet, with up to two VLAN tags), 101 (raw IP), 113 and 276 (Linux cooked SLL and SLL2, as `tcpdump -i any` writes), 228 (raw IPv4) and 229 (raw IPv6). IPv4 and IPv6 are parsed, and the transport layers are TCP and UDP. Fragmented IPv4 packets are not reassembled, IPv6 extension headers are not walked, and checksums are never verified.

Errors in the file's framing fail the import: a snap length of 0 or above 256 KiB, a file over 32 MiB, a captured length above the snap length or original length, or a timestamp fraction out of range. Two framing problems keep what was read instead: a truncated final record is dropped with a note, and the records after the 20,000th are left unread with a note. Errors inside a packet do not fail the import. The packet keeps its bytes and gets a reason in its layer list, such as "Link type unsupported; packet bytes retained". An 802.11 capture (link type 105) therefore opens, but every packet carries that reason.

**Learn more:** [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer) · [Why refuse PCAPNG and other link types?](../chapters/34-defending-the-design.md#why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do) · [`import_pcap`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L69-L188) · [`layers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L189-L255)

</details>

### A capture taken with a 96-byte snap length

**Level:** Expert


A capture was recorded with a snap length of 96 bytes, so every TCP payload is cut. What do the reassembled byte states look like, what does `contiguous_prefix` return, and what does the HTTP framing inspector see?

<details>
<summary>Answer</summary>

Each record keeps 96 bytes. After a 14-byte Ethernet header, a 20-byte IPv4 header, and a 20 to 32-byte TCP header, between about 30 and 42 payload bytes remain. The importer adds the layer note "Capture cut: fewer bytes retained than original packet".

The segment's declared payload comes from the IP length field, not from the captured length, so reassembly reserves the full sequence range the sender used. Only the retained bytes are written. The rest of each segment's range stays Missing. The stream becomes a run of Observed bytes, then a run of Missing positions, then Observed again for the next segment, and so on.

`contiguous_prefix` stops at the first Missing position, inside the first data segment. The HTTP inspector therefore sees a few dozen bytes. It usually stops with "No complete HTTP/1.1 start line in prefix" or a header-line error. It never joins the retained fragments across the holes, and a Missing position is never read as a zero byte.

**Learn more:** [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`retained`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L175-L179) · [`declared_payload`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L372) · [`contiguous_prefix`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L53-L60)

</details>

### Reading a two-byte number

**Level:** Newcomer


A TCP header holds the port as bytes `01 bb`. What number is that, and why can't Netscope use the computer's own byte order?

<details>
<summary>Answer</summary>

It is 443: the first byte is the high part, so 1 × 256 + 187. Network headers use this big-endian order whatever machine reads them, and Rust's `u16::from_be_bytes` applies it explicitly. A classic PCAP file's own header fields can use either byte order, marked by its magic bytes. The reader therefore needs one rule for the file container and another for the packets inside it.

**Learn more:** [Two bytes need an agreed order](../chapters/29-reading-packet-bytes.md#two-bytes-need-an-agreed-order) · [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer)

</details>

### Which length bounds a TCP payload?

**Level:** Practitioner


A TCP segment's payload length could come from the captured record length, the IP header's length field, or the snap length. Which one does Netscope use for `declared_payload`, which one for the retained byte range, and why does the difference matter?

<details>
<summary>Answer</summary>

The IP layer computes a declared end: the start of the IP header plus the IPv4 total length, or plus 40 and the IPv6 payload length. The retained end is the smaller of that declared end and the captured bytes. `declared_payload` is the declared end minus the IP and TCP headers, so it comes from the IP length field. The retained `payload` range runs from the end of the TCP header to the retained end, so it holds only bytes that are present in the file.

Two cases show why both are needed. Ethernet pads short frames to 60 bytes, and using the record length would turn that padding into fake TCP payload; the IP length excludes it. In a capture cut by its snap length, the declared length still says how much was sent, so reassembly reserves the whole sequence range and marks the uncaptured part `Missing` instead of pretending the segment was short. UDP works the same way, with a `complete` flag when the whole datagram was retained.

**Learn more:** [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer) · [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`declared_end`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L265-L307) · [`declared_payload`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L364-L373)

</details>
