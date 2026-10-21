# Netscope milestone status

Version 0.2.0 includes the M1–M8 implementation scopes and the subsequent interface and book refinements, reviewed October 20, 2026. Version 0.1.0 remains the earlier source release snapshot. The original local specification and concept-coverage matrix remain excluded from Git. They are historical records from October 9, 2026: this file is the current status, and [defending the design](chapters/34-defending-the-design.md) records the current coverage of TLS, keep-alive, and content negotiation. Contracts and the verification ledger supply exact scope and evidence; implementation checks do not substitute for learner feedback.

| Milestone | Implemented scope | Reference |
| --- | --- | --- |
| M1 | HAR import, request navigation, recorded content/timings | [HAR contract](contracts/m1-har-inspection.md) |
| M2 | Host summaries, conservative redirect/retry evidence, reported protocols | [Host contract](contracts/m2-host-summaries.md), [protocol contract](contracts/m2-recorded-protocols.md) |
| M3 | Reviewed GET/POST replay, cancellation, comparison, saved drafts | [Replay](contracts/m3-single-request-replay.md), [bodies](contracts/m3-request-bodies.md), [saved requests](contracts/m3-saved-drafts.md) |
| M4 | Keyboard editing/search, native export compatibility, installation | [Search](contracts/m4-url-search.md), [installation](contracts/m4-installation.md) |
| M5 | Captured SSE and classic WebSocket upgrade metadata | [SSE](contracts/m5-captured-sse.md), [upgrades](contracts/m5-websocket-upgrade.md) |
| M6 | Bounded live SSE and deeper HTTP/2 teaching with scoped evidence | [Live streams](contracts/m6-live-sse.md) |
| M7 | Bounded classic PCAP and supported recorded packet layers | [Packets](contracts/m7-packets.md) |
| M8 | Directed TCP gaps/overlaps, plaintext HTTP chunk boundaries and available signaling | [Streams](contracts/m8-streams.md) |

M1–M8's planned implementation scopes are present. The [verification ledger](VERIFICATION.md) records executed checks, limitations and chapter reviews. The [34-chapter book](chapters/01-start-here.md) covers the mechanisms and architectural alternatives, with stable previous chapter identifiers. Ongoing editorial and learner review can still identify improvements.

Explicitly deferred Phase D: proxy interception and certificate-authority management. Broader formats, full protocol conformance, decryption, HTTP/2 live sending, WebSocket frame clients, reconnecting SSE and richer packet inference are expansions of scope, not hidden unfinished acceptance cases. Each needs its own concrete contract and evidence before implementation claims.
