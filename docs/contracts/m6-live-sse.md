# M6: bounded live SSE and deeper HTTP/2 teaching

Agreed scope implemented after reader approval on October 13, 2026. Included in v0.2.0.

## Explicit network boundary

From a valid GET Review, l opens Live SSE Review. This performs validation only. Enter/s from that screen accepts one new operation and its start instant. POST and selected explicit Accept headers are rejected; the client supplies Accept: text/event-stream and Accept-Encoding: identity. Other reviewed header/destination validation remains shared. Ordinary replay keeps its 10-second/2-MiB rules; live mode has 30 seconds total including dispatch/setup, 64 KiB raw response bytes, and 100 complete observed events. Verified HTTPS, HTTP/1.1, no redirects, retries, cookie jar, proxy discovery, decompression or reconnection.

Live requires status 200, exactly one Content-Type with text/event-stream essence, and no content encoding except identity. Headers received before rejection remain in the result. No received error page/compressed body is claimed as a valid stream.

## Observation rules

Retain raw bytes bounded by 64 KiB. Interpret the valid UTF-8 prefix using captured SSE rules; a trailing partial character waits until the next read. Invalid UTF-8 stops with an error; EOF inside a character is a distinct error. Earlier events/raw bytes remain. EOF alone never dispatches pending event data. ID/retry buffers start empty per operation; no instruction schedules reconnection. [DONE] is ordinary data.

Observation durations use a monotonic clock, starting at accepted Send. New complete boundaries found in one body fragment share that read's observation duration. HTTP-library body fragments are not asserted to be packets or server writes. Screen draw times do not supply event times. These are local new observations, explicitly separate from HAR timing and browser event delivery.

At 100 events or 64 KiB, stop immediately with CaptureLimited, even if natural EOF might occur next. Extra bytes within a retained read can remain beyond event 100 but receive no additional times. This differs from ordinary replay SizeLimited, which requires observing an extra byte beyond its retained cap. Cancellation/deadline/errors retain observations and unfinished text. Cancellation returns to the draft; o opens the retained result. Quit cancels and joins cleanup.

A watch channel publishes only the latest cumulative bounded snapshot. The worker owns network state; the application owns rendered state. The final result remains once-only. Drain the newest progress while accepting a final result; reject progress from stale operation IDs. Drawing/editing/reviewing never performs I/O.

## Teaching and evidence

[Live lesson](../chapters/27-observing-live-events.md) explains clocks, fragments, UTF-8, boundaries, ownership and publication alternatives. [HTTP/2 lesson](../chapters/28-sharing-a-connection.md) explains streams/multiplexing, stream and connection DATA flow control, HPACK tables and directional state. Illustrative frames are labelled. Existing native H2 exports and the independent server log establish their recorded protocol and reuse, not concurrent frame order or HPACK byte history. This milestone adds no HTTP/2 replay or wire decoder.

Tests use fixed byte splits, pure supplied durations, local sockets/duplex I/O and explicit cancellation, with watchdogs rather than performance thresholds. Native browser fixtures are frozen inputs. Terminal checks verify a live event before EOF, review-only no-I/O, cancellation, retained result, socket disposal and terminal restoration. Whole-EventSource conformance, reconnection, load guarantees, universal HAR compatibility and WebSocket frames remain outside this contract.
