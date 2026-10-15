# Behaviour contracts

A contract states exactly what one milestone promises: which inputs Netscope accepts, what it shows, what it refuses, and which limits apply. The teaching chapters explain why a rule exists. A contract is the place to check the rule itself, for example the exact list of headers replay refuses or the exact packet link types the importer reads.

Contracts were written when each milestone was agreed and are kept current as the behaviour changes. Where a later contract extends an earlier one, the later page says which rules it replaces. The [milestone status](../MILESTONES.md) table maps each milestone to its contracts.

## Inspecting a HAR

- [M1: import and inspect a HAR](m1-har-inspection.md) defines the accepted HAR 1.2 subset, the import budgets, error messages, and the list and inspector screens.
- [M2: host summaries](m2-host-summaries.md) defines the host view and how it filters the request list.
- [M2: recorded protocols](m2-recorded-protocols.md) defines how recorded `httpVersion` labels are explained.
- [M2: redirect suggestions](m2-redirect-suggestions.md) and [M2: retry suggestions](m2-retry-suggestions.md) define which later requests are shown as possible follow-ups and what the analysis never claims.
- [M4: URL search](m4-url-search.md) defines search over request URLs.

## Replaying a request

- [M3: single-request replay](m3-single-request-replay.md) and [replay transport](replay-transport.md) define the review step, the transport, its limits, and cancellation.
- [M3: request bodies](m3-request-bodies.md) extends replay to reviewed POST bodies.
- [M3: response comparison](m3-response-comparison.md) defines how a result is compared with the recording.
- [M3: saved drafts](m3-saved-drafts.md) defines the saved-draft file and what it never stores.
- [M4: keyboard editing](m4-keyboard-editing.md) defines cursor movement inside editable fields.
- [M4: installation](m4-installation.md) defines installation and the command line.

## Streams and packets

- [M5: captured SSE](m5-captured-sse.md) and [M5: WebSocket upgrade](m5-websocket-upgrade.md) define what Netscope reads from recorded event streams and upgrade handshakes.
- [M6: live SSE](m6-live-sse.md) defines the bounded live event-stream observation.
- [M7: packets](m7-packets.md) defines the classic PCAP subset and the recorded layers.
- [M8: streams](m8-streams.md) defines directed TCP reassembly, plaintext HTTP framing, and signaling recognition.
