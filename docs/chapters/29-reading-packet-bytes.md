# Which bytes were actually recorded?

A book request appears in the browser's export, but now you want to see the packets beneath it. A packet recording answers a different question. Instead of an exporter describing an HTTP response, it stores pieces of traffic seen at a capture point. Some pieces may be cut short. Some protocols may be outside the inspector's supported formats. Understanding the recording begins with finding exactly which bytes exist.

PCAP is a file format for packet records. Its file header tells the reader how to interpret later records, including their link type and timestamp representation. Each record reports both an original packet length and a captured length. If the original length is 120 bytes but only 80 are stored, the missing forty bytes are not zeros. They are unavailable. Netscope's classic PCAP reader preserves that distinction; PCAPNG, a different container format, is explicitly outside this first subset.

## A boundary belongs to a particular layer

Ethernet is a common format for frames, the pieces transmitted on a local-network link. IP supplies addresses for routing packets, while TCP carries an ordered stream between endpoints. With those layers present, an Ethernet frame contains an IP packet, whose payload contains a TCP segment, whose payload contains application bytes. A payload is the part carried by a layer after its own header. A header contains instructions or metadata needed to interpret that layer. The same byte can therefore be application data when viewed from TCP and part of TCP's payload when viewed from IP.

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

Not every recording starts with Ethernet, not every IP packet contains TCP, and not every TCP payload contains plaintext HTTP. The link type matters before the first packet byte is interpreted. Netscope also supports selected raw-IP link types. Unknown types retain their bytes and an explanation instead of receiving invented Ethernet addresses.

IPv4 and TCP headers can have options, so their lengths are not always the minimum. A length field must be checked against the enclosing packet and the retained bytes before advancing. Ethernet tags add another boundary. IPv6 can lead through extension headers; this first reader stops before unsupported chains. An IP fragment is not automatically a complete TCP segment, so fragments remain outside transport analysis. See the [IPv4](https://www.rfc-editor.org/rfc/rfc791) and [IPv6](https://www.rfc-editor.org/rfc/rfc8200) specifications for their layer rules.

## Two bytes need an agreed order

A port number is larger than a single byte can represent. A byte has eight bits, each either zero or one, and can represent 256 different values. Two bytes can represent 65,536 values. The order matters: bytes `01 bb` represent 443 when the first byte supplies the high part, but reversing them produces a different number.

Rust names an unsigned eight-bit integer `u8` and an unsigned sixteen-bit integer `u16`. Unsigned means no negative values. A fixed array contains a known number of values. The expression below creates a two-byte array and interprets it in big-endian order, with the high byte first.

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

The record timestamp belongs to the capture process. It is not automatically the time the application received a message or an accurate server clock. Converting microsecond fractions into nanosecond units does not add precision to the measurement. Netscope retains record order rather than sorting bytes into a receiver transcript.

Try the 120-byte packet with an 80-byte capture again. If a supported TCP header fits but declares more payload than exists, which part is usable? The checked header and retained prefix can remain visible; the missing bytes cannot be recovered by increasing an offset. The [local packet walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams) supplies examples, and the [reader](../../crates/netscope-import/src/pcap.rs) implements the bounds behind them.
