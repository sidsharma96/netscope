# Netscope

A Rust terminal application for inspecting HTTP and supported packet captures and sending reviewed requests, with a beginner book explaining how the program works and why it is designed this way.

## Install and start

Install the registry package with `cargo install netscope --locked` (Rust 1.92 or newer). Cargo compiles the source on your machine; this is not a prebuilt-binary download.

Or install from a source checkout:

```sh
cargo install --path crates/netscope-tui --locked
netscope --help
netscope tests/fixtures/har/single-request.har
```

The command is `netscope`; Cargo installs it into its configured bin directory, which must be on PATH. See [installation](docs/INSTALLATION.md) for prerequisites, selecting a release version, offline builds, and removal. Opening a capture or saved draft never sends a request.

## Inspect a capture

Enter opens a request; Esc returns. Use arrows or j/k to move, Page Up/Page Down to move by a screen, and Home/End to reach either end. Press h for host summaries and Enter to filter by a host. From Requests, / edits a case-sensitive literal substring search of the full URL. Enter applies; Esc cancels; an empty query clears search. Search combines with the host filter, and a clears both. Press ? for keys and q to quit.

Terminals at least 100 columns wide keep the request list beside tabbed details; narrower supported terminals show one pane at a time. Tab switches list/detail focus. Use 1–6 for Summary, Headers, Body, Timing, Evidence, and All; g opens an optional explanatory journey. Replay uses an Edit → Review → Send → Result workbench. See the [layout guide](docs/UI_LAYOUT.md).

Headers, body representations, timings, protocol labels, and warnings remain tied to the recorded request. Redirect and retry suggestions show their evidence and ambiguity. [Browser compatibility](docs/HAR_COMPATIBILITY.md) describes frozen Chrome/Firefox cases; [large-capture checks](docs/LARGE_CAPTURES.md) record tested limits and local measurements. Responses labelled text/event-stream now show derived events from retained text, including unfinished tails and analysis limits, without inventing arrival times. Try tests/fixtures/har/sse.har and read the [event-stream lesson](docs/chapters/25-reading-event-streams.md). Redirected output prints a safe summary.

## Prepare a request

Press r from an eligible GET/POST inspector to create a separate editable draft. Review the destination, headers, and body before Send. Only sending from Review starts network traffic. The result can be compared with the captured response; cancellation retains available partial data.

Press w from a valid draft to name a file and confirm its save preview. Reopen with `netscope --draft request.json`. Known credential headers are excluded from saved files. Use the [local testing walkthrough](docs/DEVELOPMENT.md#try-a-local-send) to try GET, JSON POST, and save/reopen.

Replay uses HTTP/1.1 over HTTP or verified HTTPS. POST text is limited to 16 KiB; responses have a 10-second total deadline and a 2 MiB body limit. Redirects, retries, proxy discovery, a browser cookie jar, and decompression are absent. The [release notes](CHANGELOG.md) describe the released versions and its boundaries.

## Learn and contribute

Start with the [field notes](docs/README.md), which give reading paths for newcomers, experienced engineers, and anyone preparing to defend the design. The book is built with mdBook from `docs/SUMMARY.md` and published as a private site at https://netscope-docs.sidsharma.dev; the same Markdown reads on GitHub. It includes a [glossary](docs/GLOSSARY.md) and a three-level [question bank](docs/questions/newcomer.md) whose answers link back to the teaching sections. The [architecture decision record](docs/ARCHITECTURE.md) explains constraints, alternatives, evidence, and unresolved questions. [Defending the design](docs/chapters/34-defending-the-design.md) answers the questions an expert reviewer would ask, records known defects, and explains how to make a packet capture Netscope can read. The binary needs no documentation website.

Run `cargo fetch --locked` once to obtain dependencies, then `python3 scripts/check.py` for formatting, compilation, Clippy, tests, build, and book checks. `python3 scripts/check_install.py` verifies a temporary release installation and interactive workflows outside the checkout. See [development](docs/DEVELOPMENT.md) and [release evidence](docs/RELEASE_READINESS.md) for scope. Original planning documents and HTML mockups remain local references excluded from Git; they describe the project as of October 9, 2026 and are superseded by [milestone status](docs/MILESTONES.md) and the book.

Netscope also inspects captured SSE and WebSocket upgrades and offers separately reviewed live SSE GET observations (30 seconds, 64 KiB, 100 events). See the [local walkthrough](docs/DEVELOPMENT.md) and [M6 contract](docs/contracts/m6-live-sse.md). HTTP/2 teaching uses recorded H2 evidence and labelled illustrations; live sending remains HTTP/1.1.

Inspect a classic packet file with `netscope --pcap tests/fixtures/pcap/streams.pcap`. Enter shows layers; s shows directed TCP streams with gaps, conflicts, plaintext chunk boundaries and available signaling. See the [packet walkthrough](docs/DEVELOPMENT.md#inspect-packets-and-streams), [supported subset](docs/contracts/m7-packets.md) and [milestone status](docs/MILESTONES.md). Encrypted applications are not decoded. Classic PCAP captures from macOS loopback (`lo0`) and Linux cooked SLL/SLL2 (`-i any`) are supported. PCAPNG is not supported; the [design defense](docs/chapters/34-defending-the-design.md#why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do) explains how to record a readable capture.

A [120-scenario native compatibility campaign](docs/COMPATIBILITY_CAMPAIGN.md) checks REST, GraphQL-shaped HTTP, SSE and WebSocket traffic from Chrome/Firefox, including HTTPS/H2 and WSS, with retained captures and explicit discrepancies. The normal suite rechecks it offline.


## License

Netscope's original code and documentation are available under either the [MIT license](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option. Third-party dependencies and any separately attributed material retain their own licenses. This licensing applies to the current source; the existing v0.2.0 tag is unchanged.
