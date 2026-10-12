# M3, first slice: send one reviewed GET request

The [request-body extension](m3-request-bodies.md) adds POST, reviewed UTF-8 bodies, and managed body headers. Its rules supersede the original GET-only scope below; the transport and response limits still apply.


Status: GET draft, review, network operation, separate results, and cancellation are implemented. On October 9, 2026, we agreed to an editable URL, explicit headers, a 10-second total deadline, and a 2 MiB body limit. The [transport decision](replay-transport.md) records the selected stack and additional bounds; the [verification record](../VERIFICATION.md) records checks actually run. This contract specifies required behaviour.

The reader's question is: “What happens when I send this request?” The visible journey is select, edit, review, send, then inspect a separate result. Reading a file, drawing a screen, editing a draft, and opening a preview must never initiate network activity.

## Prepare a separate draft

Open replay from a selected GET request. Other methods remain inspectable but cannot start replay in this slice. Require a reported request body size of zero and no captured request-body representation; missing size, unavailable size, and captured postData receive an eligibility explanation. This avoids silently dropping a body. Method and body editing are later work.

Copy the source URL into an editable draft and keep the source request ID. The imported session remains unchanged. Returning from the draft restores the original inspector and list selection. Reopening replay starts a fresh draft; this slice does not save drafts or results to disk.

Accept an absolute HTTP or HTTPS URL with a host. Reject user information embedded in the URL, control characters, whitespace, and backslashes rather than silently repairing them. Localhost and private-network destinations are allowed so a user can target a development server. Parse using the project's URL library and show the effective serialized destination before sending. Fragments are not sent; show any removed fragment separately in the review. Retain path and query meaning without sorting query parameters or rebuilding the URL from HAR query metadata.

Changing the destination clears every selected or manually entered header. Explain that reset immediately; a header chosen for one destination must not silently carry over to another. A valid edited destination can be reviewed again. Invalid input stays editable with a field-specific explanation and a disabled Send action.

## Choose outgoing headers explicitly

Start with no selected captured headers and no manually entered headers. Show captured request headers as individually selectable entries, preserving duplicate entries for inspection. Allow new name/value pairs and removal of selected entries. Captured cookie arrays do not generate headers. A captured Cookie or Authorization header can be included only through an explicit selection, just like manually entering one. There is no cookie jar, browser login integration, ambient credential lookup, or automatic application of Set-Cookie from a response.

Validate selected and entered header names and values before review. Reject invalid names, embedded line breaks, and unsupported value bytes with an explanation. Preserve repeated outgoing entries where the client supports them; never silently overwrite an earlier value. If the chosen client cannot represent a selected combination, report it before sending rather than claiming an exact replay.

The client manages Host, Content-Length, Transfer-Encoding, Connection, Proxy-Connection, Keep-Alive, TE, Trailer, and Upgrade; these cannot be selected or entered, regardless of letter case. Fields named by a captured Connection header are also excluded from copying. Proxy-Authorization is unsupported because this slice uses direct connections. Do not forward pseudo-header fields such as :authority as ordinary headers.

Disable automatic response decompression and use a client-managed Accept-Encoding: identity. Do not permit a user override in this slice. This makes the response limit apply to the retained response-body bytes without an additional decompression budget. A server can still return encoded content; retain its bytes and Content-Encoding header without claiming they are decoded. Disable environment proxy discovery and automatic authentication. No implicit User-Agent, cookie, or other application header should be added without being disclosed in the review.

## Review before sending

The review names the original request, edited effective URL, GET method, no request body, explicit headers, client-managed headers, and captured headers left out. It shows the 10-second timeout and 2 MiB limit. A changed draft invalidates the previous review. An explicit Send action is the only transition allowed to start an operation.

This is a preview of the semantic request, not a promise of identical wire spelling, header order, or captured browser behaviour. Any transport-generated fields must be disclosed. Sensitive header values are masked in the review by default, with an explicit reveal action; they are never written to logs or fixed error messages. URL query values remain part of the destination the user reviews. Do not claim automatic detection of every secret in an arbitrary header or URL.

