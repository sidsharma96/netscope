# Install and run Netscope

Install [Rust through rustup](https://rustup.rs/) if Cargo is not already available. Netscope needs Rust 1.92 or newer to build. Then run:

```sh
cargo install netscope --locked
netscope --help
```

Cargo downloads the source and builds the executable on your machine. The first installation needs internet access and the build tools for your operating system. macOS and Linux are tested; interactive behavior on Windows has not been verified.

Cargo usually puts the command in `$HOME/.cargo/bin`. If your shell reports “command not found,” open a new terminal after installing Rust. On macOS or Linux, `command -v netscope` shows which executable the shell finds. The [running-a-program lesson](chapters/22-running-a-program.md) explains how command lookup works.

## First run

Export a HAR recording from your browser's developer tools, or [download the invented sample](https://raw.githubusercontent.com/sidsharma96/netscope/main/tests/fixtures/har/single-request.har) and save it as `single-request.har`. From the folder containing that file, run:

```sh
netscope single-request.har
```

The sample shows one request for a list of books. Press Enter to inspect it, `?` for help, and `q` to quit. Opening a capture reads the file without sending a request. The [first-capture walkthrough](chapters/06-first-milestone.md) explains what to look for.

The interactive view needs at least 80 columns and 24 rows. Enlarge the terminal if Netscope asks for more space. When output is redirected to a file or pipe, it prints a text summary instead.

A relative path begins in the folder where you run the command. You can also give a full path, such as `netscope /path/to/capture.har`. The file does not need to live beside the installed executable.

## Saved requests

Open a saved draft with `netscope --draft request.json`. It opens for editing; you must review it and choose Send to make a network request. The [local walkthrough](DEVELOPMENT.md#try-a-local-send) supplies a test server for trying this workflow.

## Packet inspection

For a classic PCAP recording, run:

```sh
netscope --pcap capture.pcap
```

Netscope reads an existing file, so it needs no packet-capture privileges. PCAPNG is a different format and is not supported. In Wireshark, save a compatible recording using its pcap export option. See [packet inspection](DEVELOPMENT.md#inspect-packets-and-streams) for supplied samples and controls.

## Install from a checkout

From the repository root, run:

```sh
cargo install --path crates/netscope-tui --locked
```

The directory is named `netscope-tui`; the package and installed command are named `netscope`. Reinstall after changing source code to update the installed command. Add `--offline` only after the required dependencies have been downloaded. To install a particular published version instead, use `cargo install netscope --version 0.2.1 --locked`.

## Command-line details

`netscope --version` prints the installed version. A filename beginning with a dash needs a separator: `netscope -- -capture.har`. Exit status 0 means success, 1 means a file or runtime failure, and 2 means invalid arguments.

To remove the application, run `cargo uninstall netscope`. If you installed with a custom `--root`, use that same root when removing it.

## Verify an isolated installation

Contributors can run `python3 scripts/check_install.py` from a checkout to test a temporary installation. See [installation checks](DEVELOPMENT.md#installation-checks) for what it exercises.

## Registry publication

Maintainers should follow [publishing a release](PUBLISHING.md).

<a id="packet-inspection-on-unreleased-main"></a>
