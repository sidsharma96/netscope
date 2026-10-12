# M1 contract: import and inspect a HAR

Established October 9, 2026. Status: import, summary, and interactive screens implemented; learning review pending. This contract takes precedence over earlier M1 lists and mockups. Its acceptance cases define the full milestone. See the verification ledger for executed importer checks and terminal checks.

## The visible result

Run `netscope capture.har` in a terminal. The application imports the file without sending traffic and opens a chronological request overview. Select a request and press Enter to inspect captured request and response data. Esc returns to the same selection. Press q to quit and restore the terminal.

Running without a path shows a short usage message and exits successfully. A file picker, recent-file persistence, curl paste, replay, connection reconstruction, and host diagrams are later work. No sample from a logged-in service is required.

## Accepted input profile

M1 accepts a UTF-8 JSON HAR with version `1.2`, optionally prefixed by a UTF-8 byte-order mark. The file extension does not determine validity. Reject invalid JSON, duplicate object member names, unsupported HAR versions, and wrong types in consumed fields. Do not coerce a numeric string into a number or treat null as an empty list.

Require a root object with a `log` object, a version string, a creator object with string name and version, and an entries array. An empty entries array is valid. Ignore unconsumed standard metadata and unknown fields, including underscore-prefixed exporter extensions. Unknown fields must not silently become protocol evidence. This is an inspection profile, not full HAR conformance validation or a lossless HAR editor.

Every entry requires a timestamp with an explicit UTC offset, a finite non-negative total duration, request and response objects, and a timings object. Parse timestamps to comparable instants while retaining the original timestamp string. Assign entry IDs in input order, then sort the overview by instant with source position as the tie-breaker.

Reject the entire import if a required consumed field is invalid. Report the first error in source order and field-validation order. Do not present a partially imported session without an explicit future partial-import policy. Unknown or invalid ignored metadata does not reject the inspection profile.

### Request and response fields

Require request method, URL, and reported HTTP version as strings. The method must be a nonempty HTTP token; preserve extension methods. Accept an absolute URL with an HTTP or HTTPS scheme and host. Validate without fetching it. Retain the original URL for inspection and use a parsed, normalised host only for grouping. Preserve unknown protocol strings as exporter-reported values instead of claiming support for that protocol. Empty strings mean no recorded protocol label; missing fields and other types still reject import.

Require response status as either integer 0 or an integer from 100 through 599, status text and reported HTTP version as strings, and a content object. Status 0 displays `No HTTP status recorded`, not a successful HTTP response or a diagnosed network failure. Require content size as a non-negative integer. MIME type may be omitted, producing an empty model label and a visible warning; if present it must be a string. Size is exporter-reported, not a verified decoded length.

Require request and response headers as arrays of objects with string name and value. Preserve order, original case, duplicate names, and empty values. Require query-string and cookie arrays from the request and a cookie array from the response. Query elements and cookies require string name and value. Recognised optional cookie fields are path, domain, and expires as strings, and httpOnly and secure as booleans. Cookie expires may also be null, as emitted by Chrome. Omitted or null expiry becomes no recorded expiry; this does not establish the cookie’s lifetime. A string expiry must be a timestamp with an explicit offset. Wrong types reject import; absent attributes stay absent. Unknown attributes are ignored for M1 and cannot support security claims. M1 displays captured cookies but does not reconstruct or apply a jar.

Body sizes and header sizes may be absent in this inspection profile. If supplied, accept integer `-1` as unavailable or a non-negative integer. Reject other values. Preserve the distinction between omitted and exporter-unavailable fields. This tolerance does not claim that a missing required HAR field is spec-conforming.

### Request body representations

Absent postData means `Request body not recorded`; it does not prove there was no body. If postData is present, require its MIME type and at least one of text or params. Both may be present; validate and retain them independently, without checking agreement or rebuilding one from the other. Text must be a string and an empty string is captured empty text. Params is an ordered array; each parameter needs a name, with optional string value, filename, and content type.

