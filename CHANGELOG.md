# Release notes

## 0.2.1 — October 20, 2026

- Name the installable package `netscope`, retaining the executable name and source directory. Prepare versioned registry dependencies and package descriptions.
- Apply MIT OR Apache-2.0 licensing and include both licenses in every package. Include embedded book lessons as regular archive files, with the book remaining their single source.
- Verify extracted package builds and archive contents in CI. The install command is `cargo install netscope --locked`.

## 0.2.0 — October 20, 2026

This source release completes the agreed M1–M8 scope and includes the interface and book refinements below. Install from tag `v0.2.0` using Cargo. There are no crates.io packages or prebuilt executables; Windows interactive behavior remains unverified.

- Organize replay into Method, URL, Headers and Body fields, with Summary / Headers / Body review sections. Keep explicit send confirmation, masked header values and concise beginner guidance.
- Make stream inspection follow selected exact ranges, including gaps and conflicts, with text/hex previews and expanded help. Declutter replay and packet guidance at compact terminal sizes.
- Support BSD loopback and Linux cooked SLL/SLL2 in classic PCAP, alongside Ethernet/VLAN and selected raw-IP inputs. PCAPNG and encrypted-application decoding remain outside scope.

- Add packet/stream learning panes for TCP flags, ACK direction, sequence positions and byte-coverage maps, with a separate complete recorded report. Extend the beginner book with worked acknowledgement and range examples.

- Clarify inspection for beginners with full section names, reading prompts, host outcome bars, packet direction guides and numbered replay steps. Add a deterministic active-operation indicator with reduced-motion support.

- Publish the book with mdBook 0.5.4 from `docs/SUMMARY.md`, grouped into parts, with search, a reading-path front page, a glossary and a three-level question bank whose answers link to the teaching sections. Teaching chapters end with generated questions. The custom HTML reader and its builder are retired; the documentation checks now run on the Markdown and also verify heading anchors and question format. A workflow builds the site and deploys it to Cloudflare Pages.

- Replace the journey's full-width text report with a compact exchange diagram, selectable timing stages and a focused explanation pane. Show partial timing sums and discrepancies without inventing missing durations; keep stage navigation and explanation paging separate.

- Add a default split request inspector on wide terminals, focused sections, an optional explanatory journey, and a replay workbench with a persistent operation strip. Keep one-pane navigation on smaller supported terminals and the complete captured report under All. Update the beginner walkthrough and lessons on focus, ownership, and review.

- Inspect captured SSE text as ordered derived events, with persistent ID state, multiline data, retry instructions, unfinished tails, and explicit analysis limits. The original response remains available.

- Inspect native WebSocket upgrade metadata, including ws/wss URLs, with explicit header-shape and missing-frame limits. Add frozen Chrome/Firefox SSE and WebSocket exports.
- Review and observe one live SSE GET with local boundary times, cancellation, verified HTTPS and explicit time/byte/event limits. No reconnection. Add beginner lessons on upgrades, live observations and HTTP/2 streams, flow control and HPACK.

- Inspect bounded classic PCAP records and supported Ethernet/VLAN/IP/TCP/UDP layers with explicit capture cuts and unsupported cases.
- Inspect directed TCP epochs, gaps, repeated/conflicting overlaps, wrap-around, conditional plaintext HTTP/1.1 chunk/length boundaries, DNS answers and SDP/STUN candidates. Add five beginner lessons and deterministic packet/UI/terminal/install checks.

- Retain narrow nonstandard blocked timings and absent addresses on failed WebSocket exports, following a 120-scenario native browser campaign. Add captured evidence, offline regression checks, production probes and documented representation/dispatch discrepancies.

## 0.1.0 — October 13, 2026

The first Netscope release inspects local HAR captures and prepares explicitly reviewed HTTP requests. Install from the tagged source checkout; this release has no registry package or prebuilt binaries.

- Browse requests and inspect recorded headers, bodies, timings, and warnings. Host summaries and literal URL search narrow the view while preserving request identity.
- Examine conservative redirect/retry suggestions and recorded protocol labels, with missing evidence and ambiguity visible.
- Edit, review, and send one GET or POST over HTTP/1.1, using verified TLS for HTTPS. Compare the new response with the captured response and retain partial results after cancellation or limits.
- Save and independently reopen request drafts. Known credential headers are excluded from saved files; other values can still contain sensitive data.
- Read the 24-chapter beginner Rust book on GitHub or in the generated offline reader, including design choices and alternatives.

Replay has a 10-second total deadline, a 2 MiB response-body limit, and a 16 KiB UTF-8 POST-body limit. It does not follow redirects, retry, discover proxies, maintain a browser cookie jar, or decompress response bodies. Browser HAR recordings can already contain decoded body text; imported content remains separate from replay transport.

Live SSE/WebSocket capture, HTTP/2 replay, and packet inspection are future work. Frozen Chrome/Firefox exports establish compatibility for the recorded cases, not every browser export. See [installation](docs/INSTALLATION.md), [compatibility](docs/HAR_COMPATIBILITY.md), and [release evidence](docs/RELEASE_READINESS.md).
