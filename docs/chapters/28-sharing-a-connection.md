# How can two requests share one connection?

A page asks for a book list and a cover image. The list is ready quickly; the image takes longer. If both responses must travel one after the other, the large image can make unrelated work wait. [HTTP/2](../GLOSSARY.md#http2) gives the connection a way to distinguish pieces belonging to different exchanges and carry them interleaved.

A **[stream](../GLOSSARY.md#stream)** is an identified exchange within an HTTP/2 connection. It is different from an SSE event stream: one names a protocol channel, the other names an application text format. A single HTTP/2 stream can carry an SSE response containing many events. Keeping the two meanings separate avoids counting each event as a new HTTP request.

## A label tells the receiver where a piece belongs

HTTP/2 sends structured pieces called [frames](../GLOSSARY.md#frame). A frame includes a type and, where applicable, a stream identifier. HEADERS frames carry encoded header information; DATA frames carry body bytes. The receiver uses the identifier to associate each piece with the correct exchange.

This illustration shows possible interleaving on one connection. Read left to right. The labels belong to streams, not packets; the sequence and identifiers are invented rather than reconstructed from our browser exports.

```text
one connection -> HEADERS A | HEADERS B | DATA B | DATA A | DATA B
                    |           |          |        |        |
book list A <-------+-----------|----------|--------+        |
cover image B <-----------------+----------+-----------------+
```

Stream A can finish before stream B even though pieces of B arrived first. This sharing is called **[multiplexing](../GLOSSARY.md#multiplexing)**. It removes the requirement to deliver whole responses in request order, but does not promise that every request is fast. The connection still has finite bandwidth and the sender still chooses what to send next. The [HTTP/2 standard](https://www.rfc-editor.org/rfc/rfc9113#section-5) defines streams and their relationship to the connection.

HTTP/2 normally runs over TCP, which delivers bytes in order. If a needed byte is lost, later bytes can wait for its replacement even when they contain another HTTP/2 stream. Interleaving at the HTTP layer therefore does not eliminate all waiting caused by the transport. The distinction matters when explaining an observed delay: a protocol capability is not a diagnosis.

## Sharing also needs limits

A receiver cannot necessarily consume incoming bodies as quickly as a sender can produce them. HTTP/2 **[flow control](../GLOSSARY.md#flow-control)** tells the sender how much additional DATA it may send. Budgets apply both to a stream and to the connection; WINDOW_UPDATE frames increase the permitted amount. This protects receiver capacity, rather than setting a desired requests-per-second rate.

Imagine the connection budget is exhausted. A stream with room in its own budget still cannot send more DATA until the connection has room too. Conversely, room on the connection does not override an exhausted stream budget. This two-level rule gives us a prediction without needing to know a particular implementation's buffer layout. It concerns DATA, not every frame type. The [flow-control rules](https://www.rfc-editor.org/rfc/rfc9113#section-5.2) provide the exact constraints.

A HAR usually summarizes a request and response after these mechanisms have done their work. Its timing fields do not report each budget change. Two overlapping request intervals cannot establish which frames were interleaved, which stream waited for credit, or whether both requests used the same connection.

## Repeated headers can use remembered values

Two requests may repeat the same authority, content preferences, and other header fields. Sending the full text each time wastes space. HTTP/2 uses **[HPACK](../GLOSSARY.md#hpack)**, a representation for header fields that can refer to table entries instead of repeating their complete names and values.

There is a predefined static table and a changing dynamic table. The encoder, which writes the representation, can add a field to the dynamic table; the decoder, which reads it, makes the corresponding change. A later reference is meaningful only while the correct table state is available. Request and response directions keep independent dynamic state. The [HPACK specification](https://www.rfc-editor.org/rfc/rfc7541#section-2) explains those tables and contexts.

Here is an analogy, not HPACK byte encoding. Suppose the sender first writes “remember entry 7 as `accept: text/event-stream`,” then later writes “use entry 7.” A reader starting with the second instruction cannot recover the text from that reference alone. Actual HPACK indexes and encodings follow their own specified rules; seven in this example is only an illustrative label.

That explains why a decoded header list is insufficient to reconstruct its compressed bytes. The exporter has already expanded the representation and may have omitted its history. Netscope can show the resulting names and values, but cannot derive HPACK table contents or compression savings from those fields. Gzip or Brotli compression of a response body is a separate mechanism and supplies no such header evidence.

## What our recordings establish

Our frozen Chrome and Firefox HTTPS exports identify the requests as HTTP/2. A separate log from the controlled server reports negotiated H2 and associates each browser's sequence with one server-local session. Together these establish the protocol and connection reuse in that experiment. The requests were triggered sequentially; they do not demonstrate concurrent interleaving.

The HAR importer does not consume that server log. Its connection labels, reported sizes, and timing values remain exporter observations. Netscope does not convert local server session numbers into HTTP/2 stream IDs or invent frames. The [compatibility record](../HAR_COMPATIBILITY.md) links the recordings, independent observations, and reproduction method.

This evidence boundary is a design choice as well as a teaching rule. Adding a from-scratch HTTP/2 decoder would not recover bytes absent from the input. Genuine frame inspection would need suitable wire evidence, decryption where necessary, and connection state. Existing protocol libraries can supply live transport behaviour, but adopting one would still require a new review and testing contract. Netscope's current live sender deliberately remains HTTP/1.1.

Try the two requests from the opening again. If the HAR shows overlapping times and both say HTTP/2, can you prove that they shared a connection? If the server log supplies that connection fact, can you also prove the interleaving sequence? The first answer needs connection evidence; the second needs frame evidence. Learning to ask for the missing observation is more useful than drawing a convincing sequence from guesses.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### What an HTTP/2 label proves

**Level:** Newcomer


Two overlapping requests are both labelled HTTP/2. Did they share one connection?

<details>
<summary>Answer</summary>

The label cannot tell you. HTTP/2 supports carrying several exchanges on one connection, but a version label describes a capability, not an event. A HAR does not contain the frames or stream identifiers that would show the sharing. Overlapping times are consistent with one shared connection and also with two separate ones. Proving it needs connection evidence, such as a server log.

**Common wrong answer.** "HTTP/2 always multiplexes, so they shared a connection."

**Learn more:** [A capability is not a measurement](../chapters/15-reading-protocol-labels.md#a-capability-is-not-a-measurement) · [What our recordings establish](../chapters/28-sharing-a-connection.md#what-our-recordings-establish)

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

### Streams and multiplexing

**Level:** Newcomer


What is an HTTP/2 stream, and how does it differ from an SSE event stream?

<details>
<summary>Answer</summary>

An HTTP/2 stream is one identified request and response exchange inside a connection. Its data travels in frames that carry the stream's identifier, so frames from several streams can be interleaved. That sharing is called multiplexing. An SSE event stream is a text format inside one response body. A single HTTP/2 stream can carry an SSE response with many events, so each event is not a new request.

**Learn more:** [How can two requests share one connection?](../chapters/28-sharing-a-connection.md#how-can-two-requests-share-one-connection) · [A label tells the receiver where a piece belongs](../chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs)

</details>

### Flow control and header compression

**Level:** Newcomer


What do HTTP/2 flow control and HPACK each do, and can Netscope recover either from a HAR?

<details>
<summary>Answer</summary>

Flow control limits how much body data a sender may send, with separate budgets for each stream and for the whole connection. WINDOW_UPDATE frames raise those budgets. HPACK compresses headers by referring to entries in a static table and a dynamic table that changes as headers are sent. A HAR keeps only the final decoded header names and values and request timings. Netscope therefore cannot reconstruct flow-control waits, table contents, or compression savings.

**Learn more:** [Sharing also needs limits](../chapters/28-sharing-a-connection.md#sharing-also-needs-limits) · [Repeated headers can use remembered values](../chapters/28-sharing-a-connection.md#repeated-headers-can-use-remembered-values)

</details>
