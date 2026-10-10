# Documentation verification

Verified October 9, 2026. These checks concern the reading interface and its sources. No Rust application tests were run.

- `python3 scripts/check_docs.py` passed. The generated reader matches Markdown sources, has five chapters, has unique anchors, and resolves its local links.
- `node --check docs/assets/book.js` passed.
- Browser checks verified ownership steps, shared-reference visibility, search filtering, both themes, chapter routing, and section scrolling.
- Visual review covered the default desktop viewport and a 390 × 844 mobile viewport. Neither produced whole-page horizontal overflow. Mobile chapter navigation opened, selected a chapter, and closed.

The interactive drawing is a teaching model, not a Rust execution engine. Code snippets are illustrative and have not been compiled as application code. External links were not exhaustively checked. Readability needs continued review with the learner; structure checks cannot establish understanding.

## Planning reconciliation on October 9, 2026

Regenerated the reader after adding the sixth chapter and updating the planning decisions. `python3 scripts/check_docs.py` passed with six chapters, matching generated content, unique anchors, and valid local links. Local Markdown links in the specification, coverage matrix, contract, and reading sources were checked separately. The earlier desktop and mobile visual checks remain historical evidence; no Rust runtime tests were performed.

## Rust foundation on October 9, 2026

Pinned Rust 1.92.0 and added four workspace crates. `python3 scripts/check.py` passed on the local macOS host: rustfmt, all-target compiler checks, Clippy with warnings treated as errors, three optional-timing regression tests, ten rustdoc checks including one expected compile failure, the workspace build, and generated-reader checks for nine chapters. Rust verification used the lockfile and offline mode after toolchain installation.

`cargo run -p netscope-tui --locked --offline` printed the foundation status and exited successfully. Five synthetic HAR fixtures parse as JSON; the intentionally invalid required-timing fixture is valid JSON but should later fail HAR validation. No importer currently consumes the fixtures, and the full M1 suite is not implemented.

The GitHub workflow is configured but has not run remotely. There is no Git repository or source commit to cite. HAR acceptance, rendering, cancellation, and network integration remain untested because those features do not exist. Beginner examples in chapters 7–9 are checked through rustdoc; older illustrative snippets are not all compiled. The reading layout was retained; historical visual checks are separate from this foundation verification.

## HAR import and summary on October 9, 2026

Implemented the owned session model in crates/netscope-core/src/session.rs, the M1 inspection-profile importer in crates/netscope-import/src/lib.rs, and the offline summary in crates/netscope-tui/src/main.rs. Source IDs stay in capture order; chronological and host display orders are separate. The importer rejects whole captures on consumed-field errors and retains recoverable body warnings. Registry dependencies are locked; CI now fetches the lockfile before offline checks.

`python3 scripts/check.py` passed on the local macOS host: formatting, all-target compiler checks, Clippy with warnings denied, 18 regression/process tests (3 timing, 12 importer/model, 3 CLI), 10 rustdoc checks including the expected compile failure, workspace build, and reproducible reader checks for 10 chapters. Checks used `--locked --offline` after the initial dependency fetch. Tests use synthetic data, fixed values, and child processes; no clock reads, external network, random inputs, or real-terminal state are involved.

Coverage includes source ordering and offset ties, duplicate headers, empty captures and BOM, extension methods, optional timing states, consumed-field types, source/field error order, timing mismatches and SSL containment, response representation states and decode budgets, form parameters, cookies, size absence, status zero, JSON duplicates/depth, input/entry limits, UTF-8-safe previews, control escaping, CLI exit codes, and contextual failures. The tests cover importer portions of contract cases 1–9; exhaustive combinations and fuzzing have not been performed.

Chapter 10 links the actual model, importer, and tests and includes an import-path diagram and prediction exercise. The generated book's links and reproducibility passed. No new browser visual review was performed; earlier reading-interface visual evidence remains historical. The learner's explanation has not yet been reviewed.

The interactive overview and inspector do not exist yet. Contract cases 10–12 (input state, fixed terminal buffers, Unicode width/layout, resize, and restoration) remain pending. The binary explicitly identifies the summary-only behaviour. GitHub CI has not run remotely. This directory still has no Git source revision, and no commit or push was performed.

## Interactive viewer on October 9, 2026

Implemented the chronological overview, request inspector, help, keyboard aliases, resizing, small-terminal fallback, and terminal restoration in crates/netscope-tui. View state is separate from the owned session. Inspector values are escaped before Unicode-aware wrapping. Request and response previews remain bounded. Redirected input or output uses the summary path without terminal setup.

