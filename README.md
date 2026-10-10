# Netscope

A Rust terminal application for inspecting HTTP captures and learning how the program works.

Start with the [field notes](docs/README.md), an offline reading interface with beginner Rust explanations, chapter navigation, search, and an interactive ownership diagram. The [documentation README](docs/README.md) explains editing and regeneration.

The [M1 contract](docs/contracts/m1-har-inspection.md), [host-summary contract](docs/contracts/m2-host-summaries.md), [redirect contract](docs/contracts/m2-redirect-suggestions.md), [retry contract](docs/contracts/m2-retry-suggestions.md), and [protocol contract](docs/contracts/m2-recorded-protocols.md) define the implemented scope. Original planning documents and HTML mockups are local references excluded from Git. The four-crate Rust workspace, session model, HAR importer, and offline summary command are implemented. The interactive overview, inspector, and host summaries are implemented. The inspector includes conservative redirect and retry suggestions with explicit ambiguity; try tests/fixtures/har/redirects.har or tests/fixtures/har/retries.har. Protocol notes distinguish recorded version labels from general explanations. Press h for Hosts, Enter to filter, and a to return to all requests. Run cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har in a terminal.

Single-request replay is defined in the [single-request replay contract](docs/contracts/m3-single-request-replay.md). Press r in an eligible GET inspector to edit and review a replay draft. Enter or s from Review sends one HTTP/1.1 GET over HTTP or verified HTTPS. Responses have a 10-second total deadline and a 2 MiB body limit, with partial data retained. Esc cancels; o in the draft reopens the last result. Redirects, retries, proxy discovery, and decompression are disabled. See the [local testing instructions](docs/DEVELOPMENT.md#try-a-local-send).

## Development

Run `python3 scripts/check.py` for formatting, compiler, Clippy, tests, build, and reader checks. The pinned Rust toolchain may require an initial download. Rust checks run with the lockfile and offline mode; run cargo fetch --locked once to download the locked dependencies before offline checks.

Read [the development guide](docs/DEVELOPMENT.md) for lint rules, fixtures, and test scope.
