# How this book should read

This book is for a reader who knows no Rust. Its job is to make a technical idea understandable, then help the reader recognise that idea in a working program. The reader's questions determine the sequence. Code supplies concrete examples and evidence once there is a reason to study it.

This applies to every existing chapter and every chapter added later. An implementation change is a reason to check the explanation's accuracy, not an instruction to organise the explanation around new functions or files.

## Start with something the reader can reason about

A useful opening gives the reader a situation: a missing measurement, a selected request, or text that needs to remain alive while another function reads it. Develop the question before naming the language feature that helps answer it.

For example, beginning a screen chapter with `App::apply matches Screen and Action` requires the reader to decode several program names. Beginning with a selected request that should remain selected after Esc establishes the behaviour first. An enum and a small state value then have a purpose the reader can explain.

The detail still matters. Once ownership appears, explain which value owns the data, what moves, what remains in place, and when cleanup happens. A short slogan about safety cannot replace that account.

## Be precise without making the prose dense

Use ordinary words where they express the idea accurately. Introduce a technical term when it lets us name something useful, and explain it at that first use. Continue using the same name so the reader does not have to decide whether a new synonym describes a new concept.

Give sentences enough room to express cause and consequence. Short sentences can establish a rule; a longer sentence can explain the condition under which it applies. Avoid both compressed notes and paragraphs that introduce several unfamiliar concepts at once.

Simplicity does not require removing the mechanism. "A shared reference lets this function read the request while its owner keeps the data alive" is concrete. It tells the reader what access is granted and where responsibility remains. Follow it with a small example and a consequence the reader can predict.

## Let examples earn their place

Use the book request as a running example when it helps connect ideas. Choose the smallest code fragment that demonstrates the current question. Explain new syntax close to the fragment, and show or describe the expected result.

Introduce another concept only when the explanation needs it. A vector example does not need to teach chained transformations of optional values at the same time. That extra syntax can wait until the reader has a reason to ask about it.

When an illustration differs from the application, say so near the example. Once the idea is clear, link to the actual source for readers who want to examine the complete version. Keep test counts, detailed build flags, and changing implementation inventories in reference material.

## Use diagrams to explain a relationship

The ownership drawing shows one text allocation as responsibility moves between names. A navigation drawing shows what changes when a request opens and what remains available when the reader returns. Each figure should answer a question that prose alone makes harder to follow.

Introduce the diagram before asking the reader to interpret it. Explain what its arrows mean and what the reader should notice. Label illustrative data so a precise-looking drawing does not imply a measurement or a guaranteed memory layout.

The visual direction uses charcoal, mint, thin rules, and numbered figures, with chapter navigation and a local outline. Prose uses proportional type; code and technical labels use monospace. The layout should help sustained reading as well as quick lookup.

## Review the explanation before adding detail

Read the chapter from its opening with no source file beside it. Can a newcomer explain the problem, follow the example, and predict one consequence? Check every unfamiliar term and symbol. If understanding requires an unexplained name from the repository, add the missing explanation or move the detail to a reference.

Then check the technical claims against the implementation. This second pass catches stale facts without turning the chapter into a code tour. A refactor that preserves behaviour may require a source-link correction and no change to the lesson's structure.

Exercises should follow naturally from the explanation. Ask the reader to predict a result, make a small change, or explain a choice. Provide enough setup to attempt the exercise without guessing how to run the example. Compilation checks can catch broken snippets; they cannot establish whether the prose teaches well.

## Explain why a design fits, and when it would not

A material architecture choice needs more than a description of the implementation. Explain the concrete constraint, a plausible alternative, and the consequence of choosing one. Distinguish a reason documented at the time from an assessment made during a later audit. Do not invent a rejected alternative or a performance result to make existing code sound inevitable.

Teach the reasoning in the chapter where the reader needs it. Keep dates, evidence links, and revisit conditions in the [architecture decision record](../ARCHITECTURE.md). Record unresolved choices honestly; an unnecessary atomic or an awkward ownership boundary is a reason to investigate, not an obligation to defend it. A code change should update the relevant decision and lesson when it changes the explanation, while preserving the reader's sequence.

## References and maintenance

Receipt's local prose guide, [pstack technical writing](https://github.com/cursor/plugins/blob/main/pstack/skills/technical-writing/SKILL.md), and [unslop](https://github.com/cursor/plugins/blob/main/pstack/skills/unslop/SKILL.md) informed the original style discussion. We adapt those references to this audience. Rules designed for a terse working reference should not remove explanations a beginner needs.

[Oxide](https://oxide.computer/) and [Modal's GPU glossary](https://modal.com/gpu-glossary/perf/memory-bound) informed the reading interface. These are design references, not affiliated brands.

Markdown chapters are the text source. The [development guide](../DEVELOPMENT.md) explains how to build and check them.

## Review each revision before continuing

After each coherent set of edits, read the revised pages in learning order without source code beside them. Explain the reader's starting question, the new idea, and one result the reader can now predict. Check that the examples and exercises require only concepts introduced by that point. Remove repeated explanations and implementation history that interrupt that sequence.

Then check factual claims against the implementation and run the documentation checks. Review changed Rust examples with the documentation tests as well. A passing command establishes only what it checks; report editorial findings separately. If the prose needs repair, revise it and repeat this review before starting the next part.
