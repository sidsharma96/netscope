# Writing for Netscope

The living book assumes no Rust knowledge. Apply these rules to all existing and future chapters, diagrams, exercises, and explanatory comments.

- Let the reader's question determine the chapter's structure. Begin with a concrete situation and explain its behaviour before introducing the Rust mechanism or application symbols.
- Use code to support the explanation. Do not turn a source file, function sequence, recent change, or test report into the outline of a teaching chapter.
- Keep technical depth. Explain why a rule exists, what happens during execution, and what consequence the reader can predict. Simple language must not hide the mechanism.
- Explain a technical term and new syntax at first use. Introduce only the concepts needed for the current example. Prefer a small example with a visible result over a dense fragment from production code.
- Write connected prose with varied sentence lengths. Use familiar words, stable names, and concrete cause and effect. Avoid slogans, invented jargon, compressed notes, and repetitive chapter templates.
- Use a diagram when it clarifies a relationship, change, or sequence. Introduce it and explain how to read it. Distinguish illustrative examples from recorded facts and implementation guarantees.
- Put source links after the ideas they support. Keep detailed commands, current implementation inventories, test counts, and maintenance history in reference documents unless a lesson needs them.
- Keep comments brief and explanatory. State the reason, assumption, or consequence in ordinary language. Longer lessons belong in the book.

Before editing a chapter, read `docs/chapters/05-decisions.md`. Review the draft without referring to source code: can a newcomer understand the problem, follow the example, and predict a result? Then check its factual claims against the implementation. A refactor alone does not justify reorganising a lesson.

`docs/SUMMARY.md` defines the learning order and the book's parts; mdBook 0.5.4 renders it (`mdbook build docs`). Existing chapter filenames are stable identifiers, not an instruction to order chapters by their numeric prefixes. Preserve valid deep links when changing headings or navigation, and add every new page to `SUMMARY.md`.

Questions live only in `docs/questions/`. After editing them, run `python3 scripts/sync_questions.py` to regenerate each chapter's "Check your understanding" section; never edit that section by hand. Run `python3 scripts/check_docs.py` after documentation changes. When Rust examples change, also run `cargo test --workspace --doc --locked --offline`. These automated checks establish link and example correctness; they do not substitute for an editorial review.

For material architecture choices, maintain `docs/ARCHITECTURE.md` with constraints, alternatives, consequences, evidence, and revisit conditions. Distinguish contemporaneous records from later assessments. Update the relevant reader-led lesson when the reasoning changes; do not invent historical deliberation or justify complexity merely because it exists.

## Documentation is private

`docs/` is ignored by Git in this repository and must never be committed or pushed: no `git add -f`, no docs workflow under `.github/`, and no code, test or CI step that reads `docs/`. The sources are backed up to the private `sidsharma96/learning-docs` repository, checked out beside this one at `../learning-docs`. Edit the docs here in `docs/`, never in `../learning-docs`, because the next deploy overwrites that copy.

To publish, run `../learning-docs/deploy.sh netscope`. It copies `docs/` into the backup repository and pushes it, runs this book's checks, builds the book and deploys it to the Cloudflare Pages project `netscope-docs`, served privately behind Cloudflare Access at `https://netscope-docs.sidsharma.dev`. Add `--no-deploy` to back up and check without publishing. The book's Rust examples are checked by `scripts/check_book_examples.py`, which `deploy.sh` runs; CI never sees `docs/`, so no crate may `include_str!` a chapter. Links to the book from tracked files, such as the README, use the site address rather than a `docs/` path.
