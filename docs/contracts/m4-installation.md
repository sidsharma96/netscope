# Installation and command-line entry

Install from the checkout with `cargo install --path crates/netscope-tui --locked`. The executable is netscope; publishing a registry package and distributing prebuilt binaries remain outside this slice. No repository runtime assets are required to open a user-supplied capture or saved draft. The book remains a separate resource in the checkout.

No arguments, --help, and -h print help to stdout and exit 0. --version and -V print `netscope <package version>` and exit 0; the version is embedded from Cargo metadata at compile time. Help/version must be standalone invocations. They do not read captures, open the terminal UI, or send network traffic.

Accept one capture path, --draft followed by one saved-request path, -- followed by a literal capture path, or --draft -- followed by a literal saved-request path. Bare paths starting with - are interpreted as options and rejected unless protected by the separator or written with a directory prefix such as ./ . Invalid combinations, missing option operands, extra arguments, and unknown options print a fixed error with a help hint to stderr and exit 2. Errors do not repeat arbitrary option contents. Paths remain operating-system strings; option recognition must not require converting filenames to UTF-8.

File/validation/terminal failures exit 1. Existing interactive and redirected-output behaviour remains unchanged. Opening never sends; no shell startup file, PATH, or global installation is modified by verification.

The installation check uses a temporary root, the default release build, the lockfile, and offline mode. It invokes the installed command through an isolated PATH setting from outside the checkout, with copied HAR/saved fixtures, literal dash-prefixed filenames, help/version, and documented error statuses. Existing terminal tests cover interactive behaviour; the install check covers packaging and command lookup. The script requires Python 3.11+ and bounded subprocess timeouts. CI is configured to run it on Linux; local verification is recorded separately.
