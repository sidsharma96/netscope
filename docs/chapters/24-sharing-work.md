# Who is allowed to change this value?

A response is arriving while you look at the terminal. The network code receives another block of bytes, and the screen wants to show what has arrived. Could both parts of the program simply use the same growing body?

They could, but then we would need rules about when each part may read or change it. Netscope currently chooses a different arrangement: the worker owns the body while receiving it and passes the result back when the operation ends. Before choosing a lock or an atomic variable, we should ask whether two parts of the program need to change the same value at all.

The [waiting chapter](17-waiting-for-a-response.md) introduced threads and cancellation. Here we compare the ways those threads could cooperate. Some examples describe alternatives to Netscope's current design; they are labelled where they appear.

## Hand the value over when responsibility changes

Suppose the response contains three bytes when you cancel. The worker already owns those bytes. It can finish cleanup and send a result containing them to the terminal. The terminal then owns the result it displays. There is no period in which both threads need to edit that result together.

A [channel](../GLOSSARY.md#channel) provides the handoff. Its sender puts a value into the channel; its receiver takes that value out. Netscope uses a result channel with room for one value because each operation produces one final outcome. Cancellation has a separate channel carrying a signal in the other direction.

Read the diagram as a transfer of responsibility. It describes the current design, without implying a particular operating-system schedule.

```text
Terminal thread                         Worker thread
owns editable draft                     owns reviewed request and response bytes
       │                                         │
       ├──────── cancellation signal ───────────►│
       │                                         ├─ stop exchange and close socket
       │◄──────── owned result ──────────────────┤
       └─ owns result for display
```

The result body moves with the result; sending does not require making a second copy of its entire buffer. The channel coordinates the transfer so the receiver can use the value when it receives it. This does not mean channels have no internal synchronization. It means the application does not have to coordinate simultaneous edits to that body. See Rust's [message-passing explanation](https://doc.rust-lang.org/book/ch16-02-message-passing.html).

The ordinary replay result arrives once. Showing progress while a response remains open needs another decision: periodic snapshots copy data; sending chunks needs a bounded queue and a rule for a slow receiver; sharing a buffer needs coordinated access. Netscope's separate live SSE mode publishes bounded cumulative snapshots, explained in the [live observation lesson](27-observing-live-events.md). The once-only result channel still carries the final outcome. A channel should be chosen for the information being transferred, rather than assumed to fit every update.

## Share lifetime only when lifetime is the problem

Sometimes several values need to keep the same configuration alive without changing it. Rust's `Arc<T>` is a shared owner for a value of type `T`. Cloning an `Arc` creates another owner of the same allocation; it does not clone the contained configuration. The contained value is dropped after the last strong owner goes away.

The name means atomically reference-counted: its ownership count can be updated safely across threads. That protects the count, not arbitrary edits to the contained value. Wrapping a growing response body in `Arc` alone would not let one thread mutate it while another reads it. Netscope uses `Arc` for its TLS configuration because the TLS connector accepts shared ownership of that configuration. It is not a shared mutable response store. The [Arc documentation](https://doc.rust-lang.org/std/sync/struct.Arc.html) explains this distinction.

For a value that never needs shared ownership, an ordinary owner and temporary borrows are easier to follow. For a draft that must change independently, copying the needed text is appropriate. Shared ownership, borrowed access, and independent copies solve different lifetime questions.

## Protect several related changes together

Imagine a future progress display with two fields: a byte count and the buffer containing those bytes. Its rule is that the count equals the buffer length. A reader must not see the new count with the old buffer.

A [mutex](../GLOSSARY.md#mutex), short for mutual exclusion, can protect the pair. Code acquires the mutex before accessing the pair. While one thread holds that access, another trying to acquire it must wait. The writer can update both fields before allowing another reader in. The protected interval is often called a critical section.

Rust represents that access with a guard. Dropping the guard releases the lock. The [Mutex API](https://doc.rust-lang.org/std/sync/struct.Mutex.html) also reports poisoning: a thread may have panicked while holding the lock, leaving an application rule unfinished. Recovery needs to consider the data's meaning rather than simply ignore that signal.

The design still needs judgment. Holding this lock while waiting for the next network byte could keep the screen waiting too. Two threads that each hold one lock while waiting for the other's lock can become stuck permanently; that is a [deadlock](../GLOSSARY.md#deadlock). Rust's borrowing checks do not prove that a locking design will always make progress. The Rust book discusses these costs in [shared-state concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html).

For our current final-result display, sharing a mutable body would introduce that coordination without a user-visible need. For a future shared cache with several related fields, a short, carefully scoped lock might be the clearest solution. “Avoid locks” is not a sufficient design rule; first identify the consistency rule the lock would protect.

## Update one counter without losing an increment

Consider two callers asking for the next operation number. A separate read followed by a separate write is not enough coordination. Both could read 7, both compute 8, and both believe they reserved the same number.

An [atomic operation](../GLOSSARY.md#atomic-operation) makes its change indivisible with respect to other atomic operations on that value. An atomic read-modify-write operation reads a value, changes it, and returns a result as one operation. Here is an isolated example of an increment suitable for callers sharing a counter. The example runs sequentially so its result is repeatable; it is not a stress test of competing threads.

```rust
use std::sync::atomic::{AtomicU64, Ordering};

let next = AtomicU64::new(7);
let first = next.fetch_add(1, Ordering::Relaxed);
let second = next.fetch_add(1, Ordering::Relaxed);
assert_eq!(first, 7);
assert_eq!(second, 8);
```

`use` brings these library names into scope. `AtomicU64` holds an unsigned 64-bit integer with atomic access methods. `new(7)` starts it at 7. The `fetch_add` method adds its first argument and returns the value from before the addition. The example therefore reserves 7 and then 8. The variable binding does not need `mut`: this type provides controlled changes through its atomic methods. The [atomic integer API](https://doc.rust-lang.org/1.92.0/std/sync/atomic/struct.AtomicU64.html#method.fetch_add) specifies the return value and overflow behaviour.

Memory ordering answers a separate question: what does this operation establish about access to other memory? `Relaxed` keeps the counter operation atomic but does not use it to publish unrelated request or response data. Acquiring a new number therefore does not prove that another thread's response body is ready. The response travels through the channel, which supplies its own coordination. See the [ordering definitions](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html).

If an atomic flag were instead used to announce that other shared data was ready, `Relaxed` alone would not establish the needed ordering. Release and Acquire can form such a relationship when an acquiring read observes the releasing write. That relationship still needs a correct ownership and access design for the other data. Stronger ordering on a counter does not automatically make a mutable buffer safe to share.

Netscope gives each send a number so a completion can be checked against the active operation. But only the terminal thread allocates those numbers. The current implementation therefore uses an ordinary counter owned by the application, not the atomic counter in the illustration. An editor can be closed and replaced while the application remains alive, so the counter belongs to that longer-lived owner. Starting the sequence again in each editor could let an old result carry the same number as a newer send.

The counter also needs an end-of-range rule. Its next value is an `Option<u64>`: either a number remains available, or the sequence is exhausted. Accepting Send reserves the current number and uses checked addition to prepare the next. Checked addition returns no value if the result would exceed the integer's range. The largest number can consequently be used once; a later Send stays in Review with an explanation and queues no work. It does not wrap back to an earlier number.

These IDs only distinguish operations within one application instance. They are not persistent request identities, and separate instances may allocate the same numbers because they own separate workers and results. If allocation later moves to several threads, we must reconsider how those threads coordinate it. The [decision record](../ARCHITECTURE.md#operation-identity) explains the change from the earlier global atomic and the tests protecting this ownership rule.

## Change a value only if the assumption still holds

Now imagine two callers trying to claim a single unused slot. Each needs to say, “Set it to occupied, but only if it is still empty.” This is [compare-and-swap](../GLOSSARY.md#compare-and-swap), commonly abbreviated CAS. It is an atomic operation rather than a third category beside atomics and locks.

Rust calls the operation `compare_exchange`. This illustrative example claims a Boolean slot once. A Boolean holds either `false` or `true`; it does not carry a response body.

```rust
use std::sync::atomic::{AtomicBool, Ordering};

let occupied = AtomicBool::new(false);
let first = occupied.compare_exchange(
    false, true, Ordering::Relaxed, Ordering::Relaxed,
);
let second = occupied.compare_exchange(
    false, true, Ordering::Relaxed, Ordering::Relaxed,
);
assert_eq!(first, Ok(false));
assert_eq!(second, Err(true));
```

The first argument is the expected value; the second is the replacement. The last two arguments specify ordering on success and failure. Both are relaxed here because the Boolean itself is the entire information being coordinated. A successful call returns `Ok` with the previous value. A failed comparison returns `Err` with the value it observed and leaves the slot unchanged. The first call claims it; the second discovers that its assumption is no longer true.

This example never releases the slot. If a real design allowed a value to change away and then back, comparing its current value would not reveal that intervening history. Retrying a failed comparison also needs care: a failed attempt must not have already performed an irreversible action such as sending a request. The [atomic Boolean API](https://doc.rust-lang.org/1.92.0/std/sync/atomic/struct.AtomicBool.html#method.compare_exchange) describes the operation's exact contract.

Netscope does not use application-level CAS to resolve cancellation. The worker already chooses among completion, cancellation, and deadline in one asynchronous selection, and the terminal accepts one result. Adding a second competition through an atomic flag would need a reason and a clearly defined authority. Nor does an ID increment need a handwritten CAS retry loop: `fetch_add` already expresses the required operation.

## Choose from the data relationship

For a single UI owner changing a selection, use an ordinary value. For a worker handing over a finished result, a channel expresses the transfer. For several owners keeping unchanged configuration alive, shared ownership may fit. For related shared changes that must be seen together, consider a lock. For one independently meaningful shared counter or conditional flag, consider an atomic operation.

These choices can coexist in one program. None is a universal performance ranking. The question is which rule must remain true, who can change the data, and what another part of the program may conclude when it reads it.

Try changing the requirement: the terminal must show every received byte immediately, even when drawing is slow. Explain what happens to memory with an unlimited queue, what a bounded queue would force the sender to do, and why an atomic byte count alone cannot supply the missing body. Then relax “every byte immediately” to “occasional progress updates.” Which design becomes simpler?

The worker [`Worker`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L312), terminal loop [`run_app`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L158), and replay editor [`ReplayEditor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L31) show today's ownership and ID choices. The [decision audit](../ARCHITECTURE.md) records their constraints, alternatives, evidence, and reasons to revisit them.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Waiting without freezing the screen

**Level:** Newcomer


A server accepts your request and then sends nothing. How can Netscope still react when you press Esc?

<details>
<summary>Answer</summary>

Each Send runs on its own worker thread, so the terminal thread keeps reading keys. Inside the worker, a Tokio runtime drives the exchange as a future, a value representing unfinished work. A pending socket read lets the runtime also watch a cancellation signal and a deadline. Esc sends that signal. The worker stops waiting, drops the connection, and returns whatever bytes it already received.

**Learn more:** [Waiting does not need to occupy the terminal](../chapters/17-waiting-for-a-response.md#waiting-does-not-need-to-occupy-the-terminal) · [A future holds unfinished work](../chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work) · [`Worker`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L312)

</details>

### How does a synchronous terminal meet an async client?

**Level:** Practitioner


The terminal loop is synchronous, while the HTTP client is async (tokio and hyper). How do the two meet? Describe the threads, runtimes and channels involved.

<details>
<summary>Answer</summary>

Each Send creates a `Worker`, which spawns a dedicated OS thread named `netscope-replay`. That thread builds a current-thread tokio runtime and blocks on the exchange inside it. Three channels connect the worker to the terminal. A `tokio::sync::oneshot` carries cancellation from the terminal to the worker. A `std::sync::mpsc::sync_channel(1)` carries the single final `ReplayResult` back. For live SSE only, a `tokio::sync::watch` carries the newest cumulative snapshot, replacing the previous one instead of queueing every update.

The terminal loop waits at most 50 ms for a key, and on every pass it calls `progress` and `poll`, which use non-blocking reads. At most one worker runs at a time. Dropping a `Worker` sends cancellation and joins its thread. The terminal therefore stays synchronous and needs no async runtime of its own, and each replay gets a small runtime that is torn down when it finishes.

**Learn more:** [Why keep a separate worker at all?](../chapters/17-waiting-for-a-response.md#why-keep-a-separate-worker-at-all) · [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Why give each send its own thread and runtime?](../chapters/34-defending-the-design.md#why-give-each-send-its-own-thread-and-runtime) · [`start_mode`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L338-L404) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L421-L430)

</details>

### Races in the replay screen

**Level:** Expert


What happens when (a) a result arrives in the same loop iteration as an Esc key, (b) a result for an earlier Send arrives, (c) the worker thread panics, and (d) the user presses q while a request is in flight?

<details>
<summary>Answer</summary>

(a) Completion is accepted before a later cancellation. At the top of each loop pass the terminal loop polls the worker first and sends the cancel signal only when no result is waiting. When a key arrives, it polls again before handling the key. If the cancel flag was already set when the result is accepted, `complete` returns to the draft and keeps the result. The message depends on the outcome: "Operation stopped; o opens its retained result" for `Cancelled`, and "Finished before Esc took effect; o opens its retained result" for any other outcome.

(b) Each Send gets a `u64` ID from a counter owned by `App`, which outlives editors. `complete` and `progress` ignore a result whose ID is not the running one. When the counter would overflow, sending is disabled with "Send IDs exhausted" instead of wrapping.

(c) A panic drops the worker's end of the `sync_channel`, so `try_recv` reports `Disconnected`. `poll` then returns a prebuilt fallback result with `Failed("Worker stopped without a result")`, exactly once.

(d) `q` outside text editing sets `quit`. The loop ends, the `Worker` is dropped, and its `Drop` sends cancel and joins the thread. The terminal guard then restores the screen.

**Learn more:** [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L178-L187) · [`complete`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L161-L181) · [`fallback`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L420-L430)

</details>

### Handing over a result

**Level:** Newcomer


Why does the worker send its finished result through a channel instead of sharing a growing response body with the terminal under a lock?

<details>
<summary>Answer</summary>

Only one part of the program needs to change the body at a time. The worker owns it while receiving, then moves it into the result and sends that through a channel with room for one value. The terminal owns what it receives. Nothing is edited by two threads at once, so there is no lock to hold and no consistency rule to protect. A mutex would only be worth it if two parts really had to change related data together.

**Learn more:** [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Protect several related changes together](../chapters/24-sharing-work.md#protect-several-related-changes-together) · [`sync_channel`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L353)

</details>

### Esc pressed just after the worker finished

**Level:** Expert


A replay completes normally, but the user presses Esc at almost the same moment. Under what ordering can Esc arrive after the worker finished, and what does the screen say then?

<details>
<summary>Answer</summary>

The terminal loop polls the worker before it handles a key, so most late Esc presses arrive after the result has been accepted. The window is smaller. The key path polls and finds nothing yet. The worker then sends its Complete result. The loop handles Esc, which only sets the editor's cancel flag. On the next pass, the loop polls first, finds the result, and calls `complete`.

`complete` returns to the draft, because the user asked to leave the running view, and keeps the result, which opens with `o`. It chooses the message from the result's outcome, not from the cancel flag. No cancel signal was ever sent and the outcome is Complete, so the message is "Finished before Esc took effect; o opens its retained result". Only a `Cancelled` outcome produces "Operation stopped". The view test `esc_after_the_worker_finished_does_not_claim_a_stop` drives this order.

**Common wrong answer.** "The screen says the operation stopped." That was true of earlier versions, which chose the message from the cancel flag.

**Learn more:** [Known gaps and pitfalls](../chapters/34-defending-the-design.md#known-gaps-and-pitfalls) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`complete`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L161-L181) · [`cancellation_requested`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L185-L187)

</details>
