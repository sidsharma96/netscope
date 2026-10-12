# M2 retry suggestions

This offline analysis suggests later requests worth comparing with a recorded error response. It neither recommends retrying nor sends traffic. Repeated requests can be polling, manual reloads, independent actions, or retries. No result proves which explanation applies.

## Eligibility and comparison

Only source statuses 408, 429, 500, 502, 503, and 504 initiate a search. This is a Netscope selection policy, not an exhaustive list of retryable responses or a diagnosis of temporary failure. Status zero receives an unavailable explanation: no status was recorded, so a network failure cannot be assumed. Other statuses are outside this analysis.

Require the source and candidate to have the same case-sensitive method, either GET or HEAD, request bodySize reported as zero, and no captured postData representation. Missing and unavailable sizes are insufficient evidence; even explicitly empty postData is outside this initial policy. Non-GET/HEAD methods are excluded rather than inferred safe to repeat. Source duration must be finite and non-negative.

Compare URLs using the same conservative URL parser and fragment-removal rules as the [redirect contract](m2-redirect-suggestions.md). Compare captured request headers, query fields, and cookies exactly, including order, spelling, duplicates, and every retained cookie attribute. Do not merge or sort fields, ignore authorization values, or infer a complete browser cookie jar. Changed request metadata excludes a candidate even when a real retry may have changed it. Equal empty arrays do not establish that the exporter captured all fields. Body size is exporter evidence, not proof of actual wire bytes. Request protocol, response status/body, server IP, connection labels, and response headers do not need to match.

Candidates must start strictly after the source and from 0 through 30,000 ms after its reported end, inclusive. Equal starts are excluded, including zero-duration sources. Timestamp subtraction uses integer nanoseconds before conversion to floating-point milliseconds. This window can miss real retries before reported completion or after a longer delay. No clock is read. Retry-After remains visible as a captured response header but is not interpreted; suggestions do not establish compliance with it or any client retry policy. A later candidate may have any response status, including another error or zero; it is never labelled a successful retry.

## Results and bounds

Compute evidence once per session. Keep separate states for outside scope, unavailable evidence with a fixed reason, and a completed search with candidate IDs. Store at most 20 IDs in chronological order, with source order for timestamp ties. Finding a 21st match sets a visible additional-candidates flag; never select a winner. Zero candidates is a search result, not proof that the client did not retry. Several source requests can suggest the same later entry; there is no inferred attempt number or chain.

Use a temporary URL-and-method index; use binary search to skip starts at or before the source and starts before its reported end, then stop beyond the time window. Candidate storage is bounded; worst-case comparisons remain quadratic within the importer's 10,000-entry limit. Comparisons of captured lists also cost time proportional to those lists. Selected local workloads are measured in the [large-capture reference](../LARGE_CAPTURES.md); no universal timing bound is claimed.

For eligible errors and missing status, append RETRY EVIDENCE after redirect evidence in the inspector. Show source status/method, applicable rules, unavailability or ambiguity, and candidate IDs, methods, and recorded timestamps. Do not repeat request-header values in the evidence summary. Candidate text receives control escaping and existing wrapping. Statuses outside the selected set have no extra retry section. Esc preserves list selection; candidates can be inspected by source ID from the list. Non-interactive output remains the summary.

## Acceptance

Use synthetic fixed inputs for the status/method policy, missing and nonzero body sizes, explicit postData, exact metadata comparisons, URL distinctions, source ordering, ties, overlap, window boundaries, zero and invalid durations, candidate bounds, and source status zero. Check that candidate success is not required and Retry-After does not silently change the policy. Compare complete terminal buffers at 80 × 24 and 100 × 30; test scrolling and returning to the selected source.

The beginner lesson begins with repeated requests and alternative explanations. It introduces equality and Boolean conditions only after establishing what comparison means. Its diagram must remain visible in Markdown on GitHub and in the offline reader, and its Rust example must compile through rustdoc.
