# Netscope

A Rust terminal application for inspecting HTTP captures and learning how the program works.

Start with the [field notes](docs/README.md), an offline reading interface with beginner Rust explanations, chapter navigation, search, and an interactive ownership diagram. The [documentation README](docs/README.md) explains editing and regeneration.

The [M1 contract](docs/contracts/m1-har-inspection.md), [host-summary contract](docs/contracts/m2-host-summaries.md), and [redirect contract](docs/contracts/m2-redirect-suggestions.md) define the implemented scope. Original planning documents and HTML mockups are local references excluded from Git. The four-crate Rust workspace, session model, HAR importer, and offline summary command are implemented. The interactive overview, inspector, and host summaries are implemented. The inspector includes conservative redirect suggestions with explicit ambiguity; try tests/fixtures/har/redirects.har. Press h for Hosts, Enter to filter, and a to return to all requests. Run cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har in a terminal.

## Development

Run `python3 scripts/check.py` for formatting, compiler, Clippy, tests, build, and reader checks. The pinned Rust toolchain may require an initial download. Rust checks run with the lockfile and offline mode; run cargo fetch --locked once to download the locked dependencies before offline checks.

Read [the development guide](docs/DEVELOPMENT.md) for lint rules, fixtures, and test scope.