`python3 scripts/check.py` passed on the local macOS host with formatting, all-target compiler checks, warnings-denied Clippy, 27 regression/process tests (3 timing, 12 importer/model, 3 CLI, 9 UI), 10 rustdoc checks, workspace build, and reproducible book checks for 11 chapters. The UI suite verifies source selection, scroll bounds, Enter/Esc restoration of overview selection and offset, help, empty input, resizing, 100 × 30 and 80 × 24 buffers, selected-row styling, all inspector sections, body labels, zero duration/status, wide/combining/joined Unicode, and control escaping. Three saved full-buffer fixtures were reviewed as text and compared byte-for-byte in tests.

`python3 scripts/check_terminal.py` passed separately for keyboard exit and an injected rendering error. It used POSIX pseudo-terminals at 100 × 30, checked restored termios modes, alternate-screen exit, and cursor visibility, and confirmed drawing was reached. On macOS the kernel may set PENDIN after restoring canonical input; the comparison excludes only that transient input-retype flag. This check uses a bounded wall-clock timeout and is distinct from deterministic in-memory tests. GitHub CI is configured to run it on Linux, but remote CI has not been executed.

Chapter 11 explains actual shared/mutable borrowing, state enums, closures, grapheme wrapping, test backends, and Drop cleanup, with a view-state diagram and a learner exercise. Earlier status text now reflects implementation. No new browser visual review or terminal-emulator screenshot review was performed; saved buffers are cell text rather than screenshots. Unicode display can vary with terminal fonts and versions. Forced termination, terminal disconnection, and every setup-error combination have not been tested.

M1's interactive build step is implemented. Learner explanation/review is still outstanding; passing tests alone does not complete the learning goal. There is still no Git source revision, and no commit or push was performed. Replay, networking, inferred relationships, and protocol-internal visualisations remain later milestones.

## Audience and editorial revision on October 9, 2026

Revised all eleven chapters for a reader with no Rust knowledge. The reading order now introduces values and functions before ownership, collections, and error handling. Each explanation starts with an observation or practical question, develops the mechanism, and uses implementation links as supporting references. Import and navigation diagrams now show the conceptual sequence instead of function names. Moved detailed maintenance guidance out of teaching prose, corrected stale status statements, and added an explicit match example with its syntax explained.

Root AGENTS.md, the writing chapter, and DEVELOPMENT.md now record the audience requirement for future chapters and revisions. The editorial review checks understanding before comparing claims with source code. Chapter URLs remain stable, and previous section anchors are retained as aliases. Display numbering follows the learning order rather than filename prefixes.

`python3 scripts/check_docs.py` passed for eleven chapters, matching generated content, unique anchors, and local links. `cargo test --workspace --doc --locked --offline` passed eleven documentation checks, including one expected compiler rejection. `node --check docs/assets/book.js` passed. The build validates section-alias targets. Application behaviour was unchanged, so the runtime and terminal suites were not repeated for this editorial pass.

A browser visual review was attempted but blocked by the browser tool's local-file URL policy; no workaround was attempted. This revision has structural and example checks plus an editorial read-through, not new rendered-browser verification. Those checks cannot establish that a learner finds the explanation clear; reader feedback remains the final evidence for that question.

## Host summaries and filtered requests on October 9, 2026

Implemented the first M2 step under contracts/m2-host-summaries.md: normalised URL-host groups, derived status and warning counts, source-order duration sums with explicit overflow, chronological host request IDs, and the Hosts screen. Host filtering preserves source identity and uses the whole capture's timing scale. Returning to all requests restores the saved selection and scroll position. Redirect/retry analysis and additional protocol explanations remain pending M2 work.

`python3 scripts/check.py` passed on the local macOS host with formatting, all-target compiler checks, warnings-denied Clippy, 35 regression/process tests (3 timing, 12 importer, 4 host analysis, 3 CLI, 13 view tests), 13 rustdoc checks including one expected compile failure, the workspace build, and reproducible reader checks for 12 chapters. Added tests distinguish alphabetical host order from first-source IDs, merge case/scheme/port variants by parsed host, preserve separate subdomains, cover every status class, retain status zero, count requests with multiple warnings once, and check zero, overflow, and source-order floating-point addition.

