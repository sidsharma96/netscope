# What does HTTP/2 tell us?

Two requests for books overlap on the timeline. Both carry the label HTTP/2. Does that mean they shared one connection? Did the browser send parts of both responses at the same time? Would HTTP/3 have made them faster?

The version label gives us a useful starting point, but those questions ask for more than a name. We need to separate the rules a protocol supports from the events this recording actually contains. Once that distinction is clear, we can decide how to recognize labels in Rust without silently inventing an answer.

## A message and the way it travels

An HTTP request expresses an operation, such as GET, on a resource. Its headers provide additional information, and it may carry a body. A response supplies a status, headers, and possibly a body. Those ideas remain useful across HTTP versions.

A protocol also needs a way to represent those messages as bytes. HTTP/1.1 uses a textual opening line and headers, followed by an optional body. A textual header does not require a textual body: an image can still be sent as binary data. The [HTTP/1.1 message format](https://www.rfc-editor.org/rfc/rfc9112.html#name-message-format) specifies that arrangement.

The transport is the mechanism that carries those bytes between endpoints. For HTTP/2, that transport is TCP, which delivers an ordered byte stream. HTTP/2 divides information into frames, bounded pieces with their own control information. Streams identify exchanges within a connection, allowing frames from different exchanges to be interleaved. This sharing is called multiplexing. [The HTTP/2 overview](https://www.rfc-editor.org/rfc/rfc9113.html#name-http2-protocol-overview) describes the design.

The drawing below shows only the idea of interleaving. A and B stand for two exchanges, and each box stands for a piece associated with one of them. The boxes do not represent measured packets, exact frame sizes, or captured timing.

```text
Illustration · possible HTTP/2 interleaving on one connection

Exchange A:    [A1]             [A2]
Exchange B:           [B1]             [B2]

One connection: [A1] → [B1] → [A2] → [B2]

A version label says this arrangement is supported.
It does not say this arrangement occurred in our capture.
```

HTTP/3 carries HTTP over QUIC, a transport with streams. Its rules differ from HTTP/2's use of TCP; changing the transport does not change the basic purpose of GET or a response status. The [HTTP/3 overview](https://www.rfc-editor.org/rfc/rfc9114.html#name-overview-of-http3) introduces that mapping.

The important question for this inspector comes next: which of these details did the exporter retain?

## A capability is not a measurement

Our imported HAR fields describe requests, responses, and reported timings. They do not contain a trace of the individual frames drawn above. They also do not give us the stream identifiers needed to reconstruct that picture. If two requests overlap, they could have used separate connections. The overlap alone does not settle it.

Similarly, a small reported header size is not enough to measure compression savings. To claim savings, we would need suitable evidence about both the encoded representation and the baseline we compare it with. A number without that context cannot supply the missing comparison.

A version label also does not tell us how much faster another protocol would have been. That comparison needs an experiment with controlled conditions or additional measurements. Netscope therefore presents general protocol notes beside the recorded labels, with an explicit boundary between the two.

This does not make the label useless. It gives us a vocabulary for asking the next question. We can ask for a connection trace, look for missing exporter fields, or design a separate experiment. We simply avoid presenting the answer before we have that evidence.

## Keep both labels when they differ

The file has one protocol field on the request and another on the response. If one says HTTP/1.1 and the other says HTTP/3, choosing the newer value would throw away the disagreement. Calling it an upgrade would add a story that these two strings do not establish.

Netscope shows both originals, recognizes each independently, and labels disagreement. It also accepts unrecognized nonempty text as captured data. A label such as HTTP/4 stays visible even though this version of the inspector does not explain it. “Unrecognized” describes our recognition policy; it does not declare that the exporter is wrong.

Sometimes different strings have the same classification. Netscope explicitly recognizes HTTP/2, HTTP/2.0, and h2 as HTTP/2 labels, and similarly recognizes three documented spellings for HTTP/3. Those are supported spellings in this tool, not permission to treat any text beginning with “HTTP/2” as the same value.

## Read text without taking ownership of it

The recognition function needs to look at a label, but it does not need to own or modify the text. Rust's `&str` is a shared reference to a string slice: a view of UTF-8 text whose bytes are kept alive elsewhere. A `String`, by contrast, owns a text buffer that can grow. We met that ownership in the [storage chapter](03-storing-requests.md).

Here is a small recognizer for just the HTTP/2 spellings. It returns a Boolean, the true-or-false value from the [previous chapter](14-recognizing-repeats.md). The full application keeps a separate classification for each recognized version and for unrecognized labels.

```rust
fn is_http2(label: &str) -> bool {
    match label {
        "HTTP/2" | "HTTP/2.0" | "h2" => true,
        _ => false,
    }
}

let captured = String::from("h2");
let recognized = is_http2(captured.as_str());
assert_eq!(recognized, true);
assert_eq!(captured, "h2");
assert_eq!(is_http2("HTTP/20"), false);
assert_eq!(is_http2("HTTP/2 "), false);
```

`captured.as_str()` lends a view of the existing string to the function. It does not copy the buffer or transfer its ownership. The caller can still read `captured` afterward. The borrowed view is valid while the owning string remains alive, and Rust checks that borrowed access does not outlast its data.

Inside `match`, each quoted value is an exact text pattern. The vertical bar, `|`, separates alternative patterns in the same arm: any of those three spellings produces `true`. It is not the Boolean `&&` operator from the previous lesson. The underscore, `_`, accepts every value left over and produces `false`. That final arm also handles future labels the author has never seen.

The last two checks are deliberate. HTTP/20 shares a prefix with HTTP/2, but exact matching does not confuse them. The trailing space in the final example is part of the string, so that spelling does not match either. We could choose a different recognition policy later, but it should be an explicit change rather than an accidental consequence of a broad comparison.

## Try separating the claims

Open `tests/fixtures/har/protocols.har` using the [walkthrough](06-first-milestone.md). In request #0, HTTP/2 and h2 agree on a classification while retaining different original text. In #1, two recognized versions disagree. In #2, one label is unrecognized. Scroll to PROTOCOL NOTES to see each explanation.

For each request, write one sentence about the fields that are present and one question that remains unanswered. “The exporter wrote h2” is a different claim from “these requests shared one TCP connection.” Explain what additional evidence the second sentence would need.

Then predict what the Rust example returns for `H2`. Explain whether recognition changes the owned string. The [label classifier](../../crates/netscope-core/src/protocol.rs) and [protocol contract](../contracts/m2-recorded-protocols.md) show the application's full policy once those distinctions are clear.
