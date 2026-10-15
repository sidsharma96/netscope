# When did an update become visible here?

A saved event stream can show three book updates without saying when any one arrived. To answer a timing question, we need a new observation. Suppose we send a GET now and watch an open response. The first update becomes readable while the server is still preparing the next. Waiting for the whole response to end would hide precisely the behaviour we wanted to inspect.

Live [SSE](../GLOSSARY.md#sse) in Netscope is a separate, reviewed action. From an ordinary GET review, `l` opens the live review; Enter or `s` then starts the request. It adds `Accept: text/event-stream`, asking the server for this format, and requests uncompressed content. The capture remains unchanged. Reviewing, editing, and drawing the screen still perform no network work.

## A clock needs a named starting point

An elapsed duration answers a question only when its beginning and end are clear. Netscope begins its clock when you accept Send. Connection setup, TLS verification, and waiting for response headers are included. It records a duration when the worker first sees enough valid text to identify a complete data-bearing event boundary.

This is a local observation time. It is not the instant the server created the update, the moment a network packet reached the machine, or the time a browser delivered an event to JavaScript. Those would need their own instrumentation. Netscope uses a [monotonic clock](../GLOSSARY.md#monotonic-clock), which measures elapsed duration without depending on calendar-clock corrections.

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

The word `café` ends with a character occupying two [UTF-8](../GLOSSARY.md#utf-8) bytes. A read may contain the first byte and leave the second for the next read. Rejecting the first fragment immediately would mistake a temporary split for invalid text. Decoding each fragment independently could instead replace the character and change the event's data.

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

Try predicting a split `é`, two events in one read, and cancellation after a data line without its blank boundary. Which bytes remain? Which events have times? The live observation code [`LiveCapture`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L38), network tests [`live_events_are_published_before_eof_and_cancel_disposes_the_socket`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/tests.rs#L637), and [local walkthrough](../DEVELOPMENT.md) let you check those predictions without relying on a public service.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Times in a live SSE observation

**Level:** Newcomer


During live SSE, Netscope shows a time beside each event. Where does that clock start, and what does the time measure?

<details>
<summary>Answer</summary>

The clock starts when you accept Send, so connection setup, TLS and waiting for headers all count. An event's time is when the worker first saw its complete boundary in received bytes. Events that arrive in the same read share one time. It is a local observation, not when the server created the event or when a browser delivered it. The observation stops after 30 seconds, 64 KiB, or 100 events.

**Learn more:** [A clock needs a named starting point](../chapters/27-observing-live-events.md#a-clock-needs-a-named-starting-point) · [Stopping is part of the observation](../chapters/27-observing-live-events.md#stopping-is-part-of-the-observation) · [`STREAM_TIME_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L10)

</details>

### Why does live SSE refuse your Accept header?

**Level:** Practitioner


Live SSE observation is a separate mode from ordinary replay. Which requests may use it, why does it reject a selected `Accept` header instead of replacing it, and which responses does it accept before reading the body?

<details>
<summary>Answer</summary>

`validate_stream` first runs the ordinary replay validation, then requires GET and refuses any selected or added `Accept` header with "Live SSE manages Accept; remove the selected Accept header first". The client sets `Accept: text/event-stream` itself. Silently replacing your header would mean the request sent differs from the request you reviewed, so Netscope asks you to remove it instead.

Once headers arrive, `validate_response` requires status 200, exactly one `Content-Type` whose media type is `text/event-stream`, and no content encoding other than `identity`, because live SSE does not decompress. Only then does it start reading body bytes. The whole observation shares one 30-second deadline that starts when you press Send and is never extended by progress, and it also stops at 64 KiB or 100 events.

**Learn more:** [Explicit network boundary](../contracts/m6-live-sse.md#explicit-network-boundary) · [Stopping is part of the observation](../chapters/27-observing-live-events.md#stopping-is-part-of-the-observation) · [`validate_stream`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L24-L37) · [`validate_response`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L108-L134)

</details>

### How live SSE stamps, decodes, and publishes events

**Level:** Expert


During a live SSE observation, how are per-event times produced? What happens when a UTF-8 character is split across two reads, or when invalid bytes arrive? How does the screen see progress, and what does the parsing approach cost?

<details>
<summary>Answer</summary>

Before the body is read, the response must have status 200, exactly one `text/event-stream` Content-Type, and no content encoding other than identity. Each body frame is then appended to a buffer capped at 64 KiB.

`observe` finds the longest valid UTF-8 prefix. If the error is an incomplete final character, it waits for more bytes. If the bytes are invalid, it publishes the valid prefix and stops with an error. A stream that ends inside a character is also an error.

The whole valid prefix is parsed again with `parse_sse` on every frame. `observed.resize(count, elapsed)` gives every event completed by this frame the time since Send at which the frame arrived. Several events in one frame share a time, and the time is arrival here, not the server's send time. The 100th event or the 64 KiB limit ends the observation without waiting for the server to close.

Progress goes through a `watch` channel with `send_replace`, which keeps only the newest snapshot. The screen ignores snapshots for a stale Send ID. Reparsing is O(n) per frame, so the total is quadratic in the number of frames, bounded by 64 KiB.

**Learn more:** [Bytes can stop in the middle of a character](../chapters/27-observing-live-events.md#bytes-can-stop-in-the-middle-of-a-character) · [Why does live SSE reparse its whole prefix on every read?](../chapters/34-defending-the-design.md#why-does-live-sse-reparse-its-whole-prefix-on-every-read) · [`observe`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L51-L85) · [`validate_response`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L108)

</details>
