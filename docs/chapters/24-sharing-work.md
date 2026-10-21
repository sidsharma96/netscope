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

The counter must also stop when it runs out of numbers. Reusing an old number could let an old result look current, so checked addition detects exhaustion and prevents another send. These numbers distinguish operations within one running application. If several threads later needed to allocate them, we would have to reconsider the coordination rule.

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

The worker [`Worker`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L336), terminal loop [`run_app`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L158), and replay editor [`ReplayEditor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L31) show today's ownership and ID choices. The [decision audit](../ARCHITECTURE.md) records their constraints, alternatives, evidence, and reasons to revisit them.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Handing over a result

Why does the worker send its finished result through a channel instead of sharing a growing response body with the terminal under a lock?

<details>
<summary>Answer</summary>

Only one part of the program needs to change the body at a time. The worker owns it while receiving, then moves it into the result and sends that through a channel with room for one value. The terminal owns what it receives. Nothing is edited by two threads at once, so there is no lock to hold and no consistency rule to protect. A mutex would only be worth it if two parts really had to change related data together.

**Learn more:** [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Protect several related changes together](../chapters/24-sharing-work.md#protect-several-related-changes-together) · [`sync_channel`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L377)

</details>

### Operation numbers

Each Send gets an operation number. Why, and why is it an ordinary counter rather than an atomic one?

<details>
<summary>Answer</summary>

The number lets the terminal accept a result only for the operation it is currently waiting on, so a late result from an older send cannot replace a newer one. Only the terminal thread hands out these numbers, so no other thread competes for them and an atomic counter is unnecessary. The application owns the counter, so closing an editor does not restart the sequence. When the numbers run out, sending is refused instead of wrapping back to an old number.

**Learn more:** [Update one counter without losing an increment](../chapters/24-sharing-work.md#update-one-counter-without-losing-an-increment) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`next_send_id`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L221)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="waiting-without-freezing-the-screen"></a>

[Waiting without freezing the screen](../questions/newcomer.md#waiting-without-freezing-the-screen)

<a id="how-does-a-synchronous-terminal-meet-an-async-client"></a>

[How does a synchronous terminal meet an async client?](../questions/practitioner.md#how-does-a-synchronous-terminal-meet-an-async-client)

<a id="races-in-the-replay-screen"></a>

[Races in the replay screen](../questions/expert.md#races-in-the-replay-screen)

<a id="esc-pressed-just-after-the-worker-finished"></a>

[Esc pressed just after the worker finished](../questions/expert.md#esc-pressed-just-after-the-worker-finished)

</details>
