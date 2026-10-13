# Working on Netscope

Run from the repository root:

```sh
python3 scripts/check.py
```

This checks rustfmt, every workspace target, Clippy, Rust tests and rustdoc examples, the binary build, and the generated book. All Rust commands use `--locked --offline`. The first toolchain installation still requires internet access. The importer uses serde/serde_json, chrono (without a system clock), URL parsing, and base64 decoding. The UI uses pinned Ratatui/Crossterm versions and Unicode width/segmentation helpers. When adding a justified dependency, update the lockfile deliberately and fetch it before offline verification.

## Current implementation

The core crate owns sessions, requests, representations, stable view orders, timing validation, and safe previews. The importer validates the M1 HAR profile. Run cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har in a terminal for interactive inspection; redirected output produces a chronological summary. Host summaries are derived once when a session is built. The Hosts screen opens chronological lists filtered by URL host. The network crate sends reviewed GET or POST requests using Hyper, Tokio, and rustls. Its [transport decision](contracts/replay-transport.md) records limits and cancellation behaviour.

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

The GitHub workflow is configured to run the same check script on Linux. The repository is on GitHub and has committed history; consult the verification record for dated local checks rather than assuming a hosted run passed. Record actual local outcomes and limitations in VERIFICATION.md. M1 remains incomplete until the contract's full acceptance cases and learning explanation are satisfied.

## Terminal verification

`cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har` starts the viewer when both input and output are terminals. Up/Down or j/k move; Enter inspects; Esc returns; ? toggles help; q quits. Press h for host summaries, Enter to filter, and a to return to all requests. From Requests, / edits a literal, case-sensitive full-URL search; Enter applies and Esc cancels. Ctrl+u clears the edit. Search combines with the host filter; an empty query clears only search, while a clears both filters. Ctrl+C also requests a clean exit. Redirected output uses the safe summary without changing terminal state.

`python3 scripts/check_terminal.py` is a separate POSIX pseudo-terminal check. It builds the terminal_probe example, compares input/output modes after keyboard exit and an injected rendering error, and verifies alternate-screen exit and cursor visibility. The check has a bounded process timeout and is separate from the deterministic buffer suite. On macOS it excludes the transient PENDIN input-retype bit when comparing mode flags. GitHub's Linux workflow runs this check after the pure suite.

The multi-host sample is tests/fixtures/har/multi-host.har. The [M2 host-summary contract](contracts/m2-host-summaries.md) defines grouping, count meanings, duration overflow, and navigation. Redirect suggestions are defined by the [redirect contract](contracts/m2-redirect-suggestions.md). Open tests/fixtures/har/redirects.har and scroll to REDIRECT EVIDENCE in request #0 for ambiguity, #3 for a single cross-host candidate, and #5 for a missing target. Candidates are source IDs; return to the list to inspect them. Core now uses the existing locked url crate for pure URL resolution, with no network operations. Conservative retry suggestions are now defined by the [retry contract](contracts/m2-retry-suggestions.md). Open tests/fixtures/har/retries.har and inspect #0 for ambiguity, #3 for a single candidate, #5 for an excluded method, #6 for unknown status, and #7 for no match. Suggestions are offline comparisons, not instructions to repeat a request.

The [protocol contract](contracts/m2-recorded-protocols.md) defines exact exporter-label recognition and evidence limits. Open tests/fixtures/har/protocols.har: #0 has equivalent aliases, #1 disagrees, and #2 has an unrecognized request label. Protocol notes describe general behaviour without reconstructing frames or connections.

## Replay workflow

The agreed [M3 replay contract](contracts/m3-single-request-replay.md) and its [request-body extension](contracts/m3-request-bodies.md) define an editable GET or POST draft, explicit header selection, review before sending, response limits, cancellation, and controlled local tests. The pure draft model and terminal draft/review editor are implemented. Press r from an eligible inspector; Review the destination, then Enter or s sends. The result stays separate from the capture. Esc during sending cancels and returns to the draft after cleanup; o reopens the retained result.


## Try a local send

In a separate terminal, create a small temporary response and start a server:

```sh
mkdir -p /tmp/netscope-local
printf 'hello from a local server\n' > /tmp/netscope-local/hello.txt
python3 -m http.server 8080 --bind 127.0.0.1 --directory /tmp/netscope-local
```

In the Netscope terminal, run:

```sh
cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har
```

Enter opens the inspector; r opens a draft. On Edit URL, press Enter, Ctrl+u, and type `http://127.0.0.1:8080/hello.txt`, then Enter saves. Leave headers unselected, move to Review request, and press Enter to review. Check the local destination before pressing Enter or s again to send. Expect status 200, a complete body, and the local greeting. Press c to compare the captured and replayed responses side by side. Up/Down scroll both columns, v reveals header values, and Esc returns to the result. The [comparison contract](contracts/m3-response-comparison.md) distinguishes stored-byte equality from incomplete or unavailable data. Esc from the result returns to the draft; Esc again returns to the unchanged captured request. Stop the local server with Ctrl+C in its terminal.

## Try a local POST

Start the supplied loopback echo server in a separate terminal. Stop any previous server using port 8080 first:

```sh
python3 scripts/echo_server.py
```

Run Netscope with the same single-request fixture shown above. Open the inspector and press r. Edit the URL to `http://127.0.0.1:8080/books` **before entering a body**: saving a changed destination clears the body and header choices.

Press m to switch from GET to POST. The initial format is plain text; press t to select JSON. Press b, type `{"title":"café"}`, then Enter to save. Select Review request and press Enter. Expect method POST, Content-Type application/json, and Content-Length 17. Check the destination and body, then press s to send. The response should report body_bytes 17 and echo the original text.

