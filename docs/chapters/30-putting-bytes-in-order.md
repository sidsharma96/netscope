# How do separate pieces become a stream?

Three packet records contain parts of a book response. One arrives in the recording before an earlier part, and another repeats bytes already seen. Joining their payloads in file order would change the response. TCP supplies [sequence numbers](../GLOSSARY.md#sequence-number) so the receiver can distinguish where bytes belong from when a [segment](../GLOSSARY.md#segment) was observed.

A TCP segment is a piece of one direction's byte stream plus its transport header. Its sequence number identifies a position, rather than a display row. Sending the same bytes again can produce overlapping sequence ranges. A capture may also record the same traffic twice. Those possibilities mean an overlap is useful evidence without being a complete explanation of why it happened.

## Sequence describes one direction; ACK describes the other

Imagine a sender transmitting the three bytes `abc`, beginning at sequence position 100. Those bytes occupy positions 100, 101 and 102. If its peer receives them in order, the peer can report ACK 103: the next position it expects. It is acknowledging progress through sequence space, not counting three packets.

The drawing below is an invented exchange. Read the upper arrow as data moving toward the peer and the lower arrow as a report coming back. The sequence numbers of any data travelling on the lower arrow belong to a separate numbering space; they need not be close to 100.

```text
Illustration: ACK flag is set on the reply

Sender                                      Peer
       Sequence 100, data "abc" ────────────→
       positions: 100 101 102
                               ←──────────── ACK 103
                                             expects position 103 next
```

Now suppose position 101 is missing but position 102 arrives. The cumulative acknowledgement cannot advance past the hole merely because a later byte is available. An ACK reports the next expected position after the contiguous sequence space received so far. It does not supply the contents of missing captured bytes, and it does not show that the receiving application read them.

The ACK flag is essential to this interpretation. When it is off, the raw acknowledgement field is not an active acknowledgement. A displayed zero is not evidence that the peer is waiting for byte zero. Conversely, an active ACK of zero can be meaningful because sequence numbers wrap. The packet view therefore states whether the flag is on before interpreting the number. These rules follow the [TCP sequence and acknowledgement definitions](https://www.rfc-editor.org/rfc/rfc9293#section-3.4).

## Put bytes at positions before reading their meaning

This invented example shows three observations. The bracketed ranges include their starting position and exclude their end. The last row describes a repeated observation, not new data appended to the response.

```text
observed first:   [3..6) "def"  -> [missing 3 bytes] def
observed second:  [0..3) "abc"  -> abcdef
observed third:   [0..3) "abc"  -> abcdef, 3 agreeing overlap bytes
```

The final application bytes are `abcdef`. If the third observation instead contains `abX`, position two has conflicting evidence. Choosing whichever record appeared first or last would hide that disagreement. Netscope marks the position as conflicting and refuses to treat it as a uniquely observed byte.

Likewise, `abc`, a three-byte hole, and `ghi` must not become `abcghi`. Removing the hole manufactures adjacency. HTTP inspection uses only the uniquely observed prefix before the first hole or conflict. Later recorded bytes remain visible as sequence ranges even when application interpretation cannot continue.

There is one qualification to the conflict rule. TCP can use a small keep-alive probe to check whether a peer still responds. Netscope excludes a particular one-byte pattern as a possible probe before comparing overlapping bytes. The pattern alone does not establish intent: a genuine disagreement can fit it too. The original packet remains available, but the derived stream does not mark that excluded byte as a conflict. The [capstone](34-defending-the-design.md#follow-a-packet-gap-into-the-application) returns to this uncertainty when examining the design's limits.

## Read a coverage map without hiding a hole

Suppose a recording retains `abc`, then three unknown positions, then `ghi`. A useful map must preserve the unknown space between the two pieces. It must also distinguish absence from conflicting copies of a byte.

Read this illustration from left to right by byte position. It is not a timeline, and its widths do not describe how long transmission took.

```text
Illustrative positions:  0 1 2 3 4 5 6 7 8
Recorded evidence:      a b c ? ? ? g h i
Coverage:               █ █ █ ░ ░ ░ █ █ █
Exact ranges:           [0..3) observed
                        [3..6) missing
                        [6..9) observed
```

`[3..6)` includes position 3 and excludes position 6, so it contains exactly three positions: 3, 4 and 5. These are offsets relative to the stream's retained starting sequence. They are different from offsets inside a packet's stored byte vector. Adding a stream offset to its starting sequence, with the wrap-around described below, gives the corresponding TCP sequence number.

A terminal cannot give every byte its own cell in a large stream. Netscope groups neighbouring positions into map cells. A mark shows `▓` if any position conflicts, otherwise `░` if any position is missing, otherwise `█` for observed positions. Conflict takes priority, so one mark can contain both a conflict and a gap while displaying `▓`. Exact counts and the range list preserve that distinction. A solid `█` mark never conceals a missing position. Short spans use one mark per byte rather than stretching a single byte across half the terminal. The legend states whether marks represent individual bytes or groups; the ruler labels byte offsets, not elapsed time.

The stream inspector keeps a range table beside an explanation of the selected evidence. Its range labels include both ends: `3–5` is another way to describe `[3..6)`. Both mean three bytes. The recorded report retains the end-exclusive notation above. Selecting a row highlights the corresponding map marks and places a bracket beneath them; a selected mark can also contain neighbouring positions when the map is compressed. The table gives the exact boundaries.

The map covers the retained span only. It does not claim that the capture contains the entire connection, especially when the beginning was not observed or analysis limits excluded segments.

## Follow a gap to its consequence

Return to `abc???ghi`. You want to know why the later letters are visible but cannot complete the message. Selecting the missing range brings its unknown bytes into the preview and explains what prevents continuous reading. Selecting the final range instead shows `ghi`, alongside the reason it cannot be attached directly to `abc`.

The illustration below shows these linked views with the missing range selected. The arrows connect the selection to its explanation; they do not represent network traffic.

```text
Illustrative byte map:    ███░░░███
                            ^^^
Selected range:           3–5 · 3 bytes · Missing
                             |
                             +--> Preview: ? ? ?
                             |
                             +--> HTTP reading stops before this gap.
                                  Later bytes remain inspectable.
```

Selecting a range connects three things: its location in the stream, its retained bytes, and the consequence for reading the message. The first gap or conflict is selected when the stream opens, so you can begin with the place that prevents a continuous interpretation. The [usage guide](../UI_LAYOUT.md) explains how to move between ranges and scroll the preview.

The preview has text and hexadecimal representations. Hexadecimal writes each byte as two base-sixteen digits: a recorded zero becomes `00`. Unknown bytes appear as `??`, and conflicting positions as `!!`; neither is replaced with a guessed value. Text mode displays printable ASCII bytes directly, uses `\r` and `\n` for line endings, and escapes other bytes. For example, `\x1b` represents one byte whose hexadecimal value is `1b`; it cannot act as a terminal control command. Literal question marks and exclamation marks are also escaped so they cannot be mistaken for missing or conflicting evidence. This is a byte representation, not a decoder for every text encoding.

A range with agreeing observations is still not proof that the receiver accepted those bytes. Nor does a readable beginning prove a complete HTTP message. The full recorded report remains available for the HTTP findings, control flags and capture limitations. The [inspection walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams) gives the keys for trying each view.

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

The amount of retained data also limits the conclusion. If accepting a segment would exceed the permitted span or total storage, Netscope skips it and reports that omission. Earlier accepted segments remain, so record order affects what fits. A skipped segment prevents the observed-end result: the tool must not claim completeness after deliberately leaving evidence out. The [stream reference](../contracts/m8-streams.md) gives the exact limits.

Try changing `def` in the illustration to arrive twice, then to arrive as `dXf`. Predict the agreeing-overlap count and the prefix available to an application parser. Then inspect the supplied gap and conflict examples in the [packet walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams). The reassembly implementation [`reassemble`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L89) and fixed tests [`sequence_span_and_total_memory_budgets_remain_bounded`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L56) preserve the distinctions above.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Sequence numbers and gaps

TCP segments arrive with `abc` at positions 0 to 2 and `ghi` at positions 6 to 8. What does Netscope build, and what does the HTTP parser see?

<details>
<summary>Answer</summary>

TCP sequence numbers give each byte a position in one direction's stream. Netscope places bytes at their positions and keeps a separate state for each one: Missing, Observed or Conflict. Here positions 3 to 5 are Missing, and they are not filled with zeros or squeezed out. HTTP analysis reads only the contiguous observed prefix, `abc`, and stops at the gap. It never joins the pieces into `abcghi`.

**Learn more:** [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning) · [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`ByteState`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L13)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="repeated-bytes-in-a-trace"></a>

[Repeated bytes in a trace](../questions/newcomer.md#repeated-bytes-in-a-trace)

<a id="what-is-the-unit-of-tcp-reassembly"></a>

[What is the unit of TCP reassembly?](../questions/practitioner.md#what-is-the-unit-of-tcp-reassembly)

<a id="which-length-bounds-a-tcp-payload"></a>

[Which length bounds a TCP payload?](../questions/practitioner.md#which-length-bounds-a-tcp-payload)

<a id="how-does-a-segment-find-its-directed-epoch"></a>

[How does a segment find its directed epoch?](../questions/practitioner.md#how-does-a-segment-find-its-directed-epoch)

<a id="what-limits-bound-reassembly"></a>

[What limits bound reassembly?](../questions/practitioner.md#what-limits-bound-reassembly)

<a id="a-reused-port-a-repeated-syn-and-a-late-ack"></a>

[A reused port, a repeated SYN, and a late ACK](../questions/expert.md#a-reused-port-a-repeated-syn-and-a-late-ack)

<a id="a-capture-that-starts-mid-connection"></a>

[A capture that starts mid-connection](../questions/expert.md#a-capture-that-starts-mid-connection)

<a id="a-capture-taken-with-a-96-byte-snap-length"></a>

[A capture taken with a 96-byte snap length](../questions/expert.md#a-capture-taken-with-a-96-byte-snap-length)

<a id="which-copy-wins-when-segments-overlap"></a>

[Which copy wins when segments overlap](../questions/expert.md#which-copy-wins-when-segments-overlap)

<a id="one-far-away-segment-pushes-out-a-stream"></a>

[One far-away segment pushes out a stream](../questions/expert.md#one-far-away-segment-pushes-out-a-stream)

<a id="a-keep-alive-probe-that-looks-like-an-attack"></a>

[A keep-alive probe that looks like an attack](../questions/expert.md#a-keep-alive-probe-that-looks-like-an-attack)

</details>
