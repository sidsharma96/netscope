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

Revisit creation interfaces when a real second implementation or configurable transport policy appears. Per-send TLS setup and runtime creation cost work; caching immutable configuration or using a long-lived runtime are plausible alternatives that have not been benchmarked here. Evidence: [Session construction](../crates/netscope-core/src/session.rs), [worker/configuration/stream boundary](../crates/netscope-net/src/lib.rs). Send-ID allocation now belongs to application state, as described below.

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

## Inspection workspace and replay workbench

**Chosen October 13, 2026 during the layout discussion.** The reader approved a default split inspector, optional explanatory journey, and separate replay workbench. Wide inspection keeps the filtered request list beside tabbed details. The existing overview/inspector states identify which pane receives movement keys; an independent section enum selects the information shown. Rendering still borrows the immutable session. At 80–99 columns, the same states draw one pane at a time. The complete report remains available as All rather than dropping fields to simplify the default view.

A permanent three-pane layout would leave too few columns for bodies and long headers at the supported sizes. Keeping only the previous scrolling report would preserve simple rendering but require searching through unrelated fields. Removing that report entirely would risk hiding exporter metadata and obscure evidence boundaries. The chosen sections reduce navigation work while retaining a complete fallback. Keeping the journey optional prevents explanatory prose from taking space during routine inspection; its arrows are illustrative, not claims about recorded handshake messages.

Replay uses the existing editor modes to choose its active pane and highlight Edit/Review/Send/Result. Introducing independent clickable progress steps or pane modes would require additional transitions around review invalidation and running operations. The current workbench changes presentation without adding another authority for sending. Text editors retain their existing keys, including body Tab insertion; old retained results identify their own destination. Comparison and exact save previews use the full width.

Costs include additional layout/section state and two responsive compositions. Some titles/tab labels shorten in split panes, and body/field wrapping needs more scrolling. View calculations and fixed buffers must agree on available dimensions. No performance advantage is claimed; on-demand interpretation remains bounded by the existing policies. Revisit the 100-column split threshold after user experience at real terminal sizes, or the state model if simultaneous sends, independently editable panes, or live capture are added. Evidence: [layout contract](UI_LAYOUT.md), [workspace renderer](../crates/netscope-tui/src/workspace_view.rs), [navigation/editor tests](../crates/netscope-tui/tests/views.rs), and the dated verification entry after checks execute.

## Worker ownership and communication

**Previously recorded design; audit assessment of alternatives.** Each accepted send gets a worker thread and a current-thread Tokio runtime. The UI retains one worker, a cancellation sender, and a result receiver. The worker owns response bytes until handing over one final result through a capacity-one channel. There is no application mutex-protected shared response body or live progress queue.

Running a blocking exchange on the terminal thread would prevent input handling. A fully asynchronous terminal loop with a long-lived runtime could avoid per-send setup, but would reorganize input, cleanup, and testing. A shared response buffer under a mutex could support live inspection but needs short locking periods and consistency rules. A bounded chunk channel is another streaming option; it needs a policy for a slow consumer. None is implemented or benchmarked as an alternative here.

The current design gives cleanup and response ownership one location, with setup cost and no streaming preview. Drop joins the worker after cancellation, so cleanup itself can briefly occupy the UI thread. OS resolver work may outlive cancellation; it cannot modify the editor or publish a late UI result. Revisit before concurrent sends, live progress, or stricter cancellation-latency requirements. Evidence: [transport decision](contracts/replay-transport.md), [worker](../crates/netscope-net/src/lib.rs), [sharing lesson](chapters/24-sharing-work.md).

## Cancellation and completion authority

**Previously recorded policy.** The UI accepts already delivered completion before a later cancel key. Within the worker, simultaneous readiness prioritizes cancellation, then the absolute deadline, then exchange completion. Dropping the exchange disposes of its socket; partial result data lives outside that future and survives. Cancellation stops local waiting, not necessarily an action already performed by the server.

Detaching a worker without cleanup could leave traffic running after the UI appeared to cancel. Clearing the whole result would lose useful partial evidence. Adding competing atomic flags for completion would create another place deciding the outcome. Current channels and selection already supply an authority, so application-level compare-and-swap is not used for this purpose.

Revisit if operation ownership changes or multiple producers can finish one logical operation. Evidence: [waiting lesson](chapters/17-waiting-for-a-response.md), [transport contract](contracts/replay-transport.md), [network tests](../crates/netscope-net/src/tests.rs). Controlled scheduling tests support specified cases, not a claim that every possible interleaving has been exhaustively explored.

## Operation identity

