# How can two requests share one connection?

A page asks for a book list and a cover image. The list is ready quickly; the image takes longer. If both responses must travel one after the other, the large image can make unrelated work wait. HTTP/2 gives the connection a way to distinguish pieces belonging to different exchanges and carry them interleaved.

A **stream** is an identified exchange within an HTTP/2 connection. It is different from an SSE event stream: one names a protocol channel, the other names an application text format. A single HTTP/2 stream can carry an SSE response containing many events. Keeping the two meanings separate avoids counting each event as a new HTTP request.

## A label tells the receiver where a piece belongs

HTTP/2 sends structured pieces called frames. A frame includes a type and, where applicable, a stream identifier. HEADERS frames carry encoded header information; DATA frames carry body bytes. The receiver uses the identifier to associate each piece with the correct exchange.

This illustration shows possible interleaving on one connection. Read left to right. The labels belong to streams, not packets; the sequence and identifiers are invented rather than reconstructed from our browser exports.

```text
one connection -> HEADERS A | HEADERS B | DATA B | DATA A | DATA B
                    |           |          |        |        |
book list A <-------+-----------|----------|--------+        |
cover image B <-----------------+----------+-----------------+
```

Stream A can finish before stream B even though pieces of B arrived first. This sharing is called **multiplexing**. It removes the requirement to deliver whole responses in request order, but does not promise that every request is fast. The connection still has finite bandwidth and the sender still chooses what to send next. The [HTTP/2 standard](https://www.rfc-editor.org/rfc/rfc9113#section-5) defines streams and their relationship to the connection.

HTTP/2 normally runs over TCP, which delivers bytes in order. If a needed byte is lost, later bytes can wait for its replacement even when they contain another HTTP/2 stream. Interleaving at the HTTP layer therefore does not eliminate all waiting caused by the transport. The distinction matters when explaining an observed delay: a protocol capability is not a diagnosis.

## Sharing also needs limits

A receiver cannot necessarily consume incoming bodies as quickly as a sender can produce them. HTTP/2 **flow control** tells the sender how much additional DATA it may send. Budgets apply both to a stream and to the connection; WINDOW_UPDATE frames increase the permitted amount. This protects receiver capacity, rather than setting a desired requests-per-second rate.

Imagine the connection budget is exhausted. A stream with room in its own budget still cannot send more DATA until the connection has room too. Conversely, room on the connection does not override an exhausted stream budget. This two-level rule gives us a prediction without needing to know a particular implementation's buffer layout. It concerns DATA, not every frame type. The [flow-control rules](https://www.rfc-editor.org/rfc/rfc9113#section-5.2) provide the exact constraints.

A HAR usually summarizes a request and response after these mechanisms have done their work. Its timing fields do not report each budget change. Two overlapping request intervals cannot establish which frames were interleaved, which stream waited for credit, or whether both requests used the same connection.

## Repeated headers can use remembered values

Two requests may repeat the same authority, content preferences, and other header fields. Sending the full text each time wastes space. HTTP/2 uses **HPACK**, a representation for header fields that can refer to table entries instead of repeating their complete names and values.

There is a predefined static table and a changing dynamic table. The encoder, which writes the representation, can add a field to the dynamic table; the decoder, which reads it, makes the corresponding change. A later reference is meaningful only while the correct table state is available. Request and response directions keep independent dynamic state. The [HPACK specification](https://www.rfc-editor.org/rfc/rfc7541#section-2) explains those tables and contexts.

Here is an analogy, not HPACK byte encoding. Suppose the sender first writes “remember entry 7 as `accept: text/event-stream`,” then later writes “use entry 7.” A reader starting with the second instruction cannot recover the text from that reference alone. Actual HPACK indexes and encodings follow their own specified rules; seven in this example is only an illustrative label.

That explains why a decoded header list is insufficient to reconstruct its compressed bytes. The exporter has already expanded the representation and may have omitted its history. Netscope can show the resulting names and values, but cannot derive HPACK table contents or compression savings from those fields. Gzip or Brotli compression of a response body is a separate mechanism and supplies no such header evidence.

## What our recordings establish

Our frozen Chrome and Firefox HTTPS exports identify the requests as HTTP/2. A separate log from the controlled server reports negotiated H2 and associates each browser's sequence with one server-local session. Together these establish the protocol and connection reuse in that experiment. The requests were triggered sequentially; they do not demonstrate concurrent interleaving.

The HAR importer does not consume that server log. Its connection labels, reported sizes, and timing values remain exporter observations. Netscope does not convert local server session numbers into HTTP/2 stream IDs or invent frames. The [compatibility record](../HAR_COMPATIBILITY.md) links the recordings, independent observations, and reproduction method.

This evidence boundary is a design choice as well as a teaching rule. Adding a from-scratch HTTP/2 decoder would not recover bytes absent from the input. Genuine frame inspection would need suitable wire evidence, decryption where necessary, and connection state. Existing protocol libraries can supply live transport behaviour, but adopting one would still require a new review and testing contract. Netscope's current live sender deliberately remains HTTP/1.1.

Try the two requests from the opening again. If the HAR shows overlapping times and both say HTTP/2, can you prove that they shared a connection? If the server log supplies that connection fact, can you also prove the interleaving sequence? The first answer needs connection evidence; the second needs frame evidence. Learning to ask for the missing observation is more useful than drawing a convincing sequence from guesses.
