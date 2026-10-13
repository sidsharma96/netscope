# Architecture decisions and open questions

Audited October 12, 2026 against application revision ffaa028. This record explains the current arrangement, alternatives, costs, and reasons to revisit it. It accompanies the reader-led lessons; it is not a claim that every possible design was evaluated before implementation.

**Evidence labels:** “Previously recorded” means a linked contract already states the choice or policy. “Audit assessment” means the rationale or alternative comparison was reconstructed during this review. “Open” identifies a question that this documentation update does not resolve. Alternative descriptions are reasoned comparisons, not benchmark results unless a measurement is linked.

The audit covers package boundaries, construction, data ownership and identity, validation, indexing, UI state, concurrency, transport, persistence, and verification. It does not document every local coding choice or certify that all current choices are optimal. Start with [Where should a change belong?](chapters/23-shaping-the-program.md) and [Who is allowed to change this value?](chapters/24-sharing-work.md) for the beginner explanations.

## Package boundaries

**Current choice; audit assessment.** Four workspace packages separate data rules, HAR import, replay I/O, and terminal coordination. Import and net depend on core; tui uses all three. Core has URL and serialization dependencies but no terminal loop or live network operations. Saved-file I/O remains in tui, while its format and validation live in core.

A single package with modules would reduce manifest/public-API overhead and could still separate responsibilities. The current package boundaries make dependency direction explicit and let rule tests run without terminal setup or a service. They add public interfaces and do not automatically prevent poorly placed responsibilities.

Revisit if another UI needs saved-file operations, if a dependency forces I/O into core, or if package maintenance outweighs its benefit. Do not describe the four packages as four services or processes. Evidence: [workspace](../Cargo.toml), [core manifest](../crates/netscope-core/Cargo.toml), [net manifest](../crates/netscope-net/Cargo.toml), and [saved-file operations](../crates/netscope-tui/src/saved_file.rs).

## Construction and instance scope

**Current choice; audit assessment.** Sessions and application state are explicitly constructed and passed to users of them. `new` is an ordinary Rust associated-function name, not a built-in constructor mechanism. TLS configuration is constructed per send and handed to the connector through `Arc`. There is no application-wide session singleton or abstract transport factory.

A global session would reduce parameter passing but hide which capture a function uses and complicate independent tests or multiple open captures. An abstract factory could select among interchangeable transport implementations, but currently there is only one production transport. The code instead substitutes a byte stream at the exchange boundary for controlled tests. That stream is a boxed trait object: one interface can hold either TCP, TLS, or a test stream without making the whole application choose a concrete type.

Revisit creation interfaces when a real second implementation or configurable transport policy appears. Per-send TLS setup and runtime creation cost work; caching immutable configuration or using a long-lived runtime are plausible alternatives that have not been benchmarked here. Evidence: [Session construction](../crates/netscope-core/src/session.rs), [worker/configuration/stream boundary](../crates/netscope-net/src/lib.rs). The global operation counter is a separate exception discussed below.

## Capture ownership and identity

**Previously recorded behaviour; audit assessment of alternatives.** A session owns source-order requests. Chronological and host views retain IDs, not copies of all request data. Browsing borrows the session. Editing copies selected request values into an independent draft; sending owns a reviewed snapshot. This preserves the recording while allowing edits and independently retained results.

Sorting the request vector itself would require either changing position-based IDs or adding another identity mechanism. Copying whole requests for each view would duplicate bodies. A shared mutable capture would require rules about which derived summaries become stale after edits. The current arrangement is suitable for an immutable offline recording, at the cost of extra ID vectors and deliberate copies when creating drafts.

`RequestId` and `HostId` distinguish categories, not session provenance. A number from another session can still resolve to an unrelated entry. Revisit identity before cross-session navigation, deletion, or live insertion. Evidence: [inspection contract](contracts/m1-har-inspection.md), [session model](../crates/netscope-core/src/session.rs), [draft contract](contracts/m3-single-request-replay.md), [collections lesson](chapters/08-structs-vectors-and-ids.md).

## Validation and incomplete evidence

**Previously recorded policy; audit assessment of representation costs.** Import rejects wrong types in consumed fields, duplicate JSON members, invalid required values, and limits. It ignores unconsumed metadata and preserves specific exporter variations. Body decoding can fail recoverably; invalid required data rejects the whole capture. Missing, unavailable, empty, and zero are distinguished where the model needs them.

Blindly accepting every field would postpone errors to screens and risk inconsistent interpretations. Rejecting every unknown extension would exclude useful native browser exports. Partial import could make damaged files useful, but would need explicit skipped-entry reporting and a policy for relationship analysis. Current rejection avoids displaying an incomplete request list as a complete session.

