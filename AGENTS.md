# Writing for Netscope

The living book assumes no Rust knowledge. Apply these rules to all existing and future chapters, diagrams, exercises, and explanatory comments.

Teaching chapters also follow `docs/BOOK-RULES.md`, the seven rules every learning book shares. Its canonical copy lives in `../learning-docs`; the copy in `docs/` is overwritten on sync. Where these rules and that file disagree about a teaching chapter, `docs/BOOK-RULES.md` wins.

- Let the reader's question determine the chapter's structure. Begin with a concrete situation and explain its behaviour before introducing the Rust mechanism or application symbols.
- Use code to support the explanation. Do not turn a source file, function sequence, recent change, or test report into the outline of a teaching chapter.
- Keep technical depth. Explain why a rule exists, what happens during execution, and what consequence the reader can predict. Simple language must not hide the mechanism.
- Explain a technical term and new syntax at first use. Introduce only the concepts needed for the current example. Prefer a small example with a visible result over a dense fragment from production code.
- Write connected prose with varied sentence lengths. Use familiar words, stable names, and concrete cause and effect. Avoid slogans, invented jargon, compressed notes, and repetitive chapter templates.
- Use a diagram when it clarifies a relationship, change, or sequence. Introduce it and explain how to read it. Distinguish illustrative examples from recorded facts and implementation guarantees.
- Put source links after the ideas they support. Keep detailed commands, current implementation inventories, test counts, and maintenance history in reference documents unless a lesson needs them. A mistake that git or a log records is told in the lesson that teaches its mechanism (rule 3 of `docs/BOOK-RULES.md`); `WHAT_WENT_WRONG.md` links to it.
- Keep comments brief and explanatory. State the reason, assumption, or consequence in ordinary language. Longer lessons belong in the book.

Before editing a chapter, read `docs/chapters/05-decisions.md`. Review the draft without referring to source code: can a newcomer understand the problem, follow the example, and predict a result? Then check its factual claims against the implementation. A refactor alone does not justify reorganising a lesson.

For a full learning audit, follow the reader's operations through storage, mutation, drawing, I/O and cleanup. Check the mechanisms and their prerequisites, not just whether chapter headings or terms exist. Require a concrete prediction after changing one input or condition, including effects on ownership, output, failure and cost where relevant. Mark the explanation partial when answering requires source code or an untaught concept. Keep unresolved findings and their acceptance tasks in `docs/questions/reader-audit.md`; passing links, compilation or tests does not close an editorial finding. State the scope of the review and distinguish it from observed beginner usability. Do not call a review exhaustive without evidence for that scope.

`docs/SUMMARY.md` defines the learning order and the book's parts; mdBook 0.5.4 renders it (`mdbook build docs`). Existing chapter filenames are stable identifiers, not an instruction to order chapters by their numeric prefixes. Preserve valid deep links when changing headings or navigation, and add every new page to `SUMMARY.md`.

Questions live only in `docs/questions/`. After editing them, run `python3 scripts/sync_questions.py` to regenerate each chapter's "Check your understanding" section; never edit that section by hand. Run `python3 scripts/check_docs.py` after documentation changes. When Rust examples change, also run `cargo test --workspace --doc --locked --offline`. These automated checks establish link and example correctness; they do not substitute for an editorial review.

For material architecture choices, maintain `docs/ARCHITECTURE.md` with constraints, alternatives, consequences, evidence, and revisit conditions. Distinguish contemporaneous records from later assessments. Update the relevant reader-led lesson when the reasoning changes; do not invent historical deliberation or justify complexity merely because it exists.

## Code excerpts in the book

Every teaching chapter follows Rule 7 of `docs/BOOK-RULES.md`: show the real code the chapter is about, give one concrete input before it, walk the lines that matter after it, show what they produce, and go one level down or up. A reader with only the book should understand 95% of that code. A link adds to code on the page; it never replaces it.

- Put `<!-- excerpt: crates/<crate>/src/<file>.rs#L<a>-L<b> @current -->` on the line above each excerpt and fence it as ```` ```rs ````. Never fence an excerpt as ```` ```rust ````: `scripts/check_book_examples.py` compiles every `rust` block, so those stay for complete teaching examples. Code links stay on `/blob/main/` with the symbol in backticks, because `scripts/check_docs.py` checks the symbol is near that line.
- Trim honestly. Replace each cut with a comment containing "…" that says what was cut. Every other line must appear, in order, in that range in the working source.
- Teach the current implementation. Keep old code only to explain a bug or performance improvement, with the problematic behaviour, a concrete input, and the current correction explained nearby or linked. Feature history, renames and refactors alone do not justify an old excerpt. Historical excerpts use a Git revision instead of `current`, with `<!-- historical: bug; why this old code is needed -->` or `<!-- historical: performance; why this old code is needed -->` immediately above the excerpt marker. Identify old code visibly in the prose; a hidden annotation is not an explanation. If a measurement's supporting code still matches the working source, use `@current` and identify the measured revisions separately.
- Show a walk-through once. When another chapter already walks the code in full, give one sentence and a link to its section. Pages that defend or map the design (`34-defending-the-design.md`) and the entry page link rather than repeat.
- Quote recorded measurements only, never timings rerun on a busy machine. Never edit a chapter's generated "Check your understanding" section; change the bank and run `scripts/sync_questions.py`.
- `scripts/check_docs.py` verifies both current and historical excerpts through `scripts/check_excerpts.py`, including the required bug or performance reason, so the normal private-book deployment checks them. Never convert historical examples to current merely to make a check pass. These checks verify code and labels; review the surrounding explanation separately.

## Documentation is private

`docs/` is ignored by Git in this repository and must never be committed or pushed: no `git add -f`, no docs workflow under `.github/`, and no code, test or CI step that reads `docs/`. The sources are backed up to the private `sidsharma96/learning-docs` repository, checked out beside this one at `../learning-docs`. Edit the docs here in `docs/`, never in `../learning-docs`, because the next deploy overwrites that copy.

To publish, run `../learning-docs/deploy.sh netscope`. It copies `docs/` into the backup repository and pushes it, runs this book's checks, builds the book and deploys it to the Cloudflare Pages project `netscope-docs`, served privately behind Cloudflare Access at `https://netscope-docs.sidsharma.dev`. Add `--no-deploy` to back up and check without publishing. The book's Rust examples are checked by `scripts/check_book_examples.py`, which `deploy.sh` runs; CI never sees `docs/`, so no crate may `include_str!` a chapter. Links to the book from tracked files, such as the README, use the site address rather than a `docs/` path.