**Resolved October 12, 2026 following the audit.** Send IDs are allocated from an ordinary `Option<u64>` in `App`, starting at 1. The UI passes exclusive mutable access to that state while handling a reviewed Send. The owner outlives editor replacement, and completion/start-failure handling accepts only the active operation's ID. Scope is one App and its worker loop; unrelated App instances can reuse numbers because results are not routed between them.

Validation precedes allocation. A queued send consumes an ID even if worker startup later fails. The allocator uses checked addition: u64::MAX is available once, then None means exhausted. Further Send actions remain in Review, preserve the draft, show a restart explanation, and queue no operation. Exhaustion survives editor replacement. A new App starts a new scope with its own workers; this is not persistent identity or unlimited allocation.

The earlier implementation used a process-global AtomicU64 with relaxed fetch_add. That was sufficient for the isolated increment but supported a broader concurrency scope than the UI needed and wrapped at exhaustion. A mutex or CAS loop would add coordination without another allocator. Moving a counter into each editor would reset too soon. The application owner expresses the actual lifetime and permits deterministic independent tests. Revisit if sends or completions cross App boundaries or several threads begin allocating IDs.

Evidence: [application allocation state](../crates/netscope-tui/src/view.rs), [review validation and allocation](../crates/netscope-tui/src/replay_view.rs), [editor-replacement and stale-result tests](../crates/netscope-tui/tests/views.rs), and the private near-exhaustion test in view.rs. The worker loop still retains one worker and checks completion within that App. The [sharing lesson](chapters/24-sharing-work.md) retains atomics and CAS as alternative mechanisms, and explains why ordinary ownership fits the current counter.

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

Operation-ID ownership and exhaustion were resolved on October 12, 2026 as recorded above. Remaining follow-ups are to measure per-send setup before changing runtime/configuration reuse and profile peak import memory before changing limits. These remain open questions, not promises of scheduled work.

## Cached URL search over captured identities

**Decision recorded October 12, 2026.** A literal, case-sensitive full-URL substring combines with the host condition. The imported capture stays fixed. The terminal application owns an applied query, a separate temporary edit, and a vector of matching source IDs. Rebuild on query or host changes; borrow that vector for drawing and navigation. Before rebuilding, remember the selected ID and locate it in the new list, falling back to the first result or no selection. Host-only return positions remain available; query edits invalidate saved row positions because they belong to a different list.

Scanning on every redraw would avoid the extra ID vector but repeat work while browsing unchanged data. Copying complete matching requests would duplicate bodies and make capture identity harder to maintain. Regular expressions, decoded matching, and case folding could broaden search, but introduce additional semantics; literal retained text supplies a predictable first contract. No alternative was benchmarked for this decision.

The vector costs one ID per match and requires complete invalidation at every filter change. Query editing is bounded to 1024 UTF-8 bytes; applying still scans the candidate list synchronously and has no measured latency guarantee. Revisit for mutable captures, more search fields, or measured input delays. Background search would need completion ordering and cancellation rules. Evidence: [search contract](contracts/m4-url-search.md), [view state](../crates/netscope-tui/src/view.rs), [selection and editing tests](../crates/netscope-tui/tests/views.rs), [reader lesson](chapters/11-drawing-and-navigation.md).

## Temporary captured SSE interpretation

**Decision recorded October 13, 2026.** A reported text/event-stream MIME essence selects a bounded, on-demand interpretation of retained UTF-8 body text. It produces ordered events at complete blank-line boundaries, with explicitly parse-local persistent ID state, a retained retry instruction, unavailable representations, and an unfinished tail. The original body remains unchanged. No event timing, browser acceptance/delivery, reconnection, or capture completeness is inferred.

Joined data requires owned strings; type, ID, retry, and pending text borrow the existing body. Storing a parsed list for every request would retain extra memory even when most streams are never opened. A self-contained owned cached result would copy metadata; a cache borrowing its owning session would need additional lifetime arrangements. Parsing only the raw 16 KiB display preview would silently confuse display truncation with protocol boundaries. Instead, analysis has independent 64 KiB/100-event budgets and separate display-shortening labels.

On-demand parsing repeats bounded work when drawing and clamping the inspector. This avoids cache invalidation and additional persistent state but is not benchmarked as a latency guarantee. Fixed budgets can leave useful later events unexamined, so counts describe the examined prefix and limits are visible. Retry remains digit text rather than a fixed-size integer, preserving arbitrarily large valid settings without overflow. MIME classification uses the retained content label alone; reconciling response headers is deferred rather than guessed.

