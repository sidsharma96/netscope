# Publishing a release

These instructions are for maintainers publishing Netscope. To install the application, use the [installation guide](INSTALLATION.md).


Version 0.2.1 packages the application as `netscope`. Registry installation compiles the published source on your machine; it does not download a prebuilt executable. The directory remains `crates/netscope-tui`; its Cargo package name is now `netscope`, so development commands use `cargo run -p netscope`.

Run `python3 scripts/check_package.py` before publishing. It creates and builds all four archives using Cargo's temporary registry, then checks their normalized dependency versions, both license files and embedded lesson text. Repository tests and examples use shared fixtures and remain in the checkout rather than the published archives. The current workspace test suite still checks the book's Rust examples.

Sign in to crates.io, verify your email and create a publishing token. Run `cargo login` locally and enter the token only at its prompt. After the workspace and packaging checks pass, `cargo publish --workspace --locked` publishes the dependencies before the application. Recheck name ownership and availability before the first upload. Confirm a clean `cargo install netscope --version 0.2.1 --locked` from the registry afterwards, then tag the published commit.
