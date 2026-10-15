# What does a connection proposal tell us?

A page receives text describing how a call might exchange audio. The text contains an address, a media description and fields related to establishing a connection. Does that mean audio flowed successfully? No: a proposal and the conversation it proposes are different observations.

Signaling is information applications exchange to arrange communication. [SDP](../GLOSSARY.md#sdp), the Session Description Protocol, supplies a text format for describing aspects of a session. It does not itself transmit the audio or establish that the proposed addresses were reachable. Signaling can travel through HTTP, WebSocket or an application-specific channel; its transport is not a universal WebRTC requirement. WebRTC is a collection of browser facilities and protocols for real-time communication.

## Recognize a description without inventing its outcome

An SDP description has named line fields. A version line begins with `v=`, an origin line with `o=`, and a media description with `m=`. In relevant applications, `a=` attribute lines may include an [ICE](../GLOSSARY.md#ice) username fragment or a [DTLS](../GLOSSARY.md#dtls) fingerprint. ICE is a process for checking possible connectivity paths. DTLS is the datagram form of Transport Layer Security. A DTLS fingerprint describes an expected certificate identity for a secure datagram connection; seeing its text does not perform that identity check.

The following illustration separates a description, a connectivity check and media. Its arrows show relationships, not an observed packet sequence.

```text
session description -> candidate connection information
                            |
                     connectivity checks
                            |
                     possible secure media
```

Reading the first box cannot establish that the lower steps succeeded. Even a successful connectivity check would not prove what audio or video content later flowed. The [SDP specification](https://www.rfc-editor.org/rfc/rfc8866), [ICE specification](https://www.rfc-editor.org/rfc/rfc8445), and [fingerprint rules](https://www.rfc-editor.org/rfc/rfc8122) define different parts of that arrangement.

Netscope recognizes a limited, visible rule: a complete, uncompressed `application/sdp` body whose first line is `v=0` and which contains origin, session-name, timing and media fields. It reports SDP-like text. ICE-username and fingerprint markers produce a more specific explanation, but do not turn that rule into full SDP validation or proof of WebRTC. The supplied description is illustrative and its fingerprint is synthetic.

The rule does not search arbitrary compressed bytes or pretend to decrypt TLS. It also does not extract SDP from every possible JSON object wrapping that text. A field absent from the supported representation remains unknown, rather than proving that the application used no signaling.

## A datagram has its own boundary

[UDP](../GLOSSARY.md#udp) sends [datagrams](../GLOSSARY.md#datagram), individual messages with recorded lengths. It does not supply TCP's ordered byte-stream mechanism. Joining two UDP payloads because their endpoints match would manufacture a message that neither datagram contained. Netscope checks each complete supported datagram separately.

[STUN](../GLOSSARY.md#stun) is a protocol used in several connectivity-related situations. Netscope recognizes a candidate from its header signature, declared aligned length and bounded attribute framing. An attribute is a typed part of the message after its header. This verifies only a narrow structure; it does not validate authentication or infer successful ICE. A STUN-shaped request can be recorded without any response. The [STUN standard](https://www.rfc-editor.org/rfc/rfc8489#section-6) defines the complete protocol.

This is another reason to give an analysis more than a yes/no result. “Recognized candidate,” “cut datagram,” “unsupported application format” and “successful connection” answer different questions. The last one needs evidence the current recognizer does not provide.

## Keep the observation's owner clear

The packet owns the original bytes. DNS/STUN recognition borrows its checked UDP payload for a short inspection. SDP recognition instead examines application bytes produced after supported HTTP framing is removed. These inputs have different provenance even if both analyses return human-readable descriptions.

Owning a derived HTTP body is reasonable because removing chunk boundaries creates a new sequence of bytes. Borrowing a UDP payload is reasonable because it is already one contiguous range in an owned record. The choice follows what data exists, rather than a rule that everything should always be copied or always borrowed.

Try finding an ICE username and a fingerprint in the supplied stream. State the recognition rule, then list which outcomes remain unobserved. Inspect the separate STUN candidate too. The [local walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams), datagram recognition [`inspect_datagram`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L113), and wire-body recognition [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140) keep the source and consequence of each observation visible.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### SDP and STUN

**Level:** Newcomer


Netscope reports SDP-like text with an ICE username fragment and a STUN candidate in the same capture. Did the call's audio flow?

<details>
<summary>Answer</summary>

The capture does not show that. SDP is a text description that proposes a session. STUN messages help check possible connection paths. Netscope recognises an SDP body by a narrow rule (`application/sdp`, a first line of `v=0`, and the expected fields) and a STUN datagram by its header signature and framing. Neither proves that a connectivity check succeeded or that media flowed.

**Learn more:** [Recognize a description without inventing its outcome](../chapters/32-reading-signaling.md#recognize-a-description-without-inventing-its-outcome) · [A datagram has its own boundary](../chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary) · [`inspect_datagram`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L113)

</details>

### What stops a DNS compression loop

**Level:** Expert


How does the DNS name reader avoid compression-pointer loops and unbounded work? When is a UDP datagram treated as DNS or STUN, and what is out of scope?

<details>
<summary>Answer</summary>

The name reader requires each compression pointer to target an offset earlier than the pointer itself. That rule blocks forward jumps, but it does not stop a loop. A one-byte label at offset 12 followed by a pointer at offset 14 that targets 12 satisfies the rule and repeats forever under it alone. What ends that loop is the 255-byte name limit: each pass adds a label of at least 2 encoded bytes, so the name fails after 127 passes. A chain of pointers with no labels cannot loop, because every target is earlier than its pointer, and a separate count stops it after 127 jumps with "DNS name traversal limit". Rejection of label types 0x40 and 0x80 and checked reads for every slice bound the rest of the work. Because labels and jumps are counted separately, a legal name of 127 one-byte labels is accepted.

The header is limited to 16 questions and 32 answers, and a nonzero opcode is rejected. DNS is tried only when the UDP source or destination port is 53, and only for datagrams retained in full. STUN is recognised by two zero leading bits, the magic cookie 0x2112A442, a declared length that is a multiple of 4 and equals the payload minus 20, and at most 64 attributes.

DNS over TCP, mDNS on port 5353, DoH, DoT, the authority and additional sections, and EDNS are out of scope.

**Common wrong answer.** "Pointers must point backwards, so loops are impossible." A backward pointer can still return to a label that leads back to the same pointer; the 255-byte name limit is what terminates it.

**Learn more:** [Can a malicious DNS packet hang the parser?](../chapters/34-defending-the-design.md#can-a-malicious-dns-packet-hang-the-parser) · [A datagram has its own boundary](../chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary) · [`name`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L7-L56) · [`inspect_datagram`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L113-L126)

</details>

### A DNS answer with several addresses

**Level:** Newcomer


A DNS response in a PCAP lists three addresses for one name. Does that show a load balancer, and which address did the client use?

<details>
<summary>Answer</summary>

Neither. The answer records the addresses the resolver returned. It does not say how they were chosen, which one the client tried, or whether it retried another. Netscope shows the supported UDP questions and A and AAAA answer fields only. It labels them as answer fields, not proof of resolver choice or load balancing.

**Common wrong answer.** "Several addresses mean load balancing." The answer alone does not establish a balancing algorithm.

**Learn more:** [Several addresses do not explain how one was selected](../chapters/33-interpreting-a-trace.md#several-addresses-do-not-explain-how-one-was-selected) · [Keep the observation's owner clear](../chapters/32-reading-signaling.md#keep-the-observations-owner-clear)

</details>