For a second example, return to the draft, press t to select plain text, then b and Ctrl+u to replace the body with `café`. Save and review: the body is five bytes. Ctrl+n inserts a newline while editing; Tab inserts a tab. Esc discards an unfinished edit. Stop the echo server with Ctrl+C when finished.

The editor limits bodies to 16,384 UTF-8 bytes. JSON must contain one complete value accepted by the parser; validation preserves the entered whitespace and number spelling. Content-Type and Content-Length are managed from the reviewed body. The [body contract](contracts/m3-request-bodies.md) records capture eligibility and exact limits.

## Network verification

`cargo test -p netscope-net --locked --offline` includes two kinds of checks: paused-clock/scripted-body tests for deadline and retention rules, and loopback socket tests for actual HTTP/TLS behaviour. Socket tests use ephemeral ports and explicit synchronization, with wall-clock watchdogs. Local certificates are generated for tests; production code still verifies bundled roots and the destination hostname. No captured external destination is used.

`python3 scripts/check_replay_terminal.py` exercises live local GET and JSON POST requests, cancellation while a response stalls, repeated Send keys, q, and Ctrl+C in pseudo-terminals. It checks socket disposal and restored terminal attributes. Run it alongside `python3 scripts/check_terminal.py` to cover the existing navigation and rendering-error paths. Fixed screen buffers test every terminal outcome at both supported minimum/review sizes. The CI workflow runs both process scripts after the workspace checks.

The worker emits one result through a capacity-one channel. Socket and TLS futures are disposed before delivery. An OS DNS lookup may outlive cancellation; runtime shutdown does not wait indefinitely for it, and it cannot update the UI. Body retention is capped at 2 MiB; HTTP parser buffers, TLS buffers, and stored headers are additional bounded memory, as described in the transport decision.

## Save and reopen a draft

From a valid replay draft, press w. Enter a name, then a file path such as `/tmp/book.json`. Relative paths use the working directory; `~` is not expanded. The parent directory must exist. Scroll through the save preview: it shows the exact JSON file and states that Enter replaces the chosen destination. Esc cancels. The preview excludes known authorization, cookie, and API-key headers; other headers, URLs, and bodies can still contain secrets. Enter confirms the local write without sending a request.

Close Netscope and reopen the file:

```sh
cargo run -p netscope-tui --locked --offline -- --draft /tmp/book.json
```

The saved name identifies the screen. The request opens for editing, with no captured response or comparison. Review and explicitly Send only when ready. Esc from the draft exits. The supplied `tests/fixtures/drafts/book.json` is a local POST example you can open with the same command and use with the echo server above.

Files contain versioned JSON, without response history or capture identity. Known credential headers cannot be persisted in this version; enter them explicitly after opening when needed. [The contract](contracts/m3-saved-drafts.md) lists the exact exclusions and limits. A failed write leaves an existing destination intact. A crash may leave `<filename>.netscope-tmp`; inspect that file before removing it or save to a different path. Unix saves use owner-only permissions. The writer does not promise survival across every power-loss or network-filesystem scenario.

## Correct text in the middle of a field

URL, body, header name/value, saved name, and path fields support Left/Right, Home/End, Backspace, and Delete. Home/End target the whole field, even when its display wraps. Type at the visible cursor; Ctrl+u clears. Ctrl+n and Tab insert newline/tab only in body editing. Enter saves and Esc discards the edit. Moving the cursor does not review, send, or write a file. The terminal cursor stays visible while editing and follows wrapping when resized.

The [keyboard contract](contracts/m4-keyboard-editing.md) defines Unicode boundaries and remaining limitations. Dedicated paste, selection, undo, and vertical/word-wise cursor movement are not implemented.

## Installation verification

Run `python3 scripts/check_install.py` after the workspace checks. It builds and installs the release binary into a temporary root, then verifies PATH lookup and command behaviour outside the checkout. It requires Python 3.11 or newer and cached locked dependencies; it does not change the user’s installed command. See [the installation guide](INSTALLATION.md).

## Browser HAR compatibility

The normal workspace suite imports frozen exports from Chrome and Firefox, checks their retained data and replay eligibility, and exercises CLI output and inspector labels. No browser installation or live server is required for these tests. Derived malformed-field cases are separate from the unchanged native recordings.

For a new manual recording, run `python3 scripts/har_fixture_server.py` and follow [the capture procedure](HAR_COMPATIBILITY.md). Regeneration is exploratory work: browser timings, ports, timestamps, and sometimes request order vary. Review a new recording before deliberately replacing any regression fixture.

The HTTPS extension uses `scripts/har_https_fixture_server.mjs` with disposable certificate files. See the [HTTPS reproduction procedure](HAR_COMPATIBILITY.md#reproduce-the-https-scenarios) for setup and certificate scope. Its Node/OpenSSL dependencies are only for recording new traffic; CI imports frozen HAR files offline.

## Large-capture checks

`python3 scripts/probe_large_capture.py` builds the release measurement example, creates temporary deterministic HAR workloads at 1,000 and 10,000 entries, and reports three import/render samples per workload. It requires no network after dependencies are cached. Elapsed times are observations, not pass/fail assertions. The [measurement reference](LARGE_CAPTURES.md) describes scope and limitations. Normal workspace tests cover the 10,000-entry boundary, relationship matching equivalence, and page/end navigation without wall-clock thresholds.

## Architecture decisions

Keep [the architecture record](ARCHITECTURE.md) alongside material changes. Record the problem, current choice, plausible alternatives, costs, evidence, and conditions for reconsideration. Label retrospective assessments separately from previously recorded decisions. Put the learner-facing explanation in the relevant chapter; a new chapter is justified by a new reader question, not each implementation patch. Open audit questions remain open until implementation and verification support a new status.