The importer builds a JSON value tree before typed requests. A streaming deserializer could reduce peak memory but complicate duplicate detection, field-path errors, and policy checks. File and entry limits do not cap total process memory to the file size. `Session::new` assumes validated values; it does not enforce that callers used the importer. Open questions: private validated construction if the public API grows, and peak-memory profiling before raising limits. Evidence: [importer](../crates/netscope-import/src/lib.rs), [inspection contract](contracts/m1-har-inspection.md), [native export results](HAR_COMPATIBILITY.md).

## Derived summaries and matching indexes

**Previously recorded policy; measured optimization.** Host summaries and redirect/retry candidates are calculated once when a session is constructed. Temporary ordered maps group lookup keys; stored candidate lists contain at most twenty IDs plus an overflow indication. Chronological indexes let matching skip starts before the reported end and stop past the window.

Calculating on each redraw would repeat work for an unchanged capture. Lazy calculation would reduce startup work for unused views but move that cost to the first inspection. Incremental updates would be needed for a live recording. Ordered maps give predictable key traversal; hash-based lookup is an alternative, but no map-type comparison was measured. Presentation order and timestamp tie-breaking are explicit rules, not an accidental consequence of map choice.

Costs include startup work and extra storage. In-window mismatches can still lead to quadratic comparisons. Revisit on measured startup/memory problems or a live-capture requirement. Evidence: [session](../crates/netscope-core/src/session.rs), [redirects](../crates/netscope-core/src/redirects.rs), [retries](../crates/netscope-core/src/retries.rs), [large-capture measurements](LARGE_CAPTURES.md).

## Screen state and independent effects

**Previously recorded behaviour; audit assessment of alternatives.** Input becomes an action that changes view state; rendering borrows recorded data. Screen and editor-mode enums name alternatives and sometimes carry their required values. Pending sends and saves are consumed by terminal coordination, so entering text or displaying a review does not itself perform the effect.

Independent Boolean flags would allow combinations such as “editing and sending” unless every transition repaired them. A general state-machine framework could centralize transitions but adds an abstraction before the current states require it. Current enums improve clarity without proving every invariant: optional result/pending fields must still agree with the mode, and transition tests enforce those relationships.

Revisit if new modes make those combinations difficult to reason about. Evidence: [view state](../crates/netscope-tui/src/view.rs), [editor](../crates/netscope-tui/src/replay_view.rs), [terminal loop](../crates/netscope-tui/src/terminal.rs), [navigation lesson](chapters/11-drawing-and-navigation.md). Pure state and buffer tests complement real-terminal checks; neither substitutes for the other.

## Worker ownership and communication

**Previously recorded design; audit assessment of alternatives.** Each accepted send gets a worker thread and a current-thread Tokio runtime. The UI retains one worker, a cancellation sender, and a result receiver. The worker owns response bytes until handing over one final result through a capacity-one channel. There is no application mutex-protected shared response body or live progress queue.

Running a blocking exchange on the terminal thread would prevent input handling. A fully asynchronous terminal loop with a long-lived runtime could avoid per-send setup, but would reorganize input, cleanup, and testing. A shared response buffer under a mutex could support live inspection but needs short locking periods and consistency rules. A bounded chunk channel is another streaming option; it needs a policy for a slow consumer. None is implemented or benchmarked as an alternative here.

The current design gives cleanup and response ownership one location, with setup cost and no streaming preview. Drop joins the worker after cancellation, so cleanup itself can briefly occupy the UI thread. OS resolver work may outlive cancellation; it cannot modify the editor or publish a late UI result. Revisit before concurrent sends, live progress, or stricter cancellation-latency requirements. Evidence: [transport decision](contracts/replay-transport.md), [worker](../crates/netscope-net/src/lib.rs), [sharing lesson](chapters/24-sharing-work.md).

## Cancellation and completion authority

**Previously recorded policy.** The UI accepts already delivered completion before a later cancel key. Within the worker, simultaneous readiness prioritizes cancellation, then the absolute deadline, then exchange completion. Dropping the exchange disposes of its socket; partial result data lives outside that future and survives. Cancellation stops local waiting, not necessarily an action already performed by the server.

Detaching a worker without cleanup could leave traffic running after the UI appeared to cancel. Clearing the whole result would lose useful partial evidence. Adding competing atomic flags for completion would create another place deciding the outcome. Current channels and selection already supply an authority, so application-level compare-and-swap is not used for this purpose.

Revisit if operation ownership changes or multiple producers can finish one logical operation. Evidence: [waiting lesson](chapters/17-waiting-for-a-response.md), [transport contract](contracts/replay-transport.md), [network tests](../crates/netscope-net/src/tests.rs). Controlled scheduling tests support specified cases, not a claim that every possible interleaving has been exhaustively explored.

## Operation identity

**Open simplification; audit assessment.** The editor allocates send IDs from a process-global `AtomicU64` using `fetch_add(1, Relaxed)`. Completion is accepted only for the active ID. The atomic operation prevents a lost increment if multiple callers allocate concurrently. Relaxed ordering is sufficient for this isolated counter; result transfer and mutable state safety do not rely on the counter publishing other memory. Its value wraps and is not persistent identity.