Host navigation tests exercise selection, filter changes, inspector return, all-request restoration, scrolling, help, resizing, empty input, q, control escaping, and long names. Reviewed full text-cell fixtures for Hosts and a filtered list at 80 × 24 and 100 × 30; the tests compare them exactly and check selection styling. The existing overview/inspector fixtures were updated for the visible host shortcuts.

`python3 scripts/check_terminal.py` passed after extending its key sequence to enter Hosts, filter, inspect, return, and quit. It confirms that the host screen was drawn and verifies restored terminal modes, alternate-screen exit, and cursor visibility. The injected-rendering-error check also passed. This uses a bounded real-time timeout and remains separate from deterministic tests. No new terminal-emulator screenshot or browser visual review was performed. Hosted CI has not run; no commit or push was performed.

The new grouping chapter begins with the meaning of a host group and a count, introduces a small for-loop example, and uses an explicitly illustrative overlap diagram before discussing duration sums. Both code examples are compiled through rustdoc. Source links and detailed policies follow the explanation. Learning review remains outstanding.

## Conservative redirect suggestions on October 9, 2026

Implemented capture-wide redirect evidence derived once per session, using the existing locked url dependency in the core crate for pure parsing. The [redirect contract](contracts/m2-redirect-suggestions.md) records supported statuses, single-Location evidence, URL and method comparisons, the explicit timing window, ambiguity, and candidate limits. The inspector shows possible follow-ups by stable source ID; no request is automatically chosen or sent. HAR redirectURL metadata remains ignored. Candidates are inspected by returning to the list; a direct jump action is not implemented.

`python3 scripts/check.py` passed locally after the final code changes: formatting, all-target compilation, warnings-denied Clippy, 43 regression/process tests (3 timing, 12 importer/model, 4 host, 7 redirect, 3 CLI, 14 UI), 14 rustdoc checks including the expected compile failure, workspace build, and reproducible reader checks for 13 chapters. All Rust checks used the lockfile and offline mode. New analysis cases cover relative URL resolution, normalization boundaries, methods, missing and repeated headers, ambiguous timestamp ties, timing-window edges, zero duration, bounded candidate storage, cross-host matches, and ignored exporter metadata. UI tests reach the evidence by scrolling and compare complete saved buffers at 80 × 24 and 100 × 30. Those two new buffers were reviewed as text.

`python3 scripts/check_terminal.py` also passed for keyboard exit and an injected rendering error, including restored terminal attributes, alternate screen, and cursor. This separate pseudo-terminal check uses bounded wall-clock waiting; the matching and in-memory view tests use fixed inputs.

The new lesson starts with two requests that could both explain a redirect, then introduces an enum that distinguishes an unavailable search from an empty completed search. Its Rust example compiles through rustdoc. A labelled illustrative diagram separates captured fields from suggested relationships. Editorial review checked the explanation independently of the implementation before checking the stated behaviour against code. The grouping chapter now leads into this question, and the opening chapter reflects the new capability.

No new browser visual review, terminal-emulator screenshot review, hosted CI run, performance benchmark, commit, or push was performed. The timing policy can miss real follow-ups, and a single candidate never proves causation. Learner review remains outstanding. Retry suggestions and recorded protocol explanations remain later M2 work.

## GitHub preparation on October 9, 2026

Prepared the initial Git commit with the original four root planning/mockup files excluded by .gitignore. They remain available locally. The living book and milestone contracts are included. Chapter Markdown now uses source-relative code and reference links suitable for GitHub; the generator adjusts those paths for the offline HTML reader and maps chapter links to its existing routes. The documentation README supplies the chapter reading order. Links to the excluded specification and separate Receipt checkout were replaced with plain reference descriptions.

The full local check script passed (43 regression/process tests and 14 rustdoc checks), as did terminal restoration checks. Documentation checks passed again in a temporary export containing only staged files, proving those checks do not depend on excluded references. Source-link validation now checks that Markdown chapter links stay inside the repository and resolve. Fixed-width terminal fixtures intentionally retain trailing spaces; .gitattributes records that exception. No hosted website is required or configured. Earlier no-Git statements describe the historical state when those checks ran.

## Conservative retry suggestions on October 9, 2026

Added offline retry evidence under the [retry contract](contracts/m2-retry-suggestions.md). Selected recorded error statuses initiate a bounded search for matching GET/HEAD requests with exporter-reported zero body size, no captured request body representation, and equal captured headers, query fields, and cookies. Source status zero remains unavailable evidence rather than a diagnosed failure. Candidates remain possible explanations; response success, client intent, attempt numbers, and Retry-After compliance are not inferred. URL and timestamp comparison helpers are shared with redirect analysis without changing its policy.