Implementation default for this first slice: HTTP/1.1 over HTTP or HTTPS, with certificate and hostname verification enabled. Do not attempt to reproduce the capture's reported version. State the replay's supported version in the review and display the actual response version in the result. No HTTP/2 or HTTP/3 negotiation, insecure TLS toggle, custom certificate configuration, or proxy configuration is included yet. The client library decision must be recorded before adding dependencies; use a maintained HTTP implementation rather than hand-written wire parsing.

## One operation, with bounded work

Allow only one active send. Repeated Send key events while running do not start duplicates. No automatic redirects, application retries, or hidden client retries that resend an HTTP request. Connection establishment may try resolved addresses without being labelled another user attempt. Check the chosen client's behaviour against this boundary during implementation.

The total deadline starts when Send is accepted and covers connection setup, DNS, TLS, sending, and reading the response body. A single elapsed-time limit must not restart after each received chunk. Connection and parsing errors produce explicit outcomes. Do not infer the original browser's DNS or TLS timings from this new operation.

Retain at most 2,097,152 response-body bytes after HTTP transfer framing is removed, without content decompression. Do not trust Content-Length alone to enforce the limit. A body ending exactly at the cap is complete; another byte makes it size-limited. At most one extra byte may be inspected to distinguish those cases, without retaining it. If that final distinction stalls, the total deadline still applies and the result remains incomplete. A response without a declared length is subject to the same policy.

Retain status, response version, headers, and body bytes that have reached application state before cancellation, timeout, size limit, or read failure. Distinguish a complete empty body, no response headers received, and a partial body. Bound retained header data and event queues as well; record concrete client-supported header limits when selecting the stack. A malicious or stalled response must not cause unlimited buffering or block terminal input.

## Cancellation is observable behaviour

The running view offers Cancel. Esc requests cancellation and returns to the draft only after the worker has stopped or its result has been safely detached and cannot change that draft. q and Ctrl+C cancel active work and restore the terminal while shutting down the worker. Leaving a screen must not merely hide a task that continues reading a response indefinitely.

Give each operation a monotonically assigned ID. Only events for the active operation may update its state. A terminal outcome is accepted once; later completion, cancellation, or error events cannot overwrite it. If completion has already been accepted, a later Cancel does not relabel it. If cancellation was accepted first, a later completion cannot replace the cancelled outcome. Tests control event order explicitly.

Cancellation stops local waiting and I/O as supported by the client. It cannot retract already-sent bytes or undo work performed by the server. Show that limit in the cancellation explanation. Verify task teardown and connection/body disposal rather than treating a discarded result as sufficient proof of cancellation. The stack decision must identify any DNS or other background work that cannot be synchronously interrupted and how it is isolated from UI state and bounded during shutdown.

## Inspect the new result separately

Label the result as a new request and retain its source request ID and effective destination. Never insert it into the imported request list as though the exporter recorded it. Show complete, cancelled, timed out, size-limited, or failed status; response metadata when available; retained byte count; and a bounded text or binary preview. An HTTP error status is a received response, distinct from a transport error. A 3xx result remains visible without following Location. To follow it in this slice, explicitly edit a new draft and review/send again.

Do not label a shorter duration as proof that the new client or protocol is faster. The original and new requests may differ in destination, headers, server state, and network conditions. Elapsed duration measures this operation; it is not a reconstruction of the captured connection.

## Deterministic acceptance checks

Pure tests cover draft eligibility, URL validation and effective destination, fragment disclosure, immutable imported data, header selection and validation, duplicate handling, reserved fields, destination-change resets, review invalidation, and Send gating. Fixed terminal buffers cover edit, review, running, and each result state at 80 × 24 and 100 × 30, including long Unicode values and control escaping. Text editing must have explicit key rules so typing q in a field cannot accidentally quit.

