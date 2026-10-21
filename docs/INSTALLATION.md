# Install and run Netscope

Version 0.2.0 installs from a local checkout. Use `git checkout v0.2.0` in a clean checkout to select this release before installing. It is not published on crates.io. The checked development toolchain is Rust 1.92.0, selected by rust-toolchain.toml. Install Rust through [rustup](https://rustup.rs/) first; the initial toolchain and dependency downloads need internet access. Build tools for your platform may also be required.

From the repository root:

```sh
cargo install --path crates/netscope-tui --locked
netscope --version
netscope --help
```

The package is named netscope-tui; the installed command is netscope. Cargo uses a release build for installation. Its usual destination is `$HOME/.cargo/bin`, unless your Cargo configuration or environment selects another installation root. Ensure that directory is on PATH. On a standard macOS/Linux rustup setup, opening a new terminal usually activates the configured environment. Use `command -v netscope` to see which executable your shell finds. You can also run the installed executable by its full path. See [Cargo's installation reference](https://doc.rust-lang.org/cargo/commands/cargo-install.html) for destination options.

After dependencies have been downloaded, add `--offline` to the install command to prohibit Cargo network access. `--locked` keeps dependency selection tied to Cargo.lock. Reinstall from the updated checkout to replace the installed version; editing source files alone does not update it. This project is not publishing a registry package or downloading prebuilt release binaries in this step.

## First run

From the checkout, try:

```sh
netscope tests/fixtures/har/single-request.har
netscope --draft tests/fixtures/drafts/book.json
```

Elsewhere, provide an absolute path to the file or copy it into the current directory. Capture paths and saved-draft paths are resolved from the directory where you invoke Netscope, not from the source checkout or installed binary's directory.

The interactive view needs a terminal at least 80 columns by 24 rows. Enter opens the selected capture request; / searches its full URL; h opens host summaries; a clears host and URL filters. Use r to prepare a replay draft, ? for navigation help, and q to quit. A saved draft opens directly for editing. Opening either kind of file never sends a request. Use the [local server walkthrough](DEVELOPMENT.md#try-a-local-post) before testing Send.

When input or output is redirected, Netscope prints a capture summary or the saved request's name and exits. `--help`/`-h` and `--version`/`-V` always print text without entering the terminal UI. No arguments show help. Exit status 0 means success, 1 means a file/validation/terminal failure, and 2 means invalid arguments.

A filename starting with a dash needs an explicit separator:

```sh
netscope -- -capture.har
netscope --draft -- -request.json
```

## Verify an isolated installation

```sh
python3 scripts/check_install.py
```

This needs Python 3.11 or newer. It installs a release binary into a temporary root with `--locked --offline`, runs it from a separate directory through a temporary PATH setting, and checks help, version, both file modes, malformed and empty captures, and failure exit codes. On POSIX systems it also runs navigation/search, small-window recovery, local GET/POST replay, cancellation, and save/reopen against the installed release binary. An injected drawing-error check uses a separate development probe. It cleans up that installation and does not alter your normal Cargo installation. Cargo's build cache remains reusable. The GitHub workflow runs the same check on Linux; consult the verification record for actual observed results.

To remove a normal Cargo installation, use `cargo uninstall netscope-tui`. If you selected a custom installation root, supply that same `--root` to uninstall.

<a id="packet-inspection-on-unreleased-main"></a>

## Packet inspection

Version 0.2.0 includes M5–M8. To run the current packet viewer without replacing an older installed binary:

```sh
cargo run -p netscope-tui --locked --offline -- --pcap tests/fixtures/pcap/streams.pcap
```

Use the normal source-install command from this checkout to install this release. --pcap selects the bounded classic-PCAP subset; it does not require capture privileges or libpcap/tcpdump to run. See [milestone status](MILESTONES.md) and the [packet walkthrough](DEVELOPMENT.md#inspect-packets-and-streams).
