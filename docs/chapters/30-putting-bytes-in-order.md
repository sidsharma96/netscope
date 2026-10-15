# How do separate pieces become a stream?

Three packet records contain parts of a book response. One arrives in the recording before an earlier part, and another repeats bytes already seen. Joining their payloads in file order would change the response. TCP supplies sequence numbers so the receiver can distinguish where bytes belong from when a segment was observed.

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

TCP sequence numbers use 32 bits. After the largest representable number, the sequence continues at zero. This is circular numbering, not a connection restarting. SYN, the flag used to establish initial sequence numbering, consumes a sequence position; its first data byte starts one position later. FIN, the flag ending a direction's data, consumes another position without becoming an application byte. The [TCP specification](https://www.rfc-editor.org/rfc/rfc9293#section-3.4) defines sequence space.

Rust makes the arithmetic choice explicit. `u32::MAX` names the largest unsigned 32-bit value. `wrapping_add` adds with wrap-around rather than treating overflow as an ordinary mathematical result outside the type.

```rust
let last = u32::MAX;
let next = last.wrapping_add(1);
assert_eq!(next, 0);
```

This explains a numerical rule, not a complete sequence comparison algorithm. To order positions on a circle, we need a bounded neighborhood. Netscope interprets modular differences only within its small supported sequence span. It cannot establish arbitrary history spanning half the circle from a handful of packet records. A normal integer comparison would put zero before the maximum even when those bytes are consecutive in a stream.

## A direction needs a beginning, and sometimes we do not have it

Source and destination addresses and ports identify a directed grouping. Reversing those endpoints gives another direction. A new SYN can establish a new epoch, a separate period of observations for that direction; reused endpoints must not automatically merge an old connection with a new one.

Netscope uses conservative file-order boundaries for those epochs. A changed SYN or one following an observed close begins a new group. It does not pair every handshake or certify that a group is a globally unique connection. Without a SYN, the capture's beginning is unknown. Earlier out-of-order payload can extend its retained range, but that does not recover missing connection history.

A captured TCP header can declare six payload bytes while only three are stored. Those three are useful observations; the declared remaining positions become missing unless a later segment supplies them. An ACK reports a sequence acknowledgment, not the contents of those missing bytes, so it cannot fill the hole. Checksums and receiver acceptance are not verified by this analysis.

## An observed end is a qualified result

A consistent SYN-to-FIN span with all positions observed can support a limited end-of-data interpretation. Reset flags, inconsistent FIN positions, conflicts, missing bytes, or excluded segments prevent that result. Even an unambiguous observed span is not a proof that the receiver accepted the connection or that no duplicate capture exists elsewhere.

Budgets are part of the meaning too. Netscope checks the budgets one segment at a time, in file order. A segment that would stretch its epoch past 256 KiB of sequence span, or push the sum of all retained spans past 8 Mi positions, is skipped and counted in that epoch's skipped-segment total. The epoch itself survives with the segments that fitted before it, so which bytes remain depends on record order. A skipped segment also prevents the observed-end result. The 8 Mi figure counts sequence positions, not bytes of memory: each position costs one byte for the value and one byte for its evidence state, so the reassembly arrays can reach 16 MiB. If the epoch count is exhausted, a refused new connection must not send its later records back into an older group merely because the endpoint tuple matches. A safe stop needs a state rule as well as an allocation limit.

Try changing `def` in the illustration to arrive twice, then to arrive as `dXf`. Predict the agreeing-overlap count and the prefix available to an application parser. Then inspect the supplied gap and conflict examples in the [packet walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams). The reassembly implementation [`reassemble`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L87) and fixed tests [`sequence_span_and_total_memory_budgets_remain_bounded`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L55) preserve the distinctions above.
