# M2 recorded protocol explanations

Explain the request and response httpVersion strings already retained by the importer. Preserve their original spelling and escape controls on display. Recognition is a local display policy, not validation of actual network traffic. Unknown nonempty labels remain valid imported data.

## Recognition

Recognize exactly HTTP/1.0, HTTP/1.1, HTTP/2, HTTP/2.0, h2, HTTP/3, HTTP/3.0, and h3. The HTTP/2 spellings share a classification; the HTTP/3 spellings share another. All other strings are unrecognized, including case changes, added whitespace, prefix matches, draft identifiers, and future versions. Do not derive a version from the URL scheme, status, headers, timing, IP address, or connection metadata.

Classify both fields independently. When both are recognized and classify differently, explicitly label disagreement without choosing a winner or claiming an upgrade, downgrade, or intermediary. When either is unknown, say that comparison is unavailable. Equivalent recognized aliases agree. Original labels remain visible even when classifications agree.

## Explanation and limits

Add a PROTOCOL NOTES section after import warnings and before relationship evidence. Identify each original label and its recognized version, then show a short general explanation for each distinct recognized version. HTTP/1.0 receives a limited historical-version explanation; HTTP/1.1 describes textual start lines and headers; HTTP/2 describes frames and streams over TCP; HTTP/3 describes streams over QUIC. Mark these explanations as general protocol behaviour, not observed messages.

Always state that these HAR fields do not provide packet/frame traces, stream IDs, compression savings, or proof of connection reuse. Make no inferred latency advantage, handshake reconstruction, server capability claim, or performance diagnosis. Request and response version differences are metadata disagreement, not a measured protocol transition. No network activity occurs. No new HAR fields or warnings are consumed or added, and relationship matching remains unchanged.

The book explains message versus transport, streams and interleaving, and why a version label cannot establish what ran concurrently. Its diagram is illustrative and visible in both Markdown and the offline reader. A beginner Rust example introduces string slices and exact pattern alternatives after the reader understands why recognition must be selective.

Protocol references: [HTTP/1.1 message format](https://www.rfc-editor.org/rfc/rfc9112.html#name-message-format), [HTTP/2 overview](https://www.rfc-editor.org/rfc/rfc9113.html#name-http2-protocol-overview), and [HTTP/3 overview](https://www.rfc-editor.org/rfc/rfc9114.html#name-overview-of-http3).

## Acceptance

Test every accepted spelling, unknown labels, case/whitespace/prefix exclusions, equivalent aliases, disagreement, one-sided unknown values, preserved raw labels, and control escaping. Fixed imported fixtures must show the same outcomes as the pure classifier. Verify inspector scrolling and complete terminal buffers at 80 × 24 and 100 × 30. Existing relationship test expectations must remain unchanged; their saved bottom-of-inspector buffers may gain protocol context and new scroll line numbers. Rust teaching examples compile through rustdoc. No benchmark or browser rendering claim follows from these checks.
