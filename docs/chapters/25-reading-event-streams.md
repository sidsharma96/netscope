# How many messages are inside one response?

A page shows newly added books without asking for the whole list again. The server keeps a response open and sends another piece of text when a book appears. Later, you export a HAR and find one response body containing several updates. Where does one message end and the next begin?

Server-sent events, usually shortened to SSE, provide a text format for that situation. A browser can read updates as they arrive. Netscope begins with a different kind of evidence: text already retained in a capture. We can interpret boundaries in that text, but the saved body does not tell us when each piece arrived or whether a browser delivered it to the page.

## A blank line gives a boundary meaning

Consider this invented response text. The empty line between the blocks matters just as much as the visible letters.

```text
event: added
data: The Rust Book
data: shelf 3

data: Another book

```

An `event` field supplies a type; without a nonempty type, the default is `message`. Each `data` field contributes text. For the first block, the resulting data is `The Rust Book`, a newline, then `shelf 3`. The blank line ends that event. The second block yields another event with its own data.

The type describes the event, not the HTTP method. Both events can belong to one HTTP response. Their content might be JSON, plain text, or another application format; SSE does not require JSON and does not make those contents trustworthy.

The following diagram is an illustration of parsing, not a measured timeline. Read downward to see the text held temporarily, then the boundary that produces a result.

```text
data: The Rust Book  → pending data: The Rust Book
data: shelf 3        → pending data: The Rust Book\nshelf 3
blank line           → event 1; clear pending data

data: Another book   → pending data: Another book
blank line           → event 2; clear pending data
```

Here `\n` represents a newline inside the resulting data. It is not an event boundary by itself. The boundary requires an empty input line.

## Why remembering a little state is necessary

Reading each line independently would lose the relationship between two data fields. The parser needs state: values that remember what earlier input means for the next line. Pending data belongs to the current block. The event type belongs to that block too, so it resets when the blank line is processed.

An `id` field behaves differently. Its value remains available for later events until another valid ID replaces it. An empty ID clears it. A live browser can use an ID when reconnecting, but a retained ID is not proof that a reconnection occurred. Netscope starts the ID buffer empty for each captured body; an earlier connection's state is unknown.

A `retry` field contains a reconnection setting in milliseconds. It is an instruction, not a measurement of elapsed time. Netscope displays the last valid setting it examined as digit text, without converting a huge value into a fixed-size integer. The inspector does not schedule a retry.

These are different lifetimes within one parser: some values last for a block, others can affect subsequent blocks. This is why clearing every value after an event would be just as wrong as retaining every value indefinitely.

## Keep small differences visible

Field names are literal: `data` and `Data` differ. A colon separates a name from its value, and at most one following space is removed. A line beginning with a colon is a comment. Unknown fields do not contribute data. Empty data is still data: a `data` line followed by a blank line produces an event containing an empty string. A comment-only block produces no event.

Line endings can be LF, CR, or the CRLF pair. These names describe newline characters, not different event types. One leading byte-order mark, a Unicode marker sometimes found at the start of text, is ignored. The [HTML Standard's parsing rules](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream) define the format; the [interpretation rules](https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream) explain how fields affect event state.

Netscope interprets already retained UTF-8 text. UTF-8 is the character encoding introduced in the text-editing lesson: characters occupy one or more bytes. Binary or undecodable representations remain unavailable for this analysis. A reported MIME label, the response's content-format name, must identify `text/event-stream`; text merely resembling SSE is not enough to select this interpretation.

## A file can stop before an event does

Suppose the retained body ends after `data: Another book`, without a blank line. Treating the end of the file as another boundary would manufacture an event. Netscope keeps that tail visible as an unfinished block instead. Even a line ending after the data is insufficient: the empty line is still missing.

An unfinished tail does not diagnose the server. The exporter may have stopped recording, the stream may have been interrupted, or the retained representation may simply omit later text. Conversely, a body ending at a blank line does not establish that the capture includes every event the server sent.

A parser can also stop because the application deliberately limits its work. Netscope examines at most 64 KiB of retained text and keeps at most 100 complete events. Both limits have explicit notices. They bound inspection work and the derived list; they do not change the recorded body. Individual long fields can also have shortened display previews, labelled separately from an analysis limit. Values are quoted: `""` means empty text, `\n` represents a newline, and `\\n` represents a literal backslash followed by n. This makes the characters visible without allowing captured controls to command the terminal.

## Borrow the recording, own the joined data

The captured body already owns its text. Event types and IDs can refer to portions of those existing bytes while inspection is in progress. Joining several data values with newlines creates a different string, so that result needs its own storage. Borrowing and ownership therefore serve different needs in the same event.

Keeping this interpretation temporary avoids storing another event list for every response in a large capture. It costs repeated bounded parsing when the inspector draws or adjusts its scroll position. Caching a list would trade that repeated work for retained memory and additional ownership arrangements. The current choice is deliberately small; live input also needs to preserve unfinished bytes and observation times between arriving pieces. The [live observation lesson](27-observing-live-events.md) explains how a bounded cumulative prefix can reuse these parsing rules.

Here is a small example using the parser after those rules are understood:

```rust
use netscope_core::parse_sse;

let stream = parse_sse("data: first\ndata: second\n\n");
assert_eq!(stream.events.len(), 1);
assert_eq!(stream.events[0].data, "first\nsecond");
```

`parse_sse` receives borrowed text and returns the derived inspection. `events` is its vector of events, `len()` counts them, and `[0]` selects the first one. The assertions check the count before selecting that element. Changing the input to omit the last newline leaves an unfinished block, so there is no first event to index.

## Try separating content from time

Open the supplied SSE example described in the [development guide](../DEVELOPMENT.md#inspect-retained-sse-text). Inspect its first request and scroll to the derived events. Explain why two data lines become one event and why the final block remains unfinished. Then inspect the response whose text was not recorded. Explain why unavailable text must not be reported as zero events.

Could the request's total duration tell you the pause between those two events? It cannot: one duration covers the request, while the body preserves content order. A [new live observation](27-observing-live-events.md) can measure when Netscope sees each complete boundary; it cannot recover the original browser's missing times.

The [parser](../../crates/netscope-core/src/sse.rs), [parser tests](../../crates/netscope-core/tests/sse.rs), and [display](../../crates/netscope-tui/src/sse_view.rs) support the rules explained here. The [contract](../contracts/m5-captured-sse.md) records exact recognition, budgets, and unavailable states; the [decision record](../ARCHITECTURE.md) keeps the alternatives and reasons to revisit them.
