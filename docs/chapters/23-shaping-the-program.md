# Where should a change belong?

Suppose you want to add a second capture format. The new file describes the same kinds of requests, but arranges its fields differently. Should that force you to rewrite the terminal screen, the host summaries, and the request editor?

The answer depends on where the program draws its boundaries. Architecture is the set of choices about responsibilities, dependencies, and ownership that make some changes local and others expensive. It is easier to judge those choices through a concrete change than through a list of pattern names.

By this point in the book, we have followed a recording into a [session](../GLOSSARY.md#session), opened a request, sent a separate draft, and saved it. We can now ask why those operations are arranged as they are, and when another arrangement would serve us better.

## Keep a file format away from a screen's decisions

The importer understands HAR fields and validates the parts Netscope consumes. It produces a session containing requests in the program's own representation. Drawing reads that representation, so it need not know where a browser placed a particular JSON field.

We could instead let each screen read JSON directly. That might shorten the first prototype, but every screen would need to handle missing fields, wrong types, and browser variations. Two screens could end up interpreting the same record differently. A common representation gives those checks one place to happen.

There is a cost. The representation must change when we decide to retain something new, and converting the parsed file into owned request values requires allocations. An allocation is memory the program requests for data such as a string or vector. During import, parsed JSON and the growing session can both occupy memory. A byte limit on the file is therefore not a promise that total memory stays below that same number.

A different format would need to produce the same meanings, not merely fill fields with vaguely similar numbers. If it cannot distinguish an unavailable duration from zero, the importer must preserve that uncertainty. The shared model is useful only while its distinctions remain honest.

## Make dependencies point toward the data rules

A Rust [crate](../GLOSSARY.md#crate) is a unit the compiler builds. A library crate exposes code for other crates to use; a binary crate produces a runnable program. Netscope groups its work into four packages in one Cargo [workspace](../GLOSSARY.md#workspace). The packages make their dependencies explicit.

In this diagram, an arrow means “uses code from,” not “sends a network message to.” The terminal package coordinates the other pieces; the common data rules do not depend on the terminal or network packages.

```text
netscope-tui ───────────────► netscope-core
     │                            ▲
     ├──► netscope-import ────────┤
     │                            │
     └──► netscope-net ───────────┘
```

The core package holds the session, draft rules, comparisons used by offline analysis, and saved-value representation. Import knows how to read HAR. Net handles a reviewed send and its result. Tui contains terminal drawing, input, the command entry point, and local saved-file operations. These are responsibility boundaries, not four separate running processes.

A single crate with modules could express much of the same organization with fewer manifests and less public API. Four packages are not inherently more mature or faster. Here they make it harder for an ordinary data rule to accidentally depend on terminal input or a live socket, and allow focused tests to use that rule without starting either. Core still uses libraries for URLs and serialization; independence from I/O does not mean independence from all libraries.

The boundaries are also imperfect. File operations currently live beside terminal coordination. If a second interface needs them, that placement may deserve revision. Splitting every small helper into its own package would create more boundaries to maintain without necessarily improving the user's experience.

## Create a value where its lifetime is understood

Imagine opening two recordings in separate views. Each view needs to retain its own requests. Explicitly constructing two sessions makes that relationship straightforward: each caller receives and owns its session.

A constructor is a function that creates a value. In Rust, names such as `Session::new` are ordinary associated functions: functions named through a type. Rust does not give `new` a special construction privilege. A factory is a broader creation role, often used when the caller should not need to choose the concrete implementation. A singleton is a policy that restricts something to one instance in a chosen scope, often accessed globally. Creation and instance count are different questions, so “factory versus singleton” is not automatically a meaningful choice.

A global session could avoid passing references through functions, but then the current recording would be a hidden dependency. Tests would need to coordinate resets, and a second open capture would need a new arrangement. Explicit owners and parameters keep that dependency visible. They cost some argument passing, which is a reasonable exchange for independent sessions.

Today Netscope has a concrete transport and straightforward construction functions. It does not have an abstract factory selecting among interchangeable HTTP clients. Tests substitute a smaller boundary where needed, such as an in-memory stream for a network exchange. That substitution lets a test control bytes without requiring a second full HTTP implementation.

An interface that selected a real transport or a simulator might become useful if both existed and callers genuinely needed to choose. Adding that layer now would mean maintaining another contract for a hypothetical second implementation. Conversely, a process-wide immutable resource might reasonably be initialized once and shared. The current TLS configuration is constructed for each send and shared through `Arc`; it is not a global client singleton. That costs repeated setup, so reuse is a possible future measurement-driven change.

## Represent the alternatives the reader can distinguish

Suppose a request body is absent from the recording. Another body is present but empty. Storing both as an empty string would make later code unable to explain the difference. The model uses named alternatives because the user needs those outcomes to remain distinct.

Enums help express those alternatives. Structs group values that belong together. A source request ID stays separate from a row's position because filtering changes positions without changing captured identity. These choices allow the compiler to reject some mistakes and make others easier to test.

They do not make every application rule automatic. A request ID from another session can still have an in-range number. The current constructor expects already validated request values; its name alone does not prove they came through the importer. The editor's screen mode is an enum, but other fields still have to agree with that mode. We use tests and controlled construction paths for rules the types do not fully enforce.

We could build more restrictive types that make invalid construction harder. That would add conversions and more types to learn. We should do so when a concrete misuse or growing API makes the additional restriction valuable, rather than claim that the present model makes all invalid states impossible.

## Spend memory to avoid repeating a question

Opening Hosts asks for counts and an ordered list of requests for each host. Recomputing them on every keypress would repeatedly examine the same capture. Netscope instead prepares those summaries when it builds the session.

The advantage follows from a condition: the capture does not change while you browse it. The summaries can remain valid. We spend additional memory on counts and lists of IDs, but those lists do not duplicate every request body. Redirect and retry analysis likewise keeps candidate IDs rather than copying complete requests.

A live capture would change the calculation. New records could make an earlier summary or candidate list incomplete. We would need to update derived values, rebuild them, or calculate them when requested. Similarly, very large offline captures might favour delaying an expensive analysis until its view is opened. That saves initial work at the cost of a pause later.

The recent overlap optimization changes how quickly we locate eligible candidates, not which candidates qualify. Sorted start times allow a search to skip the whole range before a request's reported end. Other comparisons can still be expensive. The [measurement reference](../LARGE_CAPTURES.md) records actual samples and the remaining limits; a data structure's name alone is not performance evidence.

## Distinguish a reason from a story added later

A useful decision explanation has a constraint and a consequence. “The capture stays unchanged, so we can calculate its summaries once” supplies both. “We use this pattern because it is best practice” supplies neither.

Some project choices were documented before implementation, such as the replay transport contract. Other reasons in this chapter are an assessment of the code during the architecture audit. That assessment can reveal a reasonable tradeoff, an unresolved question, or unnecessary complexity. It should not pretend that all alternatives were benchmarked or discussed before the code existed.

The [architecture decision record](../ARCHITECTURE.md) distinguishes those cases. It links the implementation and evidence, names plausible alternatives, and states what would make us revisit a choice. It records open issues and their resolutions, including moving operation numbers from a process-global counter to an application owner. This lets the book teach the reasoning without turning each chapter into a maintenance log.

Try the opening question again: you want to add another capture format. Name the meanings its importer must preserve and the existing parts that should not need to change. Then change the requirement to a capture that grows while you watch it. Explain why that second change reaches beyond parsing into identity, summaries, and ownership.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### What Netscope is for

**Level:** Newcomer


In one or two sentences, what does Netscope do, and what does it never do without an explicit action from you?

<details>
<summary>Answer</summary>

Netscope is a terminal program for inspecting recordings of network traffic. It reads HAR files exported by a browser and classic PCAP packet files, and shows what each recording contains, including what it leaves out. It can also send one new GET or POST request based on a recorded one, but only after you review a separate draft and press Send. Opening a capture or a saved draft never sends anything.

**Learn more:** [An observation and an explanation need different labels](../chapters/02-reading-a-capture.md#an-observation-and-an-explanation-need-different-labels) · [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules)

</details>

### What does each crate own?

**Level:** Practitioner


The workspace has four crates: `netscope-core`, `netscope-import`, `netscope-net` and `netscope-tui`. What does each one own, which way do the dependencies point, and why does the core crate contain no file access, no network access and no async runtime?

<details>
<summary>Answer</summary>

`netscope-core` holds the data model and the pure analysis: sessions, requests and timings, redirect and retry suggestions, SSE and WebSocket evidence, packet types, TCP reassembly, wire HTTP framing, DNS and STUN recognition, replay drafts and the saved-draft format. Its only dependencies are `url`, `serde` and `serde_json`. `netscope-import` reads HAR and classic PCAP files into core types. `netscope-net` performs the one reviewed HTTP/1.1 exchange with tokio, hyper and rustls. `netscope-tui` depends on the other three and owns drawing, key handling and the `netscope` binary.

The dependencies point toward the data rules: the terminal crate depends on import and net, and both of those depend on core. Keeping I/O out of core makes its analysis deterministic and easy to test with plain values. It also gives a structural guarantee: the crates that parse captures have no network dependency, so opening a capture cannot send traffic. The workspace lints also forbid `unsafe` code and deny `unwrap` and `expect`.

**Learn more:** [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules) · [Package boundaries](../ARCHITECTURE.md#package-boundaries) · [`[dependencies]`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/Cargo.toml#L11-L14) · [`[workspace.lints.rust]`](https://github.com/sidsharma96/netscope/blob/main/Cargo.toml#L15-L23)

</details>

### Limitations to disclose for incident work

**Level:** Expert


A security engineer is evaluating Netscope for incident work. Name the five most important limitations you would disclose, and say for each whether it is a deliberate scope choice or a gap.

<details>
<summary>Answer</summary>

Any five of these, each labelled correctly, make a good answer.

Scope: only classic PCAP is read. PCAPNG is rejected, and macOS loopback (link type 0) and Linux `-i any` captures show "Link type unsupported". Scope: TLS is neither parsed nor decrypted, so HTTPS in a packet capture is opaque. Scope: IPv4 fragments are not reassembled, IPv6 extension headers are not walked, and checksums are not verified. Scope: wire HTTP is HTTP/1.1 with GET and POST requests only, and the two directions are not paired.

Gap: a response to HEAD is misframed, and port reuse without a captured SYN merges into the old epoch. Gap: replay trusts only the bundled root certificates and ignores proxies. Gap: saved drafts keep URL query tokens and unlisted secret headers, and the parent directory is not synced. Gap: terminal escaping misses bidi and zero-width characters. Gap: retry matching can be quadratic, and it misses real retries that change one header.

Scope: WebSocket support covers handshake header shape only, and SSE in a HAR has no timing. Scope: the import and reassembly budgets mean a large capture is analysed in part, and the views say so.

**Learn more:** [Known gaps and pitfalls](../chapters/34-defending-the-design.md#known-gaps-and-pitfalls) · [Put the limit next to the conclusion](../chapters/33-interpreting-a-trace.md#put-the-limit-next-to-the-conclusion) · [`layers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L171-L199)

</details>

### The four crates

**Level:** Newcomer


Netscope's workspace has four crates. What does each one do, and which ones can send network traffic?

<details>
<summary>Answer</summary>

`netscope-core` holds the data model and the pure analysis rules, such as sessions, timings, drafts and reassembly. `netscope-import` reads HAR and PCAP files into core types. `netscope-net` performs the one reviewed HTTP exchange. `netscope-tui` draws the terminal, handles keys and builds the `netscope` command. Only `netscope-net` sends traffic. Core depends on no terminal or network crate, so its rules can be tested without either.

**Learn more:** [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules) · [Keep a file format away from a screen's decisions](../chapters/23-shaping-the-program.md#keep-a-file-format-away-from-a-screens-decisions)

</details>

### Computing summaries once

**Level:** Newcomer


Netscope builds host summaries and redirect and retry candidates when it loads a capture. Why is that a sound choice here, and when would it stop being one?

<details>
<summary>Answer</summary>

A loaded capture never changes while you browse it, so answers computed once stay correct. Storing them costs memory for counts and lists of request IDs, not copies of request bodies, and it avoids recounting on every key press. If requests could arrive while you watched, as in a live capture, the stored summaries would go stale. They would need updating, rebuilding, or computing on demand.

**Learn more:** [Spend memory to avoid repeating a question](../chapters/23-shaping-the-program.md#spend-memory-to-avoid-repeating-a-question) · [Why calculate a group before drawing it?](../chapters/12-grouping-requests.md#why-calculate-a-group-before-drawing-it)

</details>