Show parameters as captured fields. Do not fabricate an exact multipart boundary, uploaded file bytes, or a replayable request body from metadata. A parameter can lack a value because the capture did not retain it. Reject a postData object with neither representation.

### Response body representations

Absent content.text means `Response body not recorded`. An empty string means `Captured body is empty`. Without an encoding field, retain text as exported. Do not decompress it again based on Content-Encoding.

For encoding `base64`, decode within the configured body limit. Show valid UTF-8 bytes as decoded text labelled `Decoded from base64`. Show other bytes as a bounded hexadecimal preview labelled binary. Base64 is a storage encoding, not an HTTP compression algorithm.

For an unknown encoding or malformed base64, import the remaining request and show `Body could not be decoded` with a reason and the retained encoded representation. That is a recoverable representation warning, not silent replacement with an empty body. Wrong field types reject import. Content encoding without retained text still means body not recorded.

## Timing rules

Preserve missing optional fields separately from `-1`, zero, and positive values. Require finite non-negative send, wait, and receive durations. Optional blocked, DNS, connect, and SSL durations may be absent, `-1`, or finite non-negative numbers. Reject other negative values and wrong types. JSON does not accept NaN or Infinity.

The overview shows entry.time as `Reported total`. The inspector lists captured timing fields with units. SSL is displayed as a detail within connect, not an additional sequential stage. A missing DNS timing does not identify the resolver or prove caching. A connect value of `-1` is not enough to reconstruct a connection history.

For a timing comparison, sum available blocked, DNS, connect, send, wait, and receive values, excluding SSL and unavailable values. If the difference from reported total exceeds 0.01 ms, show a consistency warning. This tolerance is a chosen display policy, not a guarantee from HAR. Never rewrite a captured duration to make totals agree.

If non-negative SSL exceeds a recorded non-negative connect duration, or SSL is available while connect is not, show a consistency warning and retain both fields. Do not derive negative TCP setup time. M1 does not derive a transport handshake diagram from these fields.

The overview's bars use recorded start times and reported totals. A zero-duration request has a labelled marker. Unavailable stage durations are text labels, not completed zero-length stages. Waiting is labelled `Waiting for response`; it includes delays beyond pure server computation.

## Deterministic identity and display

Headers and parameters retain source order. Hosts are grouped using a parsed URL host, with first-source-occurrence IDs and lexicographic display order. Request identity is unchanged by sorting. Empty captures show an empty-state message and no selectable row.

No M1 conclusion assigns a host purpose such as analytics or CDN. Optional serverIPAddress must parse as an IPv4 or IPv6 address when present. Optional connection must be a string; an empty string is preserved as exporter-reported empty metadata without grouping. Wrong types or malformed IP addresses reject import. These values are shown only if present and valid. They are labelled exporter-reported. There are no retry arrows, DNS answers, TLS certificate checks, HTTP/2 stream IDs, or inferred connection counts in M1.

## Resource and error policy

Use an explicit default input limit of 32 MiB, at most 10,000 entries, and at most 2 MiB of decoded response bytes per body. These are initial engineering limits, not measured performance guarantees. Check input size while reading, not only from a file-stat result. Reject input or entry limits before constructing an oversized session.

A base64 body exceeding its decoded limit remains encoded with a body warning; do not allocate its full decoded representation. Preview at most 16 KiB of body text or 256 binary bytes. Clearly label preview truncation and retain the imported representation within the input budget. Do not cap the number of headers silently. Their memory use remains subject to the bounded input and parser constraints.

An unreadable file, invalid profile, or import limit failure exits with status 1 and a contextual message. CLI misuse exits with status 2. Include the filename and field path, such as `log.entries[2].timings.wait`, without echoing credential values or whole body data. Terminal setup failures also restore any terminal state already changed. No importing or rendering operation executes shell text from the capture.

Render capture text as data. Escape control characters, including ESC, instead of emitting them as terminal commands. A text body may retain visible line breaks and tabs handled by the renderer; other controls are rendered as visible escapes. Apply the same protection to URLs, headers, filenames, and errors. Handle Unicode display width without splitting multibyte characters.

