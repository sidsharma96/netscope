# Release notes

## Unreleased

- Inspect captured SSE text as ordered derived events, with persistent ID state, multiline data, retry instructions, unfinished tails, and explicit analysis limits. The original response remains available.

- Inspect native WebSocket upgrade metadata, including ws/wss URLs, with explicit header-shape and missing-frame limits. Add frozen Chrome/Firefox SSE and WebSocket exports.
- Review and observe one live SSE GET with local boundary times, cancellation, verified HTTPS and explicit time/byte/event limits. No reconnection. Add beginner lessons on upgrades, live observations and HTTP/2 streams, flow control and HPACK.

## 0.1.0 — October 13, 2026

The first Netscope release inspects local HAR captures and prepares explicitly reviewed HTTP requests. Install from the tagged source checkout; this release has no registry package or prebuilt binaries.

- Browse requests and inspect recorded headers, bodies, timings, and warnings. Host summaries and literal URL search narrow the view while preserving request identity.
- Examine conservative redirect/retry suggestions and recorded protocol labels, with missing evidence and ambiguity visible.
- Edit, review, and send one GET or POST over HTTP/1.1, using verified TLS for HTTPS. Compare the new response with the captured response and retain partial results after cancellation or limits.
- Save and independently reopen request drafts. Known credential headers are excluded from saved files; other values can still contain sensitive data.
- Read the 24-chapter beginner Rust book on GitHub or in the generated offline reader, including design choices and alternatives.

Replay has a 10-second total deadline, a 2 MiB response-body limit, and a 16 KiB UTF-8 POST-body limit. It does not follow redirects, retry, discover proxies, maintain a browser cookie jar, or decompress response bodies. Browser HAR recordings can already contain decoded body text; imported content remains separate from replay transport.

Live SSE/WebSocket capture, HTTP/2 replay, and packet inspection are future work. Frozen Chrome/Firefox exports establish compatibility for the recorded cases, not every browser export. See [installation](docs/INSTALLATION.md), [compatibility](docs/HAR_COMPATIBILITY.md), and [release evidence](docs/RELEASE_READINESS.md).
