# When did an update become visible here?

A saved event stream can show three book updates without saying when any one arrived. To answer a timing question, we need a new observation. Suppose we send a GET now and watch an open response. The first update becomes readable while the server is still preparing the next. Waiting for the whole response to end would hide precisely the behaviour we wanted to inspect.

Live SSE in Netscope is a separate, reviewed action. From an ordinary GET review, `l` opens the live review; Enter or `s` then starts the request. It adds `Accept: text/event-stream`, asking the server for this format, and requests uncompressed content. The capture remains unchanged. Reviewing, editing, and drawing the screen still perform no network work.

## A clock needs a named starting point

An elapsed duration answers a question only when its beginning and end are clear. Netscope begins its clock when you accept Send. Connection setup, TLS verification, and waiting for response headers are included. It records a duration when the worker first sees enough valid text to identify a complete data-bearing event boundary.

This is a local observation time. It is not the instant the server created the update, the moment a network packet reached the machine, or the time a browser delivered an event to JavaScript. Those would need their own instrumentation. Netscope uses a monotonic clock, which measures elapsed duration without depending on calendar-clock corrections.

The following invented example illustrates two events received in one body fragment. Arrows describe when information becomes available to the worker; the numbers are teaching values, not measurements.

```text
accepted Send       +0 ms
response headers    +40 ms
one body fragment   +75 ms  -> "data: first\n\ndata: second\n\n"
                                event 1 observed +75 ms
                                event 2 observed +75 ms
```

Both events receive the same observation time because both boundaries became available during the same read. Drawing separate times with guessed spacing would make the display look more precise while giving it less evidence. The HTTP library provides body fragments after processing HTTP framing; these fragments are not promised to match packets or server writes.

## Bytes can stop in the middle of a character

The word `café` ends with a character occupying two UTF-8 bytes. A read may contain the first byte and leave the second for the next read. Rejecting the first fragment immediately would mistake a temporary split for invalid text. Decoding each fragment independently could instead replace the character and change the event's data.

Netscope retains the raw bytes and examines the longest valid UTF-8 prefix. A partial character waits for another read. Once the complete character arrives, the same bytes become valid text. If the bytes are actually invalid, or the response ends inside a character, the operation stops with an explicit error. Earlier observations and the retained prefix remain available.

Event boundaries have the same need for patience. `data: next` followed by a single line ending is still unfinished. The parser requires an empty line before emitting the event. Ending the response does not manufacture one. A complete HTTP response can therefore contain an unfinished SSE block: the transport ended normally, while the application-level event did not finish.

## The screen needs the latest observation, not every drawing request

Consider a worker receiving twenty events while the terminal draws once. Sending twenty separate screen updates through an unbounded queue could retain more and more work if the screen is slow. Blocking the worker on every update would make terminal speed affect network observation.

Instead, the worker publishes its latest cumulative state: bounded raw bytes and the observation times collected so far. A newer publication replaces the older one. The terminal may skip intermediate publications, but the newest state still contains the earlier retained events. “Latest” would lose information if each publication held only one event; it works here because the snapshot is cumulative.

The final outcome travels separately and is accepted once for the active operation. A late update from an older operation cannot replace the current one. This is the same ownership boundary introduced in the sharing-work lesson: the worker owns network work, and the application owns the displayed state. A channel carries a copied observation between them. We do not need both threads to edit one shared event vector under a lock.

This choice does have a cost. Publishing copies up to the bounded body and time list, and parsing the retained prefix repeats work. An incremental parser could reduce that work, but would also introduce more state to keep consistent across cuts, cancellation, and final results. The current small capture budget makes the simpler approach reviewable; it is not a claim of unlimited throughput. The [architecture record](../ARCHITECTURE.md) states when to revisit it.

## Stopping is part of the observation

An event stream may never end naturally. Netscope stops after 30 seconds in total, 64 KiB of raw body bytes, or 100 complete observed events. Reaching a capture limit reports that limit; it does not claim that all messages were received. A read containing several events can leave additional bytes in the retained body, while only the first 100 events have observation times.

Esc cancels local waiting and closes this operation's connection. It cannot undo work already performed by the server. You can reopen the retained result from the draft. Quitting also cancels and cleans up the worker. An `id` or `retry` field is displayed as stream state; neither causes reconnection. The literal data `[DONE]` has no special stopping power because SSE itself assigns it no such meaning.

The response must have status 200, one content type identifying `text/event-stream`, and no unsupported content encoding. This narrow contract prevents a compressed body or an error page from being displayed as a successful stream. TLS certificates are verified for HTTPS. The sender uses HTTP/1.1, so this observation does not reproduce a browser's recorded HTTP/2 exchange.

Try predicting a split `é`, two events in one read, and cancellation after a data line without its blank boundary. Which bytes remain? Which events have times? The live observation code [`LiveCapture`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L38), network tests [`live_events_are_published_before_eof_and_cancel_disposes_the_socket`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/tests.rs#L629), and [local walkthrough](../DEVELOPMENT.md) let you check those predictions without relying on a public service.
