# M2 redirect suggestions

This slice derives possible relationships from a complete imported capture. It sends no traffic and does not assert that one request caused another. Captured headers remain unchanged. This contract extends M1 without adding consumed HAR fields: response.redirectURL remains ignored exporter metadata. Location headers alone supply targets, including when exporter metadata disagrees.

## Evidence and matching

Consider only statuses 301, 302, 303, 307, and 308. Other statuses are labelled outside this analysis. Match the response header name Location without ASCII case sensitivity. Require exactly one occurrence; repeated identical values are still ambiguous. Missing, empty, conflicting, or unsupported targets receive a fixed explanation, not a guessed URL or an import failure.

Remove leading and trailing spaces and tabs from Location. Reject remaining whitespace, controls, and backslashes in that value, and refuse to compare source/candidate URLs containing whitespace, controls, or backslashes. The two cases differ in effect. A source URL with such a character makes the source's evidence Unavailable. A candidate URL with one is left out of the URL index built by [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L35-L40), so it can never be suggested, but it does not make any result Unavailable. Resolve relative values using the locked url crate against the source URL. Require an HTTP(S) result with a host. This is URL-library parsing with additional conservative exclusions, not a complete RFC URI validator. A comma inside a single Location is treated as URL data, not split into alternative targets.

Compare the url crate's serialized URLs after removing fragments. Its parsing normalizes host case, default ports and path dot segments. Do not sort query fields, decode additional percent escapes, ignore a nondefault port, merge HTTP with HTTPS, or discard credentials. Preserve original URLs and headers separately. Fragments identify a position within a resource rather than a distinct HTTP request target.

Candidate methods are case-sensitive. For 301/302 permit the original method or POST changing to GET; for 303 permit GET or HEAD; for 307/308 require the original method. These checks establish compatibility only; bodies, credentials, browsing context, and client intent are not verified. [HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-redirection-3xx) and [Location](https://www.rfc-editor.org/rfc/rfc9110.html#name-location) provide the protocol reference.

Require a candidate start strictly later than the source start and at or after its reported end (start plus total duration), with at most 30,000 ms between that end and the candidate start. Both ends of that latter window are inclusive. Equal start instants never establish direction, even when duration is zero. Compute timestamp differences with integer nanoseconds before converting to milliseconds; compare the resulting f64 interval with the recorded f64 duration. No clock, timezone setting, or source-position guess participates.

The 30-second window and reported-end condition are Netscope policy, not HTTP requirements. They can miss real follow-ups, including a client acting before the full response body finishes. Bad capture timings can distort the result. Zero matches therefore means only that no entry passed these rules. One match is a suggestion; two or more are ambiguous. Several sources may suggest the same candidate. No automatic chain or causal graph is constructed.

## Bounds and presentation

Build evidence once with the session, across all hosts. A temporary URL index retains chronological source IDs; binary search skips starts at or before the source and starts before its reported end, and searching stops past the window. Store the first 20 matching IDs per source and mark whether any additional match exists. Ordering is by instant, then source position. Stop after finding a 21st match. This bounds stored candidates to 20 per request, but worst-case matching work can still be quadratic within the importer limit of 10,000 entries. Selected local workloads are measured in the [large-capture reference](../LARGE_CAPTURES.md); no universal timing bound is claimed.

The inspector appends a REDIRECT EVIDENCE section after warnings. It shows the resolved target, matching rule, ambiguity or absence, and candidate source IDs with recorded methods and timestamps. Values receive terminal control escaping and normal wrapping. Candidate IDs can be found in the request list; this slice does not add a jump key. Esc returns to the same selection. Clear a host filter with a from the list to find a candidate on another host. Non-interactive output remains the capture summary.

The core crate uses the same locked url dependency as the importer for pure URL parsing and resolution. It still performs no I/O. Invalid manually constructed durations produce unavailable evidence; the importer remains responsible for general request validity.

## Acceptance

Fixed tests cover relative resolution, mixed-case header names, host/default-port normalization, fragment removal, stable ambiguity ties, missing and duplicate headers, unsupported targets/statuses, each method policy, query order, scheme/port/path distinctions, overlap, equal starts, zero duration, both time-window boundaries, bounded candidates, ignored exporter metadata, and cross-host matching. View checks cover the evidence labels and reaching the section through scrolling at two widths without losing the overview selection. No external services, random inputs, or wall-clock reads participate.

The learning chapter must distinguish a recorded instruction from an inferred relationship before presenting Rust. Its illustrative enum should preserve the difference between a search that found nothing and a search that could not be attempted. Compile its example alongside the existing book examples.
