# What can a trace leave unexplained?

A request starts slowly. A packet trace contains repeated sequence ranges and a [DNS](../GLOSSARY.md#dns) response with several addresses. It is tempting to turn those observations into one story: congestion caused retransmissions, then a load balancer sent the request elsewhere. Each part of that story needs evidence beyond the visible fields.

An inspector can calculate useful facts while leaving their causes unresolved. That distinction becomes more important as the display gains more detail. A precise packet number or timestamp does not make an inferred explanation certain.

## Repeated bytes are visible; their cause may not be

Two segments can carry the same sequence range because the sender transmitted again. A capture setup can also duplicate an observation. Record order can differ from delivery order, and missing recorded packets can reflect the capture process rather than network loss. Netscope counts agreeing overlapping bytes and marks disagreeing bytes; it does not automatically label every repeat as a loss recovery.

An acknowledgment supplies another kind of evidence. Its number can indicate sequence progress when its ACK flag is present, but it does not provide the contents of unseen payload. The recorded window is a receiver-advertised quantity, and the 16-bit header field is not always the byte count. The [window scale option](https://www.rfc-editor.org/rfc/rfc7323#section-2) carries a shift count from 0 to 14, and the receiver of the window multiplies the field by 2 to that power. Each side sends its shift count once, in its SYN segment, and only if both SYN segments carry the option does scaling apply. A capture that starts after the handshake cannot know the shift. Netscope does not parse [TCP](../GLOSSARY.md#tcp) options at all, so it shows the raw field and never claims a byte capacity from it.

## Sender congestion state is not a header field

TCP congestion control manages how much data a sender may have outstanding under changing network conditions. Outstanding data has been sent but not yet acknowledged. The congestion window is sender state. It differs from the receiver-advertised window and is not simply copied into every TCP header.

Slow start is one part of congestion control in which permitted sending can grow as acknowledgments arrive. This illustrative model shows the idea, without claiming actual initial values, exact growth or an observation from our fixtures.

```text
sender's allowance:   small -> larger -> larger again
reason in model:          acknowledgment progress
possible interruption:   congestion/loss indication
```

An actual implementation and path need more careful analysis. Network hardware can perform work that would otherwise happen in the operating system, changing the packet shapes seen at some capture points. Acknowledgment behavior, capture location and missing packets can change what a trace shows. A slow first HTTP request alone cannot establish slow start. The [TCP congestion-control specification](https://www.rfc-editor.org/rfc/rfc5681) explains the sender-side mechanisms; Netscope currently teaches the distinction without reconstructing a congestion window.

## Several addresses do not explain how one was selected

A DNS answer can contain multiple IPv4 or IPv6 addresses. That is recorded name-resolution information. A client can then choose an address, retry another, or receive a changed answer later. The answer alone does not establish a load-balancing algorithm, the full resolver chain, or which result the application used.

DNS names have a wire representation too. Labels describe pieces of a name, and compression pointers refer to earlier bytes in the same message. Netscope bounds pointer traversal and every referenced range. A pointer is a number from external input, not a Rust reference whose validity the compiler has already checked. Converting it into a borrowed slice still needs runtime bounds validation.

The [DNS message specification](https://www.rfc-editor.org/rfc/rfc1035#section-4) describes those fields and compression. Our initial reader displays supported [UDP](../GLOSSARY.md#udp) questions and A/AAAA answer fields. It does not decode every record section or perform DNSSEC validation. The supplied three-address response is a fixed illustration, not a claim about a deployed resolver or balancer.

## Put the limit next to the conclusion

A useful result says both what was calculated and what would be needed for a stronger conclusion. A sequence gap explains why HTTP inspection stopped. An unknown link type explains why there is no IP layer. A recorded STUN candidate explains which structure was seen, while successful connectivity remains unknown. Encrypted bytes explain why an application body is unavailable.

These limits also shape the program. Distinct evidence states prevent a missing measurement from becoming zero, a conflict from becoming accepted content, or an omitted protocol from becoming a fabricated layer. Pure deterministic analysis makes the rule reproducible; it does not make the chosen inference universally correct. Tests should include the cases that make a tempting conclusion fail.

Return to the opening trace. Can you identify an agreeing overlap, a DNS answer and a reported delay separately? What additional observation would let you connect them? That question is the starting point for future analysis. The current [packet and stream contracts](../contracts/m8-streams.md) keep implemented calculations separate from those future explanations.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### A DNS answer with several addresses

**Level:** Newcomer


A DNS response in a PCAP lists three addresses for one name. Does that show a load balancer, and which address did the client use?

<details>
<summary>Answer</summary>

Neither. The answer records the addresses the resolver returned. It does not say how they were chosen, which one the client tried, or whether it retried another. Netscope shows the supported UDP questions and A and AAAA answer fields only. It labels them as answer fields, not proof of resolver choice or load balancing.

**Common wrong answer.** "Several addresses mean load balancing." The answer alone does not establish a balancing algorithm.

**Learn more:** [Several addresses do not explain how one was selected](../chapters/33-interpreting-a-trace.md#several-addresses-do-not-explain-how-one-was-selected) · [Keep the observation's owner clear](../chapters/32-reading-signaling.md#keep-the-observations-owner-clear)

</details>

### What limits bound reassembly?

**Level:** Practitioner


What are the flow, per-stream and total budgets in TCP reassembly? What happens to a packet or segment when each is exceeded, and what are the time and memory costs?

<details>
<summary>Answer</summary>

`FLOW_LIMIT` allows 128 directed epochs, `STREAM_LIMIT` allows a 256 KiB sequence span per epoch, and `TOTAL_STREAM_LIMIT` allows 8 MiB of spans in total. When a packet would need a 129th epoch, it is counted in `unassigned_packets`, and its key is removed from the active map, so later packets in that direction are unassigned too. A segment that would stretch its epoch past 256 KiB, push the total past 8 MiB, or start before the SYN anchor is skipped and counted in `skipped_segments`, and its FIN and RST flags are not recorded. An epoch with any skipped segment never reports an observed end.

Grouping costs O(P log F) for P packets and F epochs. Placing bytes costs time proportional to the payload bytes examined. Memory is about two bytes per retained position (the byte and its state), so at most around 16 MiB for 8 MiB of span, plus the packet bytes themselves. The PCAP limits of 20,000 packets and 32 MiB apply before any of this.

**Learn more:** [How are memory and time bounded in reassembly?](../chapters/34-defending-the-design.md#how-are-memory-and-time-bounded-in-reassembly) · [Sequence observations](../contracts/m8-streams.md#sequence-observations) · [`FLOW_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L6-L10) · [`unassigned_packets`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L102-L107) · [`skipped`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L165-L173)

</details>

### Which copy wins when segments overlap

**Level:** Expert


Operating systems and tools such as Wireshark pick "first copy wins" or "last copy wins" for overlapping TCP segments. Which policy does Netscope use? Does it sort segments by capture timestamp before placing them? Defend the choice.

<details>
<summary>Answer</summary>

Neither policy. The first observed value fills a position. A later copy that agrees increments `repeated_bytes`. A later copy that disagrees turns the position into `Conflict`, and a conflict is permanent. The contiguous prefix stops at a conflict, so no HTTP analysis uses a disputed byte.

Segments are processed in record order, and capture timestamps are never read during reassembly. Positions come from sequence numbers only. Because of that, the state of each position and every agreed value are the same whatever order the records are in. Record order does change three things: the anchor of an epoch without a SYN, which segments survive the span budgets, and the repeat count at a conflicted position.

The defence is that a capture cannot tell which copy the receiver accepted. Checksums are not verified, receiver state is unknown, and real stacks disagree. An intrusion detector that picks a different copy from the target can be evaded. Marking the conflict keeps the ambiguity visible instead of hiding it behind a guess.

**Common wrong answer.** "Netscope sorts by timestamp and keeps the first copy, as most stacks do." It neither sorts by time nor picks a winner.

**Learn more:** [What happens when two segments disagree about the same byte?](../chapters/34-defending-the-design.md#what-happens-when-two-segments-disagree-about-the-same-byte) · [`ByteState::Conflict`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L223-L230)

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

### Limitations to disclose for incident work

**Level:** Expert


A security engineer is evaluating Netscope for incident work. Name the five most important limitations you would disclose, and say for each whether it is a deliberate scope choice or a gap.

<details>
<summary>Answer</summary>

Any five of these, each labelled correctly, make a good answer.

Scope: only classic PCAP is read. PCAPNG is rejected, and link types outside 0, 1, 101, 113, 228, 229 and 276 show "Link type unsupported". Scope: TLS is neither parsed nor decrypted, so HTTPS in a packet capture is opaque. Scope: IPv4 fragments are not reassembled, IPv6 extension headers are not walked, and checksums are not verified. Scope: wire HTTP is HTTP/1.1 with GET and POST requests only, and the two directions are not paired.

Gap: a response to HEAD is misframed, and port reuse without a captured SYN merges into the old epoch. Gap: replay trusts only the bundled root certificates and ignores proxies. Gap: saved drafts keep URL query tokens and unlisted secret headers, and the parent directory is not synced. Gap: terminal escaping misses bidi and zero-width characters. Gap: retry matching can be quadratic, and it misses real retries that change one header.

Scope: WebSocket support covers handshake header shape only, and SSE in a HAR has no timing. Scope: the import and reassembly budgets mean a large capture is analysed in part, and the views say so.

**Learn more:** [Known gaps and pitfalls](../chapters/34-defending-the-design.md#known-gaps-and-pitfalls) · [Put the limit next to the conclusion](../chapters/33-interpreting-a-trace.md#put-the-limit-next-to-the-conclusion) · [`layers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L171-L236)

</details>