The final `python3 scripts/check.py` run passed locally: formatting, all-target compilation, warnings-denied Clippy, 50 regression/process tests (3 timing, 12 importer/model, 4 host, 7 redirect, 6 retry, 3 CLI, 15 UI), 15 rustdoc checks including one expected compile failure, build, and generated-book/source-link checks for 14 chapters. Rust checks used locked dependencies and offline mode. The new tests cover status/method scope, both sides of body eligibility, metadata order and value changes, URL distinctions, overlap, equal starts, time-window boundaries, invalid/zero durations, bounded ambiguity, missing status, candidate errors, and uninterpreted Retry-After. Complete retry buffers at 80 × 24 and 100 × 30 were reviewed as text and compared in tests. Existing redirect fixtures and tests still pass.

The separate pseudo-terminal check passed for keyboard exit and injected rendering error, restoring terminal attributes, alternate screen, and cursor. The new chapter begins with competing explanations for repeated requests before teaching tuples, equality, Boolean values, short-circuit evaluation, and borrowing during comparison. Its example compiles. Its text diagram appears in Markdown on GitHub and in the offline reader. Editorial review checked the lesson's sequence and terminology before reconciling it with the contract and implementation.

This work is local and has not been committed or pushed. No new browser visual review or hosted CI run was performed for this change. Exact metadata matching and the time window can exclude actual retries, and exporter omissions can make independent requests appear alike. Learner review and recorded protocol explanations remain outstanding.

## Interactive retry walkthrough on October 9, 2026

Ran the actual netscope binary with retries.har in an 80 × 24 pseudo-terminal and drove its keyboard input directly. Opened #0 and scrolled to its two ambiguous candidates, returned with Esc and confirmed #0 remained selected, then inspected #3 (one candidate, #4), #5 (POST excluded), #6 (missing status kept unknown), and #7 (no matching follow-up). Read the terminal output from those interactions. Quitting with q exited with code zero and emitted cursor-show and alternate-screen-exit sequences. This was an interactive process walkthrough, not another TestBackend run or a terminal-emulator screenshot review. No code changes were needed.

## Recorded protocol explanations on October 9, 2026

The preceding retry implementation and interactive walkthrough were committed and pushed to origin/main as 0a3891a, “Add conservative retry evidence and explain repeated requests.” Earlier statements that retry work was local describe its state before that commit.

Added protocol-label classification and inspector notes under the [protocol contract](contracts/m2-recorded-protocols.md). Request and response labels remain separate and retain their original spelling. Explicit aliases share a classification; recognized disagreements and unknown labels receive different explanations. General protocol capabilities are labelled separately from capture evidence. No frames, stream IDs, compression savings, connection reuse, protocol transition, or comparative speed are inferred.

The final local check script passed formatting, all-target compilation, warnings-denied Clippy, 53 regression/process tests (4 core, 12 importer/model, 4 host, 7 redirect, 6 retry, 3 CLI, 17 UI), 16 rustdoc checks including the expected compile failure, build, and reader/source-link checks for 15 chapters. Rust checks used locked offline dependencies. An initial run identified missing module documentation on the new integration test; this was corrected before the passing final run. New cases cover all recognized spellings, exact-match exclusions, alias agreement, disagreement, unknown values, original-label retention through import, and control escaping. Complete protocol buffers at 80 × 24 and 100 × 30 were reviewed as text. Relationship buffers were regenerated and reviewed: candidate content is unchanged, with new protocol context and scroll offsets.

Also ran the actual binary against protocols.har in an 80 × 24 pseudo-terminal. Opened and scrolled through #0 (equivalent aliases), #1 (different recognized versions), and #2 (unknown request label), returning through the list between entries. q exited with code zero and cursor/alternate-screen restoration sequences. The separate terminal-restoration script passed both normal keyboard exit and injected rendering failure.

The beginner chapter begins with overlapping requests and the limits of a version label, then explains message representation, transport, interleaving, and the distinction between capability and observation. A labelled diagram is visible in GitHub Markdown and the offline reader. The Rust example explains borrowed string slices and exact alternative patterns and compiles through rustdoc. Editorial review checked the explanation independently of application symbols, then reconciled claims with implementation and protocol references.

The protocol work is local and not yet committed or pushed. No new browser visual review or hosted CI verification was performed for it. Learner review remains outstanding; implementing these M2 features does not establish that the lessons are understood. Replay still requires a separate M3 contract.
