# Release readiness: 0.2.0

<a id="release-readiness-010"></a>

Release candidate prepared October 20, 2026. This is a tagged source release containing the Cargo binary and mdBook sources. It does not publish crates.io packages or attach prebuilt executables. Version 0.1.0 remains an unchanged historical tag.

## Release scope

M1–M8 are implemented within their [contracts](contracts/overview.md): HAR inspection and search, reviewed GET/POST replay, comparison and saved drafts, captured SSE and WebSocket upgrade metadata, bounded live SSE, classic PCAP, and directed TCP stream evidence. The release includes the revised split inspector, journey, packet/stream workbenches, replay fields and review sections, and the beginner book. [Milestone status](MILESTONES.md) separates these features from future expansion.

## Required checks

Run `python3 scripts/check.py` for formatting, all-target compilation, warnings-denied Clippy, workspace tests and Rust examples, the frozen browser campaign, and documentation links. Build the reader with `mdbook build docs`. Run `python3 scripts/check_terminal.py` for the development restoration probe and `python3 scripts/check_install.py` for an isolated release installation outside the checkout.

The installation check runs the installed binary through CLI errors, navigation/search, small-window recovery, reviewed local GET/POST, saved drafts, comparison, cancellation, live/captured SSE and packet/stream navigation. Terminal watchdogs detect hangs; they do not establish latency guarantees. The release commit must also pass the Linux Foundation checks workflow before tagging. Actual results belong in the [verification ledger](VERIFICATION.md).

## Documentation and interface review

The closing pass reconciles README and installation scope with classic-PCAP link types, replaces obsolete replay-pane and stream-range descriptions, and checks the maintained terminal workflows. Keyboard controls and detailed explanations remain visible at supported sizes; no new feature or interaction model is introduced in this release pass. Historical verification entries describe what was tested at their own dates rather than being rewritten as current claims.

## Release boundaries

Local validation targets macOS arm64; hosted CI targets Ubuntu. Windows interactive support remains unverified. Source installation requires Rust and platform build tools. Replay sends HTTP/1.1, with verified TLS for HTTPS, bounded bodies and deadlines; it does not automatically follow redirects, retry, discover proxies or maintain browser cookies. Live SSE does not reconnect. WebSocket inspection covers captured upgrades, not a frame client. PCAPNG, decryption and proxy interception remain outside scope.

Frozen browser recordings and synthetic packet fixtures cover documented cases, not universal format conformance. This release pass does not repeat the native browser recording campaign or establish external-service reliability, exhaustive race freedom, performance guarantees or beginner-comprehension results. See [release notes](https://github.com/sidsharma96/netscope/blob/main/CHANGELOG.md) for the shipped behavior and [verification](VERIFICATION.md) for executed evidence.
