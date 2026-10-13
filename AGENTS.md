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

`docs/chapters.json` defines the learning order. Existing chapter URLs are stable identifiers, not an instruction to order chapters by their numeric filename prefixes. Preserve valid deep links when changing headings or navigation.

Edit chapter sources and regenerate `docs/index.html` with `python3 scripts/build_docs.py`. Run `python3 scripts/check_docs.py` after documentation changes. When Rust examples change, also run `cargo test --workspace --doc --locked --offline`. These automated checks establish link and example correctness; they do not substitute for an editorial review.

For material architecture choices, maintain `docs/ARCHITECTURE.md` with constraints, alternatives, consequences, evidence, and revisit conditions. Distinguish contemporaneous records from later assessments. Update the relevant reader-led lesson when the reasoning changes; do not invent historical deliberation or justify complexity merely because it exists.
