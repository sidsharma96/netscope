# What can a trace leave unexplained?

A request starts slowly. A packet trace contains repeated sequence ranges and a DNS response with several addresses. It is tempting to turn those observations into one story: congestion caused retransmissions, then a load balancer sent the request elsewhere. Each part of that story needs evidence beyond the visible fields.

An inspector can calculate useful facts while leaving their causes unresolved. That distinction becomes more important as the display gains more detail. A precise packet number or timestamp does not make an inferred explanation certain.

## Repeated bytes are visible; their cause may not be

Two segments can carry the same sequence range because the sender transmitted again. A capture setup can also duplicate an observation. Record order can differ from delivery order, and missing recorded packets can reflect the capture process rather than network loss. Netscope counts agreeing overlapping bytes and marks disagreeing bytes; it does not automatically label every repeat as a loss recovery.

An acknowledgment supplies another kind of evidence. Its number can indicate sequence progress when its ACK flag is present, but it does not provide the contents of unseen payload. The recorded window is a receiver-advertised quantity; TCP can negotiate a multiplier for that field, so the raw number is not always the final advertised byte capacity.

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

The [DNS message specification](https://www.rfc-editor.org/rfc/rfc1035#section-4) describes those fields and compression. Our initial reader displays supported UDP questions and A/AAAA answer fields. It does not decode every record section or perform DNSSEC validation. The supplied three-address response is a fixed illustration, not a claim about a deployed resolver or balancer.

## Put the limit next to the conclusion

A useful result says both what was calculated and what would be needed for a stronger conclusion. A sequence gap explains why HTTP inspection stopped. An unknown link type explains why there is no IP layer. A recorded STUN candidate explains which structure was seen, while successful connectivity remains unknown. Encrypted bytes explain why an application body is unavailable.

These limits also shape the program. Distinct evidence states prevent a missing measurement from becoming zero, a conflict from becoming accepted content, or an omitted protocol from becoming a fabricated layer. Pure deterministic analysis makes the rule reproducible; it does not make the chosen inference universally correct. Tests should include the cases that make a tempting conclusion fail.

Return to the opening trace. Can you identify an agreeing overlap, a DNS answer and a reported delay separately? What additional observation would let you connect them? That question is the starting point for future analysis. The current [packet and stream contracts](../contracts/m8-streams.md) keep implemented calculations separate from those future explanations.
