# Knowing whether the program is right

A request can appear on screen with the wrong timing. The program might compile, open the file, and draw every row while still turning a `-1` timing into zero. What would tell us that the behaviour is wrong?

We need to state what a small, controlled input should produce. That expectation gives a test something meaningful to check. Other tools answer different questions about the code, and understanding their limits helps us use them well.

## The compiler checks Rust's rules

The compiler rejects using a string after it has moved and passing a host ID where a request ID is required. These are rules expressed by the language and our types.

It cannot decide whether we have interpreted a HAR timing correctly unless we express that interpretation in the program. A missing timing converted into zero can be perfectly valid Rust and still be incorrect for our purpose.

This is why a successful build matters but does not finish the check. We also need evidence about the meaning of the result.

## A test makes an expectation precise

Take the optional timing `-1`. HAR 1.2 defines it as a stage that did not apply to the request, and we expect the importer to retain that meaning. A test supplies that value and compares the result with the `NotApplicable` variant that stores it. If the importer changes it to zero, the comparison fails.

A useful test protects a distinction someone cares about. In this project, examples include preserving duplicate headers, returning to the same selected request, and distinguishing an empty body from an unrecorded body.

We use invented capture data for these checks. A [fixture](../GLOSSARY.md#fixture) is a saved test input with a known purpose. Small fixtures make it possible to understand why a result is expected rather than trusting a large recording no one has read closely.

## Control the inputs that affect the answer

A [deterministic test](../GLOSSARY.md#deterministic-test) produces the same observable result from the same controlled inputs. Sorting three requests with fixed timestamps is a good example. Equal timestamps need an explicit tie rule, or a change in ordering could make the selected request appear to jump.

A test that depends on a public server has more inputs than its source file reveals. The server can change its response, fail, or reply at a different speed. Most of Netscope's current tests therefore use local synthetic data and screen buffers held in memory.

Time needs the same treatment when it becomes part of the behaviour. To test whether a cookie has expired, supply the time used by the check. Then test just before expiry, at expiry, and after expiry. Waiting for a real clock to cross the boundary makes the experiment harder to repeat.

## Check the rule and the visible result

Navigation tests apply actions to view state and check which request remains selected. Drawing tests inspect the resulting terminal cells at fixed sizes. The two checks support different claims: the intended request can be selected even if the screen fails to show that selection clearly.

Terminal restoration needs another kind of evidence. An in-memory screen cannot prove that the terminal's input mode was restored. The separate terminal check opens a controlled pseudo-terminal, runs the viewer, and inspects the modes and restoration commands after exit.

That integration check involves the operating system and uses a timeout. We keep it separate from the deterministic state and buffer tests, so a reader can tell exactly what each result establishes.

## Use linting for recurring code concerns

A [lint](../GLOSSARY.md#lint) is an automated check for a code pattern that deserves attention. [Clippy](../GLOSSARY.md#clippy) is Rust's linting tool. It can catch common mistakes and suggest clearer forms; it cannot decide whether an explanation is suitable for a beginner.

The [workspace](../GLOSSARY.md#workspace), our group of related Rust packages, also uses a formatter for consistent code layout and [documentation tests](../GLOSSARY.md#documentation-test) that compile the marked examples. It fixes the Rust toolchain version and retains a dependency lockfile, which records the dependency versions chosen for the application. Those choices reduce accidental differences between development environments.

Our lint rules forbid unsafe code and deny unhandled results that must be used. They also restrict `unwrap` and `expect`, which can panic on failure. A rejected capture should produce a useful error message, so that rule supports a visible requirement rather than merely imposing a style preference.

The exact rules and check commands belong in the [development guide](../DEVELOPMENT.md). A chapter should explain why the checks matter without requiring you to memorise their flags.

## Let comments explain a reason

A comment such as `Loop over requests` repeats what a loop already says. A comment such as `Keep source order so equal timestamps stay stable` explains a decision that may be less obvious.

Use familiar words in comments and keep the explanation near the relevant code. Longer lessons belong here, where a reader has room to follow the idea and an example. Passing a linter cannot establish that either explanation is clear.

## Design a small experiment

Choose one distinction: missing versus zero timing, an empty versus absent body, or source order versus display order. Describe the smallest input that exercises it and the exact result you expect.

Then describe a plausible bug that would make the test fail. If your test would still pass with that bug present, revise the expectation. The [verification record](../VERIFICATION.md) shows which experiments the current suite actually performs.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Separate types for separate IDs

**Level:** Newcomer


A request ID and a host ID are both numbers. Why does Netscope wrap them in `RequestId` and `HostId` instead of using plain `usize` values?

<details>
<summary>Answer</summary>

`RequestId(usize)` and `HostId(usize)` are tuple structs, so the compiler treats them as different types. A function that expects a `RequestId` rejects a `HostId` at compile time, even when both hold the number 2. That catches a whole class of mix-ups before any test runs. The wrapper does not prove the request exists. A lookup must still check that.

**Learn more:** [Give different IDs different types](../chapters/08-structs-vectors-and-ids.md#give-different-ids-different-types) · [The compiler checks Rust's rules](../chapters/04-quality-and-tests.md#the-compiler-checks-rusts-rules) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6)

</details>

### What does each crate own?

**Level:** Practitioner


The workspace has four crates: `netscope-core`, `netscope-import`, `netscope-net` and `netscope-tui`. What does each one own, which way do the dependencies point, and why does the core crate contain no file access, no network access and no async runtime?

<details>
<summary>Answer</summary>

`netscope-core` holds the data model and the pure analysis: sessions, requests and timings, redirect and retry suggestions, SSE and WebSocket evidence, packet types, TCP reassembly, wire HTTP framing, DNS and STUN recognition, replay drafts and the saved-draft format. Its only dependencies are `url`, `serde` and `serde_json`. `netscope-import` reads HAR and classic PCAP files into core types. `netscope-net` performs the one reviewed HTTP/1.1 exchange with tokio, hyper and rustls. `netscope-tui` depends on the other three and owns drawing, key handling and the `netscope` binary.

The dependencies point toward the data rules: the terminal crate depends on import and net, and both of those depend on core. Keeping I/O out of core makes its analysis deterministic and easy to test with plain values. It also gives a structural guarantee: the crates that parse captures have no network dependency, so opening a capture cannot send traffic. The workspace lints also forbid `unsafe` code and deny `unwrap` and `expect`.

**Learn more:** [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules) · [Package boundaries](../ARCHITECTURE.md#package-boundaries) · [`[dependencies]`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/Cargo.toml#L18) · [`[workspace.lints.rust]`](https://github.com/sidsharma96/netscope/blob/main/Cargo.toml#L15-L23)

</details>

### The question-mark operator

**Level:** Newcomer


What does `?` do after a call that returns a `Result`, and why does Netscope not call `unwrap` there instead?

<details>
<summary>Answer</summary>

If the result is `Ok`, `?` takes out the value and execution continues. If it is `Err`, `?` returns that error from the current function at once. The importer uses this to stop at the first invalid required field. `unwrap` would panic on `Err` and end the program, which is wrong for an expected problem such as a bad file. The workspace lints deny `unwrap` and `expect` for this reason.

**Learn more:** [Stop when the next step cannot proceed](../chapters/09-options-results-and-matching.md#stop-when-the-next-step-cannot-proceed) · [Use linting for recurring code concerns](../chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns) · [`unsafe_code`](https://github.com/sidsharma96/netscope/blob/main/Cargo.toml#L16)

</details>

### Leaving the terminal usable

**Level:** Newcomer


Netscope switches the terminal into raw mode and an alternate screen. How does it restore the terminal if drawing fails partway through?

<details>
<summary>Answer</summary>

A guard value owns the cleanup. When setup changes the terminal, the guard records those changes. A normal exit restores them explicitly and reports any error. If the function returns early after an error, the guard's `Drop` implementation runs when it leaves scope and attempts the same restoration. A separate pseudo-terminal check runs the real program and inspects the terminal modes after it exits.

**Learn more:** [Leave the terminal usable](../chapters/11-drawing-and-navigation.md#leave-the-terminal-usable) · [Check the rule and the visible result](../chapters/04-quality-and-tests.md#check-the-rule-and-the-visible-result) · [`Restore`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L42)

</details>

### What a test proves

**Level:** Newcomer


The program compiles and draws every row. Why does Netscope still need a test that feeds in a `-1` timing, and why are most tests built on small invented captures?

<details>
<summary>Answer</summary>

The compiler checks Rust's rules, not whether the program means the right thing. Turning `-1` into zero is valid Rust and still wrong, and only a test that states the expected `NotApplicable` result catches it. Small invented fixtures make the expected result easy to explain. Local data also keeps tests deterministic: the same inputs give the same result, with no public server whose answer or speed can change.

**Learn more:** [A test makes an expectation precise](../chapters/04-quality-and-tests.md#a-test-makes-an-expectation-precise) · [Control the inputs that affect the answer](../chapters/04-quality-and-tests.md#control-the-inputs-that-affect-the-answer)

</details>
