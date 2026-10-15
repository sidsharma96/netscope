# How do separate pieces become a stream?

Three packet records contain parts of a book response. One arrives in the recording before an earlier part, and another repeats bytes already seen. Joining their payloads in file order would change the response. TCP supplies [sequence numbers](../GLOSSARY.md#sequence-number) so the receiver can distinguish where bytes belong from when a [segment](../GLOSSARY.md#segment) was observed.

A TCP segment is a piece of one direction's byte stream plus its transport header. Its sequence number identifies a position, rather than a display row. Sending the same bytes again can produce overlapping sequence ranges. A capture may also record the same traffic twice. Those possibilities mean an overlap is useful evidence without being a complete explanation of why it happened.

## Put bytes at positions before reading their meaning

This invented example shows three observations. The bracketed ranges include their starting position and exclude their end. The last row describes a repeated observation, not new data appended to the response.

```text
observed first:   [3..6) "def"  -> [missing 3 bytes] def
observed second:  [0..3) "abc"  -> abcdef
observed third:   [0..3) "abc"  -> abcdef, 3 agreeing overlap bytes
```

The final application bytes are `abcdef`. If the third observation instead contains `abX`, position two has conflicting evidence. Choosing whichever record appeared first or last would hide that disagreement. Netscope marks the position as conflicting and refuses to treat it as a uniquely observed byte.

Likewise, `abc`, a three-byte hole, and `ghi` must not become `abcghi`. Removing the hole manufactures adjacency. HTTP inspection uses only the uniquely observed prefix before the first hole or conflict. Later recorded bytes remain visible as sequence ranges even when application interpretation cannot continue.

## Remember absence separately from the value

A real application byte can have value zero. Filling missing positions with zero would make absence indistinguishable from recorded data. Netscope keeps byte values and their evidence states separately: Missing, Observed, or Conflict.

An enum, introduced in the missing-values lesson, lets those states have stable names. The stored byte value is meaningful only in the Observed state. This is a reason to choose a richer representation even when a plain byte vector looks simpler. The additional state vector costs memory, but makes a consequence predictable: a missing position cannot accidentally contribute a zero byte to an HTTP body.

A dense span reserves a slot for every position within its range, including holes. This makes overlapping positions easy to compare. A sparse map stores entries only for observed positions and could avoid allocating space for large holes, at the cost of per-entry overhead and different traversal rules. Netscope deliberately limits the span and rejects distant jumps before allocation. It does not allocate billions of missing positions just because an input supplies a large sequence number. The [architecture record](../ARCHITECTURE.md) documents this tradeoff and when to revisit it.

## Sequence numbers eventually wrap

TCP sequence numbers use 32 bits. After the largest representable number, the sequence continues at zero. This is circular numbering, not a connection restarting. [SYN](../GLOSSARY.md#syn), the flag used to establish initial sequence numbering, consumes a sequence position; its first data byte starts one position later. [FIN](../GLOSSARY.md#fin), the flag ending a direction's data, consumes another position without becoming an application byte. The [TCP specification](https://www.rfc-editor.org/rfc/rfc9293#section-3.4) defines sequence space.

Rust makes the arithmetic choice explicit. `u32::MAX` names the largest unsigned 32-bit value. `wrapping_add` adds with [wrap-around](../GLOSSARY.md#wrap-around) rather than treating overflow as an ordinary mathematical result outside the type.

```rust
let last = u32::MAX;
let next = last.wrapping_add(1);
assert_eq!(next, 0);
```

This explains a numerical rule, not a complete sequence comparison algorithm. To order positions on a circle, we need a bounded neighborhood. Netscope interprets modular differences only within its small supported sequence span. It cannot establish arbitrary history spanning half the circle from a handful of packet records. A normal integer comparison would put zero before the maximum even when those bytes are consecutive in a stream.

## A direction needs a beginning, and sometimes we do not have it

Source and destination addresses and ports identify a directed grouping. Reversing those endpoints gives another direction. A new SYN can establish a new [epoch](../GLOSSARY.md#directed-epoch), a separate period of observations for that direction; reused endpoints must not automatically merge an old connection with a new one.

Netscope uses conservative file-order boundaries for those epochs. A changed SYN or one following an observed close begins a new group. It does not pair every handshake or certify that a group is a globally unique connection. Without a SYN, the capture's beginning is unknown. Earlier out-of-order payload can extend its retained range, but that does not recover missing connection history.

A captured TCP header can declare six payload bytes while only three are stored. Those three are useful observations; the declared remaining positions become missing unless a later segment supplies them. An ACK reports a sequence acknowledgment, not the contents of those missing bytes, so it cannot fill the hole. Checksums and receiver acceptance are not verified by this analysis.

## An observed end is a qualified result

A consistent SYN-to-FIN span with all positions observed can support a limited end-of-data interpretation. Reset flags, inconsistent FIN positions, conflicts, missing bytes, or excluded segments prevent that result. Even an unambiguous observed span is not a proof that the receiver accepted the connection or that no duplicate capture exists elsewhere.

Budgets are part of the meaning too. Netscope checks the budgets one segment at a time, in file order. A segment that would stretch its epoch past 256 KiB of sequence span, or push the sum of all retained spans past 8 Mi positions, is skipped and counted in that epoch's skipped-segment total. The epoch itself survives with the segments that fitted before it, so which bytes remain depends on record order. A skipped segment also prevents the observed-end result. The 8 Mi figure counts sequence positions, not bytes of memory: each position costs one byte for the value and one byte for its evidence state, so the reassembly arrays can reach 16 MiB. If the epoch count is exhausted, a refused new connection must not send its later records back into an older group merely because the endpoint tuple matches. A safe stop needs a state rule as well as an allocation limit.

Try changing `def` in the illustration to arrive twice, then to arrive as `dXf`. Predict the agreeing-overlap count and the prefix available to an application parser. Then inspect the supplied gap and conflict examples in the [packet walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams). The reassembly implementation [`reassemble`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L89) and fixed tests [`sequence_span_and_total_memory_budgets_remain_bounded`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L56) preserve the distinctions above.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Sequence numbers and gaps

**Level:** Newcomer


TCP segments arrive with `abc` at positions 0 to 2 and `ghi` at positions 6 to 8. What does Netscope build, and what does the HTTP parser see?

<details>
<summary>Answer</summary>

TCP sequence numbers give each byte a position in one direction's stream. Netscope places bytes at their positions and keeps a separate state for each one: Missing, Observed or Conflict. Here positions 3 to 5 are Missing, and they are not filled with zeros or squeezed out. HTTP analysis reads only the contiguous observed prefix, `abc`, and stops at the gap. It never joins the pieces into `abcghi`.

**Learn more:** [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning) · [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`ByteState`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L13)

</details>

### What is the unit of TCP reassembly?

**Level:** Practitioner


What does Netscope reassemble TCP payload into, and how does it represent a byte position that was never seen, or one that was seen twice with different values?

<details>
<summary>Answer</summary>

The unit is a directed epoch: one direction (source socket to destination socket) of one connection lifetime. The two directions of a connection are never merged or paired. Each epoch has a `bytes` vector and a parallel `states` vector of the same length, where every position is `Missing`, `Observed` or `Conflict`.

A gap stays `Missing`. Its slot in `bytes` holds a placeholder 0 that means nothing, because a byte value is meaningful only where the state is `Observed`. If two observations of the same position agree, the state stays `Observed` and `repeated_bytes` goes up by one. If they disagree, the position becomes `Conflict`. `contiguous_prefix` returns bytes only up to the first position that is not `Observed`, so later analysis such as HTTP framing never crosses a hole or a disagreement.

**Learn more:** [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`ByteState`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L13-L20) · [`contiguous_prefix`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L53-L60)

</details>

### A reused port, a repeated SYN, and a late ACK

**Level:** Expert


A client reuses the same source port for a second connection to the same server. Separately, a SYN is retransmitted. Separately, a pure ACK arrives after the FIN. Which directed epoch does each of these packets join?

<details>
<summary>Answer</summary>

Netscope keeps one active epoch per direction and decides on each packet in file order. A SYN starts a new epoch when the active one has no SYN, when its sequence number differs from the active anchor (a different ISN), or when the active epoch was already closed by a FIN or RST. Port reuse with a new SYN therefore starts a new epoch, and the second connection never merges into the first.

A retransmitted SYN with the same ISN on an open epoch matches the anchor, so it joins the existing epoch. A pure ACK after the FIN is not a SYN, so it joins the active epoch even though that epoch is closed. It adds no byte positions, because a segment with no payload and no FIN or RST is skipped when bytes are placed.

The weak case is reuse where the new connection's SYN was not captured. Its first packet is not a SYN, so it joins the old closed epoch, and its bytes are placed relative to the old anchor.

**Learn more:** [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`is_none_or`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L99-L101) · [`tuple_reuse_resets_directions_and_budget_does_not_merge_new_epochs_into_old`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L36)

</details>

### Repeated bytes in a trace

**Level:** Newcomer


Two TCP segments carry the same sequence range with the same bytes. Is that a retransmission after packet loss?

<details>
<summary>Answer</summary>

It might be, but the trace does not say so. The sender may have sent again, or the capture setup may have recorded the same packet twice. Netscope counts agreeing overlaps as repeated bytes and marks positions where copies disagree as conflicts. It does not label a repeat as loss recovery, because the cause is not in the recorded fields.

**Common wrong answer.** "Repeated bytes always mean loss and retransmission."

**Learn more:** [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning)

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
