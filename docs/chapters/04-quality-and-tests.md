# Knowing whether the program is right

A request can appear on screen with the wrong timing. The program might compile, open the file, and draw every row while still turning a `-1` timing into zero. What would tell us that the behaviour is wrong?

We need to state what a small, controlled input should produce. That expectation gives a test something meaningful to check. Other tools answer different questions about the code, and understanding their limits helps us use them well.

## The compiler checks Rust's rules

The compiler rejects using a string after it has moved and passing a host ID where a request ID is required. These are rules expressed by the language and our types.

It cannot decide whether we have interpreted a HAR timing correctly unless we express that interpretation in the program. A missing timing converted into zero can be perfectly valid Rust and still be incorrect for our purpose.

This is why a successful build matters but does not finish the check. We also need evidence about the meaning of the result.

## A test makes an expectation precise

Take the optional timing `-1`. HAR 1.2 defines it as a stage that did not apply to the request, and we expect the importer to retain that meaning. A test supplies that value and compares the result with the `Unavailable` variant that stores it. If the importer changes it to zero, the comparison fails.

A useful test protects a distinction someone cares about. In this project, examples include preserving duplicate headers, returning to the same selected request, and distinguishing an empty body from an unrecorded body.

We use invented capture data for these checks. A fixture is a saved test input with a known purpose. Small fixtures make it possible to understand why a result is expected rather than trusting a large recording no one has read closely.

## Control the inputs that affect the answer

A deterministic test produces the same observable result from the same controlled inputs. Sorting three requests with fixed timestamps is a good example. Equal timestamps need an explicit tie rule, or a change in ordering could make the selected request appear to jump.

A test that depends on a public server has more inputs than its source file reveals. The server can change its response, fail, or reply at a different speed. Most of Netscope's current tests therefore use local synthetic data and screen buffers held in memory.

Time needs the same treatment when it becomes part of the behaviour. To test whether a cookie has expired, supply the time used by the check. Then test just before expiry, at expiry, and after expiry. Waiting for a real clock to cross the boundary makes the experiment harder to repeat.

## Check the rule and the visible result

Navigation tests apply actions to view state and check which request remains selected. Drawing tests inspect the resulting terminal cells at fixed sizes. The two checks support different claims: the intended request can be selected even if the screen fails to show that selection clearly.

Terminal restoration needs another kind of evidence. An in-memory screen cannot prove that the terminal's input mode was restored. The separate terminal check opens a controlled pseudo-terminal, runs the viewer, and inspects the modes and restoration commands after exit.

That integration check involves the operating system and uses a timeout. We keep it separate from the deterministic state and buffer tests, so a reader can tell exactly what each result establishes.

## Use linting for recurring code concerns

A lint is an automated check for a code pattern that deserves attention. Clippy is Rust's linting tool. It can catch common mistakes and suggest clearer forms; it cannot decide whether an explanation is suitable for a beginner.

The workspace, our group of related Rust packages, also uses a formatter for consistent code layout and documentation tests that compile the marked examples. It fixes the Rust toolchain version and retains a dependency lockfile, which records the dependency versions chosen for the application. Those choices reduce accidental differences between development environments.

Our lint rules forbid unsafe code and deny unhandled results that must be used. They also restrict `unwrap` and `expect`, which can panic on failure. A rejected capture should produce a useful error message, so that rule supports a visible requirement rather than merely imposing a style preference.

The exact rules and check commands belong in the [development guide](../DEVELOPMENT.md). A chapter should explain why the checks matter without requiring you to memorise their flags.

## Let comments explain a reason

A comment such as `Loop over requests` repeats what a loop already says. A comment such as `Keep source order so equal timestamps stay stable` explains a decision that may be less obvious.

Use familiar words in comments and keep the explanation near the relevant code. Longer lessons belong here, where a reader has room to follow the idea and an example. Passing a linter cannot establish that either explanation is clear.

## Design a small experiment

Choose one distinction: missing versus zero timing, an empty versus absent body, or source order versus display order. Describe the smallest input that exercises it and the exact result you expect.

Then describe a plausible bug that would make the test fail. If your test would still pass with that bug present, revise the expectation. The [verification record](../VERIFICATION.md) shows which experiments the current suite actually performs.