## Authoritative M1 screens

At 100 columns by 30 rows, the overview has a title, filename and request count, a scrollable request list, and a key legend. Each row shows source ID, method, host and path, status, reported total, and a relative timing bar. Selection uses a marker and contrast; colour alone never identifies the selected row. Longer URLs have a preview and remain available in full in the inspector.

The inspector has a summary plus scrollable sections for request fields, response fields, body previews, timings, and warnings. Captured empty bodies and unrecorded bodies have different labels. Long headers wrap or scroll without replacing their retained values. Warnings are text, not only red colouring.

Up and Down move selection; j and k are equivalents. Enter opens the selected request. Esc returns to overview. Inspector scrolling uses the same movement keys. Page Up/Page Down move by the current view’s visible rows. Home/End select the first/last request or host, or reach the beginning/end of the inspector. Movement stays within the active host filter; empty lists remain unselected. q quits from either screen. Help is available through ?. Resize redraws the current state. Below 80 columns or 24 rows, show `Terminal too small: use at least 80 columns × 24 rows`, allow q, and resume the same state when enlarged.

Import errors are visible before entering the TUI. The filename and input data remain local. Terminal restoration is part of correctness, not just visual polish.

## Acceptance cases

The suite must cover each case below with synthetic fixtures and fixed expected outcomes.

1. A single GET to example.test shows the recorded method, URL, duplicate headers in order, status 200, JSON text, and available durations.
2. Out-of-order and equal timestamps produce chronological rows with stable source-order ties, including timestamps with different offsets representing the same instant.
3. Empty entries produce a usable empty screen. Unknown fields and exporter extensions do not change understood data. An extension HTTP method is retained.
4. Omitted optional timing, `-1`, zero, and positive durations remain distinguishable. Negative send, null wait, and wrong-type receive reject import at the correct field path.
5. A synthetic connect duration of 30 ms containing SSL duration of 20 ms is not displayed as 50 ms setup. Mismatched totals and impossible SSL relationships produce warnings without changing recorded fields.
6. Missing, captured empty, ordinary text, base64 text, binary, unknown-encoding, malformed-base64, and oversized-decoded body cases have distinct specified outcomes. Captured text with Content-Encoding is not decompressed twice.
7. Form parameters preserve duplicates and missing values without inventing multipart bytes. Text and params may coexist or disagree; preserve both and reject invalid types in either representation.
8. Invalid JSON, duplicate JSON members, unsupported version, malformed URL, invalid timestamp, bad status, wrong known-field types, entry limit, and input limit fail deterministically. Error text identifies a field without leaking its value.
9. Missing connection metadata and TLS transcript fields never cause synthetic connection or certificate claims. Status 0 is not interpreted as 200 or as a specific failure.
10. Selection, Enter, Esc, scrolling, resize, help, q, and empty-state input have deterministic state transitions. Esc preserves overview selection and scroll position.
11. Fixed terminal buffers at 100 × 30 and 80 × 24 show required labels and readable selection. A small buffer shows the fallback. Unicode and malicious ESC input cannot corrupt output.
12. A controlled terminal integration check verifies restoration after q and after an injected rendering error. This real-terminal check is separate from pure deterministic UI tests.

## Learning completion

The corresponding book chapter must explain values, structs, vectors, typed IDs, optional fields, error results, pattern matching, and borrowing through the actual importer and renderer. You complete a small exercise about missing versus zero timings and explain why views borrow rather than copy the session.

Successful test output alone does not complete the learning goal. Document the implementing source revision, verification commands, skipped checks, and what you can explain about the design.

## Later contracts

M2 must define evidence and ambiguity rules before adding redirect or retry relationships. M3 must choose the HTTP stack and define replay, cookie selection, timeouts, cancellation, and instrumented observations. Phase C needs its own accepted packet formats and reassembly rules. Those decisions do not block this offline milestone.