Use scripted in-memory transport events for state transitions, partial data, stale operation IDs, channel closure, duplicate Send actions, and completion/cancellation races. Inject or pause the runtime clock for deadline tests; advance it deliberately. Test timeout before headers and during body reading, total deadline versus repeated chunks, and unchanged deadlines after progress.

Use local servers bound to ephemeral loopback ports for actual HTTP behaviour. Synchronize readiness and response stages with explicit signals, not arbitrary sleeps. Verify what headers and target the server receives, that no automatic redirect or retry occurs, and that the UI stays responsive while the server withholds a response. Use exact bodies of cap minus one, cap, and cap plus one bytes, misleading or absent lengths where valid, chunked transfer, encoded content, abrupt disconnects, and deliberately slow delivery. TLS checks use local test certificates and a test-only trust configuration; production verification remains enabled. Include rejection of an untrusted certificate and a hostname mismatch.

Real-socket and pseudo-terminal tests still need bounded wall-clock watchdogs to prevent hangs. Keep them separate from deterministic state/clock tests and do not assert exact elapsed durations. Cancellation tests must observe worker termination and local I/O disposal; they must not assert that the server never saw already-sent data. No public endpoint, real credentials, external DNS, or nondeterministic service response is needed.

## Teaching and implementation order

Begin the lesson with the visible send-and-wait sequence. Explain a future as work that can make progress when polled, then introduce async and await where the reader needs to understand waiting without freezing input. Use an illustrative timeline to distinguish terminal input, network waiting, and cancellation. Explain ownership of a draft, an operation, and a result before introducing channels or operation IDs. Examples must compile, and code names must not determine the chapter outline.

The draft/review model, sending operation, terminal integration, and ownership/waiting lessons are present. The HTTP/runtime/TLS choice was recorded before dependencies were added. This contract does not authorize live traffic to a captured external destination as a development test; the approved verification path uses synthetic local servers.

Completion requires the accepted behaviours, tests, terminal restoration, and beginner explanation to be reviewed together. This document remains a specification, not a verification report. Cookie persistence, body/method editing, redirect automation, HTTP/2 or HTTP/3 replay, curl import, saved results, and comparative benchmarking remain later slices.


## Implemented editor slice

From an inspector, r opens an eligible draft or displays an eligibility explanation. Up/Down selects rows; Enter edits the URL, toggles one captured header, removes an added header, starts a two-field header entry, or reviews. In a text field, characters append, Backspace removes the final Unicode grapheme, Ctrl+u clears, Enter saves, and Esc cancels that edit. Left/Right cursor movement and multiline paste are not implemented. After entering a header name, Enter moves to its value; Esc abandons the pending header. Validation failures retain editable input. Small terminals preserve draft state and suspend editing; Ctrl+c still exits.

All header values are masked in lists/review until v reveals them. Values being typed are visible in their editing field. Returning from review or saving an edit resets masking. Esc from review returns to the draft; Esc from the draft discards it and returns to the same inspector position. q quits outside text editing; inside a field it is ordinary text. Ctrl+c always quits. Enter or s from a valid review queues one send. While sending, additional Send keys have no effect. Esc requests cancellation and returns to the draft after cleanup; o opens the retained result. Ordinary completion opens the result directly.

URL text is limited to 16 KiB when validating and entering it. Header names and values are each limited to 8 KiB when selecting or adding them, with at most 100 manually added entries. Initial header values support printable ASCII and tabs only; other captured values remain inspectable but cannot be selected. A change to any saved URL text clears every header choice, including a fragment-only edit; cancelling the edit does not. Review validation is recomputed, not cached. The imported request, its body and the inspector/list position remain unchanged. Review and Send both check the concrete client representation. Explicit outgoing fields are limited to 100 entries and 32 KiB combined. Response parsing accepts at most 100 fields and 32 KiB of headers including the status line, with a 32 KiB connection read-buffer cap. The channel holds one result; there is no progress-body queue.
