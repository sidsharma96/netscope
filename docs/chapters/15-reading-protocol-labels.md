# What does HTTP/2 tell us?

Two requests for books overlap on the timeline. Both carry the label [HTTP/2](../GLOSSARY.md#http2). Does that mean they shared one connection? Did the browser send parts of both responses at the same time? Would HTTP/3 have made them faster?

The version label gives us a useful starting point, but those questions ask for more than a name. We need to separate the rules a protocol supports from the events this recording actually contains. Once that distinction is clear, we can decide how to recognize labels in Rust without silently inventing an answer.

## A message and the way it travels

An HTTP request expresses an operation, such as GET, on a resource. Its headers provide additional information, and it may carry a body. A response supplies a status, headers, and possibly a body. Those ideas remain useful across HTTP versions.

A protocol also needs a way to represent those messages as bytes. [HTTP/1.1](../GLOSSARY.md#http11) uses a textual opening line and headers, followed by an optional body. A textual header does not require a textual body: an image can still be sent as binary data. The [HTTP/1.1 message format](https://www.rfc-editor.org/rfc/rfc9112.html#name-message-format) specifies that arrangement.

The transport is the mechanism that carries those bytes between endpoints. For HTTP/2, that transport is [TCP](../GLOSSARY.md#tcp), which delivers an ordered byte stream. HTTP/2 divides information into frames, bounded pieces with their own control information. Streams identify exchanges within a connection, allowing frames from different exchanges to be interleaved. This sharing is called [multiplexing](../GLOSSARY.md#multiplexing). [The HTTP/2 overview](https://www.rfc-editor.org/rfc/rfc9113.html#name-http2-protocol-overview) describes the design.

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

## Why the body can be larger than the download

Imagine downloading a book description with a sentence repeated many times. Compression represents repeated content more compactly; decompression restores the original bytes. Gzip and Brotli are two compression formats a server can use for a response body. They are separate from the way HTTP/2 represents headers and divides messages into frames.

The browser can decompress a response before saving its content in a HAR. The captured headers still describe the response that arrived, so `Content-Encoding: gzip` may appear beside ordinary readable text. Applying gzip decompression to that text again would be a mistake: the browser has already performed that step.

Read the following illustration from left to right. Each arrow changes the representation of the same content; the sizes are invented to make the relationship clear.

```text
Illustration · one body, two representations

Server                         Browser                         HAR
1,000 bytes of text
       │ compress
       ▼
  80 compressed bytes ────────► decompress ──────────────────► 1,000 bytes of text
                               response headers retained ───► Content-Encoding: gzip
```

The downloaded body and the saved content therefore answer different size questions. A recorded content size describes the exported content, while a reported body size is a separate exporter value. Netscope keeps those numbers separate. In our controlled recordings, the browsers did not report body size in the same way, so subtracting it from content size would not give a dependable measure of compression savings. An unavailable size gives us even less basis for that calculation.

Binary content adds another representation. A browser may decompress the received bytes and then write them as base64 so they fit in a JSON string. Base64, introduced in the [import chapter](10-importing-a-har.md), encodes bytes as text; it is not compression. Netscope decodes the HAR's base64 representation once and shows the resulting bytes. It does not apply the response's gzip header to them again.

## When a connection timing is not a measurement

Now suppose a later request has a connection timing of `-1`. HAR 1.2 defines this marker as "does not apply to the current request": a request on an already open connection had no connection setup of its own. It is not a measured duration of zero. If the field is omitted entirely, the record supplies no value at all. A numeric zero remains a reported zero; its precision and collection rules do not establish that no work happened.

Rust's `Timing` enum keeps these cases separate. An enum value selects one named alternative: missing, not applicable (`-1`), or a duration carrying a number. The inspector matches that alternative to choose the display. It can show a not-applicable label without manufacturing a number that would later enter a calculation.

Reusing a connection could explain why a later request has no new connection setup measurement, but the timing field alone cannot prove reuse. Even a reported zero is insufficient. We need separate evidence about which connection handled each request. The [browser compatibility reference](../HAR_COMPATIBILITY.md) keeps our controlled server observations separate from what the HAR inspector can infer.

## Keep both labels when they differ

The file has one protocol field on the request and another on the response. If one says HTTP/1.1 and the other says HTTP/3, choosing the newer value would throw away the disagreement. Calling it an upgrade would add a story that these two strings do not establish.

Netscope shows both originals, recognizes each independently, and labels disagreement. It also accepts unrecognized nonempty text as captured data. A label such as HTTP/4 stays visible even though this version of the inspector does not explain it. “Unrecognized” describes our recognition policy; it does not declare that the exporter is wrong.

Sometimes different strings have the same classification. Netscope explicitly recognizes HTTP/2, HTTP/2.0, http/2.0, and h2 as HTTP/2 labels, and similarly recognizes three documented spellings for HTTP/3. Those are supported spellings in this tool, not permission to treat any text beginning with “HTTP/2” as the same value.

## Read text without taking ownership of it

The recognition function needs to look at a label, but it does not need to own or modify the text. Rust's `&str` is a shared reference to a string slice: a view of UTF-8 text whose bytes are kept alive elsewhere. A `String`, by contrast, owns a text buffer that can grow. We met that ownership in the [storage chapter](03-storing-requests.md).

Here is a small recognizer for the supported HTTP/2 spellings. It returns a Boolean, the true-or-false value from the [previous chapter](14-recognizing-repeats.md). The full application keeps a separate classification for each recognized version and for unrecognized labels.

```rust
fn is_http2(label: &str) -> bool {
    match label {
        "HTTP/2" | "HTTP/2.0" | "http/2.0" | "h2" => true,
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

Inside `match`, each quoted value is an exact text pattern. The vertical bar, `|`, separates alternative patterns in the same arm: any of those four spellings produces `true`. It is not the Boolean `&&` operator from the previous lesson. The underscore, `_`, accepts every value left over and produces `false`. That final arm also handles future labels the author has never seen.

The last two checks are deliberate. HTTP/20 shares a prefix with HTTP/2, but exact matching does not confuse them. The trailing space in the final example is part of the string, so that spelling does not match either. We could choose a different recognition policy later, but it should be an explicit change rather than an accidental consequence of a broad comparison.

## Try separating the claims

Open `tests/fixtures/har/protocols.har` using the [walkthrough](06-first-milestone.md). In request #0, HTTP/2 and h2 agree on a classification while retaining different original text. In #1, two recognized versions disagree. In #2, one label is unrecognized. Scroll to PROTOCOL NOTES to see each explanation.

For each request, write one sentence about the fields that are present and one question that remains unanswered. “The exporter wrote h2” is a different claim from “these requests shared one TCP connection.” Explain what additional evidence the second sentence would need.

Then predict what the Rust example returns for `H2`. Explain whether recognition changes the owned string. The label classifier [`RecordedProtocol`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/protocol.rs#L3) and [protocol contract](../contracts/m2-recorded-protocols.md) show the application's full policy once those distinctions are clear.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Missing, minus one, and zero

**Level:** Newcomer


A HAR's DNS timing can be absent, `-1`, or `0`. What does each one mean, and how does Netscope keep them apart?

<details>
<summary>Answer</summary>

An absent field means the exporter recorded nothing. HAR 1.2 defines `-1` for an optional timing as "does not apply to the current request", for example DNS on a connection the browser had already opened. A `0` is a reported duration of zero at the exporter's precision. Netscope stores these as different variants of its `Timing` enum: `Missing`, `Unavailable` (shown as `Not applicable (-1)`), and `Milliseconds(0.0)`. None of them is turned into another.

**Common wrong answer.** "`-1` means the value is unknown, so treat it like zero." It means the stage did not happen for this request, and adding it as zero would invent a measurement.

**Learn more:** [Missing time is different from zero](../chapters/02-reading-a-capture.md#missing-time-is-different-from-zero) · [A present number still needs a meaning](../chapters/09-options-results-and-matching.md#a-present-number-still-needs-a-meaning) · [`Timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L11)

</details>

### What do two HTTP version labels tell you?

**Level:** Practitioner


A request is labelled `HTTP/1.1` and its response `h2`. Another entry has both labels `HTTP/2.0`. What does Netscope show for each, and why does it not conclude that the second entry shared a connection with other requests?

<details>
<summary>Answer</summary>

`RecordedProtocol::from_label` recognises a fixed set of spellings and maps everything else to "Unrecognized label". `h2` and `HTTP/2.0` both become HTTP/2. The protocol notes show each raw label beside its classification. For the first entry, the two recognised labels differ, so Netscope says "Labels disagree; no upgrade, downgrade, or cause is inferred". For the second, it says the labels agree on a recognised version and calls that exporter evidence. If either label is unrecognised, comparison is unavailable.

The general explanation that follows, such as "HTTP/2 can interleave frames from different streams on one TCP connection", describes what the protocol allows, not what happened. A HAR records no frames and no stream identifiers, and overlapping requests could have used separate connections. Claiming that requests shared a connection would need connection evidence the file does not contain.

**Learn more:** [A capability is not a measurement](../chapters/15-reading-protocol-labels.md#a-capability-is-not-a-measurement) · [Keep both labels when they differ](../chapters/15-reading-protocol-labels.md#keep-both-labels-when-they-differ) · [What our recordings establish](../chapters/28-sharing-a-connection.md#what-our-recordings-establish) · [`from_label`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/protocol.rs#L17-L25) · [`protocol_lines`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L1277-L1303)

</details>

### What an HTTP/2 label proves

**Level:** Newcomer


Two overlapping requests are both labelled HTTP/2. Did they share one connection?

<details>
<summary>Answer</summary>

The label cannot tell you. HTTP/2 supports carrying several exchanges on one connection, but a version label describes a capability, not an event. A HAR does not contain the frames or stream identifiers that would show the sharing. Overlapping times are consistent with one shared connection and also with two separate ones. Proving it needs connection evidence, such as a server log.

**Common wrong answer.** "HTTP/2 always multiplexes, so they shared a connection."

**Learn more:** [A capability is not a measurement](../chapters/15-reading-protocol-labels.md#a-capability-is-not-a-measurement) · [What our recordings establish](../chapters/28-sharing-a-connection.md#what-our-recordings-establish)

</details>

### Two protocol labels that disagree

**Level:** Newcomer


A request's protocol says HTTP/1.1 and its response says HTTP/3. What does Netscope show?

<details>
<summary>Answer</summary>

It shows both original strings, classifies each one on its own, and labels the disagreement. It does not pick the newer value or call it an upgrade, since the two strings do not establish that story. Recognition is exact: `HTTP/2`, `HTTP/2.0`, `http/2.0` and `h2` count as HTTP/2, while unrecognised text such as `HTTP/4` stays visible as captured data.

**Learn more:** [Keep both labels when they differ](../chapters/15-reading-protocol-labels.md#keep-both-labels-when-they-differ) · [`RecordedProtocol`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/protocol.rs#L3)

</details>

### Readable text with a gzip header

**Level:** Newcomer


A captured response has `Content-Encoding: gzip`, but its body in the HAR is plain readable text. Is the file broken?

<details>
<summary>Answer</summary>

No. The browser received compressed bytes, decompressed them, and saved the readable result. The header still describes the response as it arrived. Netscope does not apply gzip again, because the browser already did that step. Base64 in a HAR is a separate storage encoding, and Netscope decodes it once. For the same reason, replay comparison refuses to judge body bytes when either side reports a non-identity content encoding.

**Learn more:** [Why the body can be larger than the download](../chapters/15-reading-protocol-labels.md#why-the-body-can-be-larger-than-the-download) · [Bytes need a common meaning](../chapters/18-comparing-responses.md#bytes-need-a-common-meaning)

</details>
