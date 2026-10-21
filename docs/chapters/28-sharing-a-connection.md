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

Suppose the browser labels two requests HTTP/2 and a separate server log says they used the same connection. We can establish connection sharing in that experiment. We still cannot draw the order of individual frames without a record of those frames. Sequential requests on one connection would fit the same observations.

Netscope reads the browser export, not the separate server log. Its inspector therefore keeps the conclusion within what the export contains. Adding a frame decoder would not recover bytes absent from the file; frame inspection needs a different input with suitable connection information. The [compatibility reference](../HAR_COMPATIBILITY.md) records the controlled experiment.

The distinction also explains why inspecting an HTTP/2 label does not give the request sender HTTP/2 support. Reading saved fields and speaking a protocol are different responsibilities. Netscope's live sender uses HTTP/1.1.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Streams and multiplexing

What is an HTTP/2 stream, and how does it differ from an SSE event stream?

<details>
<summary>Answer</summary>

An HTTP/2 stream is one identified request and response exchange inside a connection. Its data travels in frames that carry the stream's identifier, so frames from several streams can be interleaved. That sharing is called multiplexing. An SSE event stream is a text format inside one response body. A single HTTP/2 stream can carry an SSE response with many events, so each event is not a new request.

**Learn more:** [How can two requests share one connection?](../chapters/28-sharing-a-connection.md#how-can-two-requests-share-one-connection) · [A label tells the receiver where a piece belongs](../chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs)

</details>

### Flow control and header compression

What do HTTP/2 flow control and HPACK each do, and can Netscope recover either from a HAR?

<details>
<summary>Answer</summary>

Flow control limits how much body data a sender may send, with separate budgets for each stream and for the whole connection. WINDOW_UPDATE frames raise those budgets. HPACK compresses headers by referring to entries in a static table and a dynamic table that changes as headers are sent. A HAR keeps only the final decoded header names and values and request timings. Netscope therefore cannot reconstruct flow-control waits, table contents, or compression savings.

**Learn more:** [Sharing also needs limits](../chapters/28-sharing-a-connection.md#sharing-also-needs-limits) · [Repeated headers can use remembered values](../chapters/28-sharing-a-connection.md#repeated-headers-can-use-remembered-values)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="what-an-http2-label-proves"></a>

[What an HTTP/2 label proves](../questions/newcomer.md#what-an-http2-label-proves)

<a id="what-do-two-http-version-labels-tell-you"></a>

[What do two HTTP version labels tell you?](../questions/practitioner.md#what-do-two-http-version-labels-tell-you)

</details>
