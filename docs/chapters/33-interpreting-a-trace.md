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

Try these questions using the explanation above, then open the answers.

### A DNS answer with several addresses

A DNS response in a PCAP lists three addresses for one name. Does that show a load balancer, and which address did the client use?

<details>
<summary>Answer</summary>

Neither. The answer records the addresses the resolver returned. It does not say how they were chosen, which one the client tried, or whether it retried another. Netscope shows the supported UDP questions and A and AAAA answer fields only. It labels them as answer fields, not proof of resolver choice or load balancing.

**Common wrong answer.** "Several addresses mean load balancing." The answer alone does not establish a balancing algorithm.

**Learn more:** [Several addresses do not explain how one was selected](../chapters/33-interpreting-a-trace.md#several-addresses-do-not-explain-how-one-was-selected) · [Keep the observation's owner clear](../chapters/32-reading-signaling.md#keep-the-observations-owner-clear)

</details>

### Repeated bytes in a trace

Two TCP segments carry the same sequence range with the same bytes. Is that a retransmission after packet loss?

<details>
<summary>Answer</summary>

It might be, but the trace does not say so. The sender may have sent again, or the capture setup may have recorded the same packet twice. Netscope counts agreeing overlaps as repeated bytes and marks positions where copies disagree as conflicts. It does not label a repeat as loss recovery, because the cause is not in the recorded fields.

**Common wrong answer.** "Repeated bytes always mean loss and retransmission."

**Learn more:** [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning)

</details>

### The TCP window field

A TCP header shows a window of 512. Can you read that as "the receiver will accept 512 more bytes"?

<details>
<summary>Answer</summary>

Not in general. With window scaling, the receiver of the window multiplies the 16-bit field by 2 to the power of a shift count from 0 to 14. Each side announces its shift only in its SYN segment. A capture that starts after the handshake cannot know the shift, and Netscope does not parse TCP options at all. It shows the raw field and claims no byte capacity. The window is also not the sender's congestion window.

**Learn more:** [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [Sender congestion state is not a header field](../chapters/33-interpreting-a-trace.md#sender-congestion-state-is-not-a-header-field)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="what-limits-bound-reassembly"></a>

[What limits bound reassembly?](../questions/practitioner.md#what-limits-bound-reassembly)

<a id="which-copy-wins-when-segments-overlap"></a>

[Which copy wins when segments overlap](../questions/expert.md#which-copy-wins-when-segments-overlap)

<a id="limitations-to-disclose-for-incident-work"></a>

[Limitations to disclose for incident work](../questions/expert.md#limitations-to-disclose-for-incident-work)

<a id="a-keep-alive-probe-that-looks-like-an-attack"></a>

[A keep-alive probe that looks like an attack](../questions/expert.md#a-keep-alive-probe-that-looks-like-an-attack)

</details>