Revisit for measured inspection delays, configurable exploration beyond the prefix, live chunks with partial UTF-8, or connection-state continuity. A live parser would need persistent pending state between chunks, decoding policy, cancellation, and actual observation timestamps. Evidence: [contract](contracts/m5-captured-sse.md), [parser](../crates/netscope-core/src/sse.rs), [view](../crates/netscope-tui/src/sse_view.rs), [lesson](chapters/25-reading-event-streams.md), and the linked deterministic tests. The protocol source is the [HTML Standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#interpreting-an-event-stream); retained-text inspection is a narrower policy than a running browser EventSource.

## M5/M6: inspect agreements and bound new stream observations

Decision made October 13, 2026, after the reader approved the proposed explanations. This is a contemporaneous implementation record, not evidence of a formal usability study.

**Constraints.** Native HAR exports retain SSE text and classic WebSocket upgrade metadata but do not reliably contain frame history or per-event observation times. Live observation must remain explicit, cancellable, responsive, and separate from imported facts. The installed sender already owns a verified-TLS HTTP/1.1 connection per reviewed operation.

**Choice.** Import ws/wss URLs for inspection, reject their replay, and report classic HTTP/1.1 upgrade header shape with explicit unverified acceptance checks. Preserve Chrome's observed negative 101 bodySize in a warning while representing size as unavailable; accept its empty server address as absent only for ws/wss 101 entries. Keep other malformed sizes and addresses rejected. Browser fixture bytes remain unchanged. No WebSocket frame decoder or conversation client is added.

Live SSE gets a second review screen and one GET, client-managed Accept and identity encoding, a 30-second absolute deadline, 64 KiB retained raw bytes, and 100 observed event boundaries. Response status/type/encoding and UTF-8 are checked. A worker-owned bounded prefix is reparsed using the existing SSE rules; incomplete UTF-8 waits, invalid text stops, EOF does not dispatch pending data. A monotonic duration begins at accepted Send and records when a read first supplies a complete boundary. No server, packet, or browser time is inferred.

**Alternatives and consequences.** An incremental parser would avoid repeated prefix parsing but requires additional state for partial characters, lines, event fields, and interruption. At the current small limits, cumulative parsing favours reviewability; it costs repeated parsing and bounded copying. A queue of individual updates could accumulate if drawing stalls. A blocking queue could make UI speed affect observation. Tokio's watch channel holds one latest cumulative snapshot, so intermediate publications may be skipped while earlier retained observations remain in the latest one. The final outcome stays on the existing once-only channel; progress is drained again when accepting it. Operation identity rejects stale progress. No shared mutable UI/event list is introduced.

A fully reconnecting EventSource client, WebSocket client, or HTTP/2 live sender would broaden the network contract and require independent state, evidence, and tests. This milestone instead teaches HTTP/2 multiplexing, DATA flow control, and HPACK using standard-defined illustrations and existing H2/independent server evidence. A HAR cannot supply missing compressed header or frame bytes merely because a decoder is added.

**Evidence.** [Native fixtures and provenance](../tests/fixtures/har/browsers/README.md), [browser regression tests](../crates/netscope-import/tests/browser_exports.rs), [bounded live parser/publication tests](../crates/netscope-net/src/streaming.rs), [network tests](../crates/netscope-net/src/tests.rs), [terminal checks](../scripts/check_replay_terminal.py), and [contracts](contracts/m6-live-sse.md). Lessons: [switching protocols](chapters/26-switching-protocols.md), [local event times](chapters/27-observing-live-events.md), [sharing a connection](chapters/28-sharing-a-connection.md).

**Revisit.** Measure parsing/publication cost before larger budgets or high-throughput claims. Revisit streaming state before reconnect/resume, compression decoding, exporting observations, additional methods, HTTP/2 transport, or richer WS/packet input. Frame claims require suitable instrumentation; current browser experiments establish neither concurrent frame interleaving nor wire header compression ratios.

## M7/M8: bounded packets, explicit sequence evidence and narrow application recognition

Decision made October 13, 2026 while completing the remaining planned milestones. The user requested implementation and continuing book upkeep; no claim of a subsequent formal learner study is made.

**Constraints.** External binary lengths and sequence positions can be malformed or enormous. A packet capture has record timestamps and possibly truncated frame bytes, not HAR request/timing semantics. The learning goal requires understanding byte order, checked slicing, ordering and missing evidence. The terminal must retain existing HAR/replay behavior and support packet inspection without live-capture privileges.

**Choices.** Use explicit --pcap to select a classic 2.4 reader with bounded container validation, then separate checked layer interpretation. A malformed container aborts; unsupported/cut inner layers retain record bytes and valid preceding metadata. Own each record's byte vector and store payload ranges, borrowing only during analysis. No native packed-header casts, unsafe code or self-referential mapping. Keep PacketCapture separate from Session and a small separate packet navigation state using the existing terminal restore guard. No packet replay or network worker is created.

A pure directed-epoch analysis places TCP payload at bounded modular sequence positions. Maintain a dense byte vector plus explicit Missing/Observed/Conflict states. Declared captured cuts leave holes, agreeing repeats count observations, and disagreements get no first/last-wins policy. Conservative SYN/close boundaries separate reuse; absence of SYN stays unknown. Limit directed epochs, per-span and aggregate span allocation before growth; refused epochs cannot fall back into an old active tuple. HTTP examines only the uniquely observed prefix, never concatenation through holes. Derivation is separate from original bytes and does not validate checksums or receiver acceptance.

HTTP/1.1 framing is a narrow candidate inspector: bounded start/header lines, GET/POST starts, response status starts, single lengths/chunked transfer, explicit chunks/trailers and qualified close delimiting. Opposite directions/methods are not paired, so response framing is conditional. Invalid/unsupported/conflicting framing stops instead of guessing pipelined starts. No HTTP/2/3 wire decoding, TLS decryption or body decompression. SDP recognition is a labelled MIME-and-line rule; UDP DNS and STUN are bounded metadata candidates without resolver, balancing, identity, ICE or media-success conclusions.

**Alternatives and consequences.** A maintained PCAP/protocol library could broaden compatibility; the present limited checked reader keeps the wire-learning exercise reviewable and makes unsupported cases explicit. It costs narrower format/protocol coverage and requires adversarial boundary tests. A memory map would avoid copies but require different lifetime/resource ownership; current file/record budgets keep owned vectors straightforward. Sparse sequence structures could retain distant data without a large dense hole, but add per-entry overhead and more ordering operations. The bounded dense span makes overlaps/conflicts direct and reproducible, at a cost of allocating holes within its limit. Sender/receiver semantics would need richer state, not simply sorting packets or choosing an overlap winner.

A shared generalized HAR/PCAP UI would risk treating packets as requests with invented timing or replay meaning. Separate navigation repeats a small loop but borrows the same recording/restoration components. Rendering reparses bounded selected stream prefixes and allocates temporary display strings; no latency or zero-allocation guarantee is asserted. Output preview limits are separate from retained-data/analysis limits.

**Evidence.** [M7 contract](contracts/m7-packets.md), [M8 contract](contracts/m8-streams.md), [fixed fixtures and provenance](../tests/fixtures/pcap/README.md), import/sequence/framing regression tests, six fixed buffers, [real PTY workflow](../scripts/check_packet_terminal.py), installed-binary checks and [verification ledger](VERIFICATION.md). An independent tcpdump read cross-checked constructed fields/checksums; it does not establish real traffic. Five reader-led lessons explain mechanisms, ownership, alternatives and limits, with checked Rust byte/sequence/framing examples.

**Revisit.** Before PCAPNG/additional link types, fragments/extension chains, arbitrary TCP history, true handshake/request pairing, richer DNS/STUN/SDP, decryption, saved/live packet capture or larger workloads. Benchmark before growing dense budgets or caching parsed views. Congestion, packet loss, load balancing and media outcomes need independent evidence/instrumentation; the current book teaches their limits instead of claiming analyses not implemented.

## Native campaign: retain narrow nonstandard observations without inventing measurements

Decision made October 13, 2026 from the executed 120-scenario campaign, not a historical rationale. Native Chrome captures exposed blocked timings between -1 and zero and empty server addresses on failed status-zero WebSocket attempts. Rejecting them discarded otherwise inspectable evidence.

Keep a distinct ExporterNegative timing variant only for the observed blocked interval, with exact value/warning and a non-duration display. The standard Timing constructor and all other fields remain strict. Do not clamp to zero, substitute -1, or admit arbitrary negative measurements. Empty ws/wss addresses on zero/101 status represent absence; no response status/IP is reconstructed. Original HAR bytes remain the provenance record.

Alternative choices were rejecting the entire capture, repairing values, or relaxing every timing/address field. The narrow distinction costs another state/formatter branch and requires explicit regression coverage, but preserves meaning and existing validation. Captured-text and dispatch discrepancies stay external evidence; import does not transcode text by guessing a browser's decoder. The [campaign report](COMPATIBILITY_CAMPAIGN.md), frozen corpus/checker and focused timing/address regressions provide evidence. Revisit only with additional native cases and defined semantics, not a broad “accept anything” policy. Live production observations and browser test-certificate exceptions remain separate trust boundaries.