The present terminal loop allocates on one UI thread and starts at most one worker. Therefore the code does not demonstrate a need for globally synchronized allocation. A counter owned by the long-lived application could avoid global state, provided its scope spans editor replacement and overflow is handled deliberately. A counter reset for every editor would weaken the intended separation of operations. A mutex is unnecessary for this single atomic increment; a handwritten CAS loop would restate an existing operation without solving another requirement.

This audit leaves behaviour unchanged. Before changing allocation, specify its required uniqueness scope, define exhaustion behaviour, and test stale completion after editor replacement and repeated sends. Do not assert exact global counter numbers in tests that can run concurrently. Evidence: [editor allocation and completion checks](../crates/netscope-tui/src/replay_view.rs), [terminal worker identity](../crates/netscope-tui/src/terminal.rs), [counter and CAS lesson](chapters/24-sharing-work.md).

## HTTP stack and replay scope

**Previously recorded selection; audit assessment of alternatives.** Replay uses Hyper's HTTP/1 connection API, Tokio, and rustls with bundled WebPKI roots. A reviewed operation opens one connection, sends one GET/POST, and retains a bounded response. Redirect following, application retries, proxy discovery, cookie storage, and decompression are disabled. Browser HAR protocol recognition does not imply replay support for that protocol.

A higher-level client could reduce connection-management code and offer pooling, redirects, and decompression, but those policies would need deliberate configuration and separate tests. A handwritten parser would increase protocol work and correctness risk. No head-to-head client benchmark or comprehensive dependency evaluation was recorded; the supported reason for the current stack is control over this narrow connection lifecycle while delegating HTTP parsing and TLS.

Costs include more lifecycle code, no pooling, and HTTP/1.1-only replay. Revisit for sustained sends, H2/H3 replay, proxy support, or new TLS requirements. Evidence: [dated transport decision](contracts/replay-transport.md), [network implementation](../crates/netscope-net/src/lib.rs), [replay contract](contracts/m3-single-request-replay.md), [browser compatibility](HAR_COMPATIBILITY.md).

## Saved files and replacement

**Previously recorded policy; audit assessment of alternatives.** A versioned JSON file retains selected draft values, not a live worker or session snapshot. Known credential headers are excluded; an exact preview precedes confirmation. Saving writes and syncs an exclusively created sibling temporary file before renaming it over the destination.

Writing directly to the destination could destroy the previous contents before a failed write finishes. A database could support indexed collections, migrations, and multi-record transactions, but introduces lifecycle and schema work for today's one-request-per-file workflow. A filesystem lock could coordinate cooperating writers but would not itself provide crash durability. Current exclusive temporary creation refuses a conflicting temporary path rather than silently overwriting it.

Costs and limits: synchronous local I/O, possible leftover temporary files after interruption, no directory sync or universal power-loss guarantee, and no multi-writer conflict-resolution protocol. Credential filtering is not a guarantee that other values contain no secrets. Revisit for shared storage, concurrent editing, automatic migration, or durability requirements. Evidence: [saved contract](contracts/m3-saved-drafts.md), [value format](../crates/netscope-core/src/saved.rs), [file operations](../crates/netscope-tui/src/saved_file.rs), [persistence lesson](chapters/20-remembering-a-request.md).

## Evidence and testing boundaries

**Previously recorded strategy; audit assessment of limits.** Pure state tests, fixed terminal buffers, frozen browser HAR files, paused/in-memory network tests, loopback integration tests, and real pseudo-terminal checks establish different behaviours. Timing measurements live outside pass/fail regression assertions. The lockfile, pinned toolchain, formatting, and warnings-denied Clippy reduce accidental variation.

Live services in the main suite would add changing inputs; mocking every boundary would miss interoperability and actual cleanup failures. Current tests combine controlled logic checks with explicit integration checks. Browser exports record particular versions and scenarios, not universal compatibility. Compilation establishes Rust example correctness, not reader understanding. Revisit the test boundary when a feature changes an assumption such as shared state, multiple workers, or file concurrency. Evidence: [development guide](DEVELOPMENT.md), [verification ledger](VERIFICATION.md), [quality lesson](chapters/04-quality-and-tests.md).

## Keeping this record useful

For a material change, update the affected decision with the actual constraint, choice, alternatives, cost, and revisit condition. Record a decision date separately from a later audit date. Link tests or measurements for factual claims; label untested alternatives as assessments. Explain the relevant reasoning in the existing lesson when it helps the reader, rather than adding a chapter for every patch.

The current follow-ups are to review operation-ID ownership and exhaustion, measure per-send setup before changing runtime/configuration reuse, and profile peak import memory before changing limits. They are open questions, not promises of scheduled work. This audit adds explanations and executable teaching examples; it does not implement those redesigns.
