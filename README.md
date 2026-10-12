# Netscope

A Rust terminal application for inspecting HTTP captures and learning how the program works.

Start with the [field notes](docs/README.md), an offline reading interface with beginner Rust explanations, chapter navigation, search, and an interactive ownership diagram. The [documentation README](docs/README.md) explains editing and regeneration.

The [M1 contract](docs/contracts/m1-har-inspection.md), [host-summary contract](docs/contracts/m2-host-summaries.md), [redirect contract](docs/contracts/m2-redirect-suggestions.md), [retry contract](docs/contracts/m2-retry-suggestions.md), and [protocol contract](docs/contracts/m2-recorded-protocols.md) define the implemented scope. Original planning documents and HTML mockups are local references excluded from Git. The four-crate Rust workspace, session model, HAR importer, and offline summary command are implemented. The interactive overview, inspector, and host summaries are implemented. The inspector includes conservative redirect and retry suggestions with explicit ambiguity; try tests/fixtures/har/redirects.har or tests/fixtures/har/retries.har. Protocol notes distinguish recorded version labels from general explanations. Press h for Hosts, Enter to filter, and a to return to all requests. Run cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har in a terminal.

Single-request replay is defined in the [single-request replay contract](docs/contracts/m3-single-request-replay.md). Press r in an eligible GET or POST inspector to edit and review a replay draft. Use m for method, t for plain text/JSON, and b for body editing. Enter or s from Review sends one HTTP/1.1 GET or POST over HTTP or verified HTTPS. POST bodies are limited to 16 KiB of UTF-8; JSON is checked without rewriting it. Responses have a 10-second total deadline and a 2 MiB body limit, with partial data retained. Esc cancels; o in the draft reopens the last result. Press c on a result to compare it beside the captured response, with partial data and unavailable comparisons labelled. Redirects, retries, proxy discovery, and decompression are disabled. See the [local testing instructions](docs/DEVELOPMENT.md#try-a-local-send).

## Install and start

From this checkout, run:

```sh
cargo install --path crates/netscope-tui --locked
netscope --help
netscope tests/fixtures/har/single-request.har
```

The installed command is `netscope`. Ensure Cargo's installation bin directory is on PATH. Use `netscope --version` to identify the installed package version, or `netscope --draft request.json` to reopen a saved draft. See the [installation guide](docs/INSTALLATION.md) for prerequisites, offline installation, command lookup, and removal. Opening files never sends network requests.

## Development

Run `python3 scripts/check.py` for formatting, compiler, Clippy, tests, build, and reader checks. The pinned Rust toolchain may require an initial download. Rust checks run with the lockfile and offline mode; run cargo fetch --locked once to download the locked dependencies before offline checks.

Read [the development guide](docs/DEVELOPMENT.md) for lint rules, fixtures, and test scope.

Save a reviewed, valid draft with **w**, choose a name and path, and confirm the file preview. Reopen it independently with `netscope --draft request.json`. Files preserve body text and selected non-credential headers; known authorization, cookie, and API-key headers are excluded. Opening never sends. See the [save/open walkthrough](docs/DEVELOPMENT.md#save-and-reopen-a-draft).

Editable fields support Left/Right, Home/End, Backspace, and Delete, with a visible Unicode-aware cursor. Enter saves the edit; Esc discards it.

Native Chrome and Firefox HAR exports are covered by frozen, offline regression fixtures. See [browser compatibility](docs/HAR_COMPATIBILITY.md) for tested versions, observed variations, and local reproduction.

Use Page Up/Page Down and Home/End in request lists, Hosts, and the inspector. See [large-capture checks](docs/LARGE_CAPTURES.md) for tested limits and local measurements.
