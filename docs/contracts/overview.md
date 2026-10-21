# Behaviour contracts

A behavior contract states the rules a contributor can check: which inputs Netscope accepts, what it shows, what it refuses, and which limits apply. The teaching chapters explain why a rule exists. A contract is the place to check the rule itself, for example the exact list of headers replay refuses or the exact packet link types the importer reads.

These pages define accepted inputs, supported actions, and the limits of their results. Their filenames retain the original milestone identifiers so existing links keep working.

## Inspecting a HAR

- [import and inspect a HAR](m1-har-inspection.md) defines the accepted HAR 1.2 subset, the import budgets, error messages, and the list and inspector screens.
- [host summaries](m2-host-summaries.md) defines the host view and how it filters the request list.
- [recorded protocols](m2-recorded-protocols.md) defines how recorded `httpVersion` labels are explained.
- [redirect suggestions](m2-redirect-suggestions.md) and [retry suggestions](m2-retry-suggestions.md) define which later requests are shown as possible follow-ups and what the analysis never claims.
- [URL search](m4-url-search.md) defines search over request URLs.

## Replaying a request

- [single-request replay](m3-single-request-replay.md) and [replay transport](replay-transport.md) define the review step, the transport, its limits, and cancellation.
- [request bodies](m3-request-bodies.md) extends replay to reviewed POST bodies.
- [response comparison](m3-response-comparison.md) defines how a result is compared with the recording.
- [saved drafts](m3-saved-drafts.md) defines the saved-draft file and what it never stores.
- [keyboard editing](m4-keyboard-editing.md) defines cursor movement inside editable fields.
- [installation](m4-installation.md) defines installation and the command line.

## Streams and packets

- [captured SSE](m5-captured-sse.md) and [WebSocket upgrade](m5-websocket-upgrade.md) define what Netscope reads from recorded event streams and upgrade handshakes.
- [live SSE](m6-live-sse.md) defines the bounded live event-stream observation.
- [packets](m7-packets.md) defines the classic PCAP subset and the recorded layers.
- [streams](m8-streams.md) defines directed TCP reassembly, plaintext HTTP framing, and signaling recognition.
