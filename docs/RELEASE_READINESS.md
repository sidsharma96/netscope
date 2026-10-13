# Release readiness: 0.1.0

Reviewed October 13, 2026. This release is a tagged source checkout containing the Cargo binary and offline book. It does not publish a crates.io package or attach prebuilt executables.

## Local evidence

The observed environment was macOS on arm64, Rust 1.92.0, and Python 3.14.2. `scripts/check.py` verifies formatting, all-target compilation, warnings-denied Clippy, the workspace tests and Rust examples, build, and reproducible documentation with valid local links.

`scripts/check_install.py` builds with `cargo install --locked --offline` into a temporary installation root. It invokes the executable through a temporary PATH from a separate working directory. Help/version, relative and dash-prefixed filenames, saved-draft summary, invalid arguments, missing files, malformed JSON, invalid HAR values, and an empty capture have checked outcomes and exit codes. The normal Cargo installation is untouched.

On POSIX, the installation check supplies that binary and external working directory to the terminal scripts. Navigation covers paging, host filtering, URL search, replacing and clearing a query, help, exit, and recovery from a 40×10 window to 100×30. Deterministic view tests separately assert empty search results and selection rules. The injected rendering failure uses the dedicated development probe, not the installed executable.

Loopback replay tests search a captured URL, inspect it, prepare a draft, review, and send. They verify GET, exact JSON POST bytes, response comparison, cancellation, q/Ctrl+C, connection disposal, saving a draft, and independent reopening. Saved-draft sends start from their separate file mode. Watchdog deadlines detect hung processes; they do not establish latency guarantees.

## Documentation review

Reviewed the README, installation and development instructions, chapter order, introductory walkthrough, navigation/search explanation, replay/save lessons, and architecture decisions against shipped behaviour. The README and documentation landing page now give the current scope rather than an early implementation inventory. CLI help lists search and host controls. The book retains the reader-led sequence and existing URLs; detailed check results remain in reference documents.

## Release boundaries

This pass establishes the observed macOS checks and the separately reported CI result, not universal operating-system compatibility. Windows interactive behaviour has not been verified. HTTPS validation has controlled network regression coverage; this pass's live sends use loopback HTTP. Native HAR fixtures cover the documented browser recordings. No fresh-browser recording, external-service replay, search-latency benchmark, exhaustive race analysis, or learner-comprehension study was performed.

Version 0.1.0 keeps the limits and exclusions described in [release notes](../CHANGELOG.md). The [verification ledger](VERIFICATION.md) records final outcomes. The release tag identifies a reviewed snapshot; it does not imply API stability or registry publication.
