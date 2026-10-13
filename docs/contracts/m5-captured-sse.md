# Captured SSE event inspection

Implemented October 13, 2026. This is an offline interpretation of retained response text. It neither runs EventSource nor captures live events. HTTP status, headers, and request timings do not establish browser delivery or per-event arrival times.

## Recognition and representation

Select the interpretation from the HAR content MIME label's essence before the first semicolon. Trim surrounding ASCII spaces/tabs and compare text/event-stream ignoring ASCII letter case. Parameters are permitted. Do not guess from body text or reconcile competing Content-Type headers. Preserve all original metadata. A status other than 200 does not prevent inspecting labelled text; this is not browser acceptance policy.

Body::Text and Body::DecodedText are available. Missing, binary/non-UTF-8, and undecoded bodies have distinct unavailable explanations. Empty retained text is available and has zero derived events. No additional decoding, replacement of invalid UTF-8, or decompression is performed.

## Retained-text rules

Based on the [HTML Standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream): ignore one leading BOM, accept LF/CR/CRLF, compare field names literally, split at the first colon, and remove at most one leading ASCII space from the value. A field without a colon has an empty value. Colon-prefixed comments and unknown fields produce no data. Count examined comments and ignored fields/invalid values.

Complete data fields append their values and a newline. On a blank line, a nonempty data buffer yields an event after removing the final appended newline; event type defaults to message if empty. Clear data/type at every blank line, including blocks with no data. An empty data field still creates data; metadata-only blocks do not emit an event.

A valid id value replaces the persistent parse-local ID buffer, including an empty reset; a value containing NUL is ignored. Begin that buffer empty for each retained body, explicitly without knowledge of earlier connections. Each derived event retains the buffer at its boundary. Retry accepts a nonempty sequence of ASCII digits, retaining the original digit string so numeric overflow cannot change validity. Show the last examined setting as an instruction, not an actual reconnection or timestamp. No default retry interval is invented.

Only complete terminated lines are interpreted. A final line without a terminator remains in the pending tail. EOF never creates a blank line or emits pending data. Show the trailing block after the last blank-line boundary as unfinished, including comment/metadata-only tails; this label concerns the boundary, not a diagnosis of network failure. The final ID/retry summaries concern examined complete lines, which can include lines in an unfinished block.

## Budgets and display

Examine at most 64 KiB of original UTF-8 text, trimming a cut to a character boundary. Retain at most 100 events. The 101st complete data-bearing boundary stops analysis with an event-budget notice. Exactly 100 events alone do not imply omitted events. A text-budget cut has a distinct notice and cannot supply a missing delimiter. Later content is not examined after stopping; counts describe examined text, not the whole connection.

Parse on demand in the inspector. Borrow type/ID/retry/pending slices and own joined data. No event interpretation is stored in Session and no original request is mutated. Display ordered derived event numbers, type, ID buffer, escaped data, counters, last settings, pending tail, and limit notices beside the original response preview. Event numbers are display positions, not application IDs or timestamps. Display previews cap type/ID/retry at 256 UTF-8 bytes and data/tail at 2048 bytes, with shortening labelled. Values use quoted escaped strings, distinguishing empty text and literal backslashes from controls. Existing body-preview limits remain separate.

## Evidence

Synthetic [HAR fixture](../../tests/fixtures/har/sse.har) covers exported text, base64 UTF-8 text, unavailable text, and non-SSE text. [Parser tests](../../crates/netscope-core/tests/sse.rs) cover delimiters, reset/inheritance, literal field values, empty data, EOF, huge retry values, and budgets. [View tests](../../crates/netscope-tui/tests/views.rs) cover eligibility, safe output, reachable inspection, representation distinctions, and preserved source data. The [lesson](../chapters/25-reading-event-streams.md) explains boundaries and state before source names.

This is not whole-EventSource conformance, complete capture validation, live UTF-8 decoding, a reconnecting client, or a latency guarantee. Revisit before live capture or broader automatic content recognition.
