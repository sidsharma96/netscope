# What does a connection proposal tell us?

A page receives text describing how a call might exchange audio. The text contains an address, a media description and fields related to establishing a connection. Does that mean audio flowed successfully? No: a proposal and the conversation it proposes are different observations.

Signaling is information applications exchange to arrange communication. SDP, the Session Description Protocol, supplies a text format for describing aspects of a session. It does not itself transmit the audio or establish that the proposed addresses were reachable. Signaling can travel through HTTP, WebSocket or an application-specific channel; its transport is not a universal WebRTC requirement. WebRTC is a collection of browser facilities and protocols for real-time communication.

## Recognize a description without inventing its outcome

An SDP description has named line fields. A version line begins with `v=`, an origin line with `o=`, and a media description with `m=`. In relevant applications, `a=` attribute lines may include an ICE username fragment or a DTLS fingerprint. ICE is a process for checking possible connectivity paths. DTLS is the datagram form of Transport Layer Security. A DTLS fingerprint describes an expected certificate identity for a secure datagram connection; seeing its text does not perform that identity check.

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

UDP sends datagrams, individual messages with recorded lengths. It does not supply TCP's ordered byte-stream mechanism. Joining two UDP payloads because their endpoints match would manufacture a message that neither datagram contained. Netscope checks each complete supported datagram separately.

STUN is a protocol used in several connectivity-related situations. Netscope recognizes a candidate from its header signature, declared aligned length and bounded attribute framing. An attribute is a typed part of the message after its header. This verifies only a narrow structure; it does not validate authentication or infer successful ICE. A STUN-shaped request can be recorded without any response. The [STUN standard](https://www.rfc-editor.org/rfc/rfc8489#section-6) defines the complete protocol.

This is another reason to give an analysis more than a yes/no result. “Recognized candidate,” “cut datagram,” “unsupported application format” and “successful connection” answer different questions. The last one needs evidence the current recognizer does not provide.

## Keep the observation's owner clear

The packet owns the original bytes. DNS/STUN recognition borrows its checked UDP payload for a short inspection. SDP recognition instead examines application bytes produced after supported HTTP framing is removed. These inputs have different provenance even if both analyses return human-readable descriptions.

Owning a derived HTTP body is reasonable because removing chunk boundaries creates a new sequence of bytes. Borrowing a UDP payload is reasonable because it is already one contiguous range in an owned record. The choice follows what data exists, rather than a rule that everything should always be copied or always borrowed.

Try finding an ICE username and a fingerprint in the supplied stream. State the recognition rule, then list which outcomes remain unobserved. Inspect the separate STUN candidate too. The [local walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams), [datagram recognition](../../crates/netscope-core/src/datagrams.rs), and [wire-body recognition](../../crates/netscope-core/src/wire_http.rs) keep the source and consequence of each observation visible.
