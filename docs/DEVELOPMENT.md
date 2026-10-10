# Working on Netscope

Run from the repository root:

```sh
python3 scripts/check.py
```

This checks rustfmt, every workspace target, Clippy, Rust tests and rustdoc examples, the binary build, and the generated book. All Rust commands use `--locked --offline`. The first toolchain installation still requires internet access. The importer uses serde/serde_json, chrono (without a system clock), URL parsing, and base64 decoding. The UI uses pinned Ratatui/Crossterm versions and Unicode width/segmentation helpers. When adding a justified dependency, update the lockfile deliberately and fetch it before offline verification.

## Current implementation

The core crate owns sessions, requests, representations, stable view orders, timing validation, and safe previews. The importer validates the M1 HAR profile. Run cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har in a terminal for interactive inspection; redirected output produces a chronological summary. Host summaries are derived once when a session is built. The Hosts screen opens chronological lists filtered by URL host. The network crate remains reserved.

Core timing tests exercise timing distinctions, valid precision, and invalid numbers. Rustdoc also checks chapter examples, including an intentional compiler rejection. The pure tests do not depend on elapsed time, network, random input, or terminal state; the separate pseudo-terminal check is described below. Importer tests consume the synthetic fixtures and mutate fixed values to cover malformed input, body states, warnings, identity, and limits. Binary integration tests verify exit codes and escaped output.

## Lint policy

Every crate inherits the root lint configuration. Unsafe code is forbidden. Ignored must-use results are denied. Clippy's default `all` group and missing public documentation are warnings locally; the check command treats warnings as errors. Clippy denies unwrap and expect, including in tests unless a narrow, explained exception is required. Current tests use assertions without exceptions.

Do not suppress a whole crate to silence a warning. Use a local `#[expect(..., reason = "...")]` when a justified exception cannot be removed; this records the reason and detects when the expected warning disappears. Tests may panic to signal an assertion failure. User input and ordinary I/O failures must return contextual errors.

Functions and types should express the work they do. Add abstractions when a real behaviour needs substitution, such as a clock or transport. Do not create generic layers merely to exercise a Rust feature. Keep core logic free of terminal and network I/O.

## Writing and examples

The book assumes no Rust knowledge. Its order follows the learner's questions: observations, basic syntax, ownership, collections, missing values, then file reading and screen behaviour. Numeric filename prefixes are stable chapter identifiers; chapters.json defines the actual reading order.

Read [the book's writing principles](chapters/05-decisions.md) before adding or revising a lesson. Root AGENTS.md records the same audience requirement for future work. Start from a concrete question and build a coherent explanation before choosing source examples. Code structure, recent implementation changes, and test output do not dictate the outline. Explain the mechanism in ordinary language, introduce terms and symbols before relying on them, and use diagrams where they help the reader reason about a relationship.

Review in two passes. First, read without source code and check that a beginner can follow the example and predict its consequence. Then verify the claims and source links against the implementation. Keep status reports and detailed check instructions in reference material. Comments explain a local reason, assumption, or consequence; long lessons belong in the book.

Edit the Markdown chapters, run `python3 scripts/build_docs.py`, and check the result with `python3 scripts/check_docs.py`. If examples change, run `cargo test --workspace --doc --locked --offline`. Rustdoc checks the examples included from the basic-language chapters. Compilation checks do not certify beginner readability or the accuracy of uncompiled illustrations.

## CI and evidence

The GitHub workflow runs the same check script on Linux. It is configured but has not been executed by GitHub. This directory has no Git history yet; no commit or push was performed. Record actual local outcomes and limitations in VERIFICATION.md. M1 remains incomplete until the contract's full acceptance cases and learning explanation are satisfied.

## Terminal verification

`cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har` starts the viewer when both input and output are terminals. Up/Down or j/k move; Enter inspects; Esc returns; ? toggles help; q quits. Press h for host summaries, Enter to filter, and a to return to all requests. Ctrl+C also requests a clean exit. Redirected output uses the safe summary without changing terminal state.

`python3 scripts/check_terminal.py` is a separate POSIX pseudo-terminal check. It builds the terminal_probe example, compares input/output modes after keyboard exit and an injected rendering error, and verifies alternate-screen exit and cursor visibility. The check has a bounded process timeout and is separate from the deterministic buffer suite. On macOS it excludes the transient PENDIN input-retype bit when comparing mode flags. GitHub's Linux workflow runs this check after the pure suite.

The multi-host sample is tests/fixtures/har/multi-host.har. The [M2 host-summary contract](contracts/m2-host-summaries.md) defines grouping, count meanings, duration overflow, and navigation. Redirect suggestions are defined by the [redirect contract](contracts/m2-redirect-suggestions.md). Open tests/fixtures/har/redirects.har and scroll to REDIRECT EVIDENCE in request #0 for ambiguity, #3 for a single cross-host candidate, and #5 for a missing target. Candidates are source IDs; return to the list to inspect them. Core now uses the existing locked url crate for pure URL resolution, with no network operations. Conservative retry suggestions are now defined by the [retry contract](contracts/m2-retry-suggestions.md). Open tests/fixtures/har/retries.har and inspect #0 for ambiguity, #3 for a single candidate, #5 for an excluded method, #6 for unknown status, and #7 for no match. Suggestions are offline comparisons, not instructions to repeat a request.
