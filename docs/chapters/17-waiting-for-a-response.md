# How can we wait for a server and still respond to a key?

You review a request to a server on your own machine and press Send. The server receives it, but sends nothing back. You press Esc. The application should notice that key even though the response has not arrived.

This is a different problem from reading a completed capture. A capture gives us values we can examine immediately. A live request has intervals when there is no answer yet. During those intervals, the application must keep track of unfinished work, remain able to accept input, and decide what to keep if waiting ends early.

## Waiting does not need to occupy the terminal

A [thread](../GLOSSARY.md#thread) is a sequence of instructions that the operating system can schedule independently of another sequence. Netscope keeps terminal input on one thread and gives a send operation its own worker thread. Waiting for a server therefore does not require the terminal thread to stop reading keys.

Moving work to a thread is only part of the answer. A worker that waits indefinitely for a response would still need a way to notice cancellation. Our worker uses asynchronous I/O: an operation can report that it cannot progress yet, allowing other pending work, such as a cancellation signal or a [deadline](../GLOSSARY.md#deadline), to be checked.

The following timeline is illustrative. Columns represent responsibilities, not processor cores or measured durations. Read downward to follow events. The arrows show information passing between the terminal and the worker.

```text
Terminal                         Worker                       Server
Review → Send ──request values──→ Open connection
Read keys                        Send GET ───────────────────→ Receive request
Draw waiting view                Wait for response             Work continues
Esc ───────────cancel signal───→ Stop exchange
                                 Close connection
Draft ←────────result────────── Keep received data
```

The terminal does not wait inside the operation that reads the response. It periodically checks for a result and continues reading input. The worker does not keep a reference to an editable text field; it owns the reviewed values for this send. Returning to a draft cannot silently change an operation already in progress.

## A future holds unfinished work

Rust calls a value representing an asynchronous computation a [future](../GLOSSARY.md#future). A future can contain the data and progress needed to continue later. Calling an `async fn` creates such a value. It does not, by itself, run the function's body.

This small example makes that distinction visible. It creates work and then discards it before anything drives it forward.

```rust
async fn report() {
    println!("response received");
}

let unfinished = report();
drop(unfinished);
```

This prints nothing. `async` changes the kind of function call: the call produces a future whose eventual output would be `()`, Rust's value for “no useful returned value.” The standard `drop` function consumes a value and destroys it now. Here it destroys a future whose body has never started.

A future makes progress when it is polled. Polling means asking it to do the work it can do now. It answers either Ready, with its result, or Pending, meaning it needs another opportunity later. A network future can register a waker before returning Pending. A waker is a way for the waiting infrastructure to notify the task when another poll may make progress; it does not contain the response itself.

We do not write a loop that repeatedly asks an idle socket whether a byte has arrived. A [runtime](../GLOSSARY.md#runtime) coordinates these polls with notifications from the operating system and with timers. Netscope uses the [Tokio](../GLOSSARY.md#tokio) runtime in the worker. A pending socket read lets that runtime wait efficiently while still being able to wake for cancellation or a deadline.

## Awaiting keeps a place to continue

The `.await` expression lets one future wait for another. If the inner future is ready, execution continues with its output. If it is pending, the enclosing future can also return Pending, keeping enough state to resume at this point later. Values needed after the wait remain part of that state.

Here is a complete, deliberately small example. There is no server in this example: the inner future is immediately ready, so the wait need not suspend at all.

```rust
async fn received_length() -> usize {
    5
}

async fn describe_response() -> String {
    let length = received_length().await;
    format!("received {length} bytes")
}

fn main() -> Result<(), std::io::Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let description = runtime.block_on(describe_response());
    assert_eq!(description, "received 5 bytes");
    Ok(())
}
```

`usize` is an unsigned integer type used for lengths and indexes. The arrow after a function's parameter list names its output type. Inside the asynchronous function, `.await` turns the inner future's eventual output into the number assigned to `length`. The `format!` macro builds a String from the text and the value in braces.

The lower half supplies the machinery that runs the example. The double colons in `tokio::runtime::Builder` follow names through a library and its modules to a type. `new_current_thread()` begins a runtime configuration that polls tasks on the calling thread. `enable_all()` enables its available I/O and timer support, and `build()` constructs it. Building can fail, so `?` returns that error from `main`, as we saw in the error-handling chapter.

`block_on` drives a future to completion and returns its output. It occupies the calling thread until that future finishes. This is why Netscope calls it on the worker thread rather than the terminal thread. Inside that runtime, asynchronous waits still allow the response, cancellation, and timer to compete for progress. The example checks the resulting text with `assert_eq!`; changing the inner return value to 8 would make that assertion fail.

Adding `async` does not automatically create a thread or make every instruction interruptible. A long calculation that never yields can occupy the thread polling it. Asynchronous code stays responsive when its work between suspension points is bounded and its waits cooperate with the runtime.

## Keep one deadline for the whole request

Suppose a server sends a few bytes just before each timeout would expire. If every received chunk started a fresh timeout, that server could keep the request alive indefinitely. Netscope instead records one deadline when Send is accepted. Looking up the host, opening the connection, verifying HTTPS, and receiving the body all spend time from the same allowance.

An operation ends with one accepted outcome. Completion means the HTTP body ended normally; a status such as 404 can still be a complete response. Timeout, cancellation, size limit, and a transport failure explain different reasons why the operation stopped. A transport failure means the connection or HTTP processing failed, rather than that the server chose an error status.

The body limit introduces another boundary. Receiving exactly the allowed number of bytes does not always tell us whether the body is complete. If the HTTP framing says the body has ended, it is complete. If another byte arrives, it is size-limited. If neither fact becomes available before the deadline, it is timed out with the bytes already received.

That distinction matters when examining partial data. “We retained these bytes” and “this is the whole response” are separate claims.

## Why keep a separate worker at all?

We could make the entire terminal program asynchronous and run input, drawing, and network operations on one long-lived runtime. That could avoid creating a thread and runtime for each send. It would also move cancellation, terminal input, and shutdown into a different coordination scheme. Asynchronous I/O does not by itself require an extra operating-system thread.

The current design keeps the existing terminal loop in charge of input and gives each send a bounded lifetime elsewhere. This is a choice about separation of responsibilities, not evidence that one thread per operation is always faster. It has setup cost, and waiting for worker cleanup on exit can still occupy the terminal thread briefly. Many concurrent sends would be a reason to reconsider it. The [sharing chapter](24-sharing-work.md) compares handing over a result with sharing a mutable buffer.

## Why let a library interpret HTTP?

A response can arrive in pieces that do not line up with a header or body boundary. Handling those pieces correctly is separate from deciding how much data Netscope should retain. [Hyper](../GLOSSARY.md#hyper), our HTTP library, interprets the protocol; the application sets its limits and decides what the result means. Rustls handles the encrypted connection and certificate checks for HTTPS, while Tokio coordinates waiting.

Writing our own HTTP parser would add another large correctness problem to a learning inspector. A higher-level client library could do more of the connection management for us, but we would need to check its redirect, retry, cookie, proxy, and decompression behaviour against the reviewed-send promise. Such libraries can be configured; they are not inherently unsuitable. Our current lower-level connection API makes the one-connection lifetime explicit, at the cost of more code to manage it.

The resulting replay deliberately differs from a browser: it sends one reviewed operation using HTTP/1.1, does not follow a redirect or retry an HTTP request, and keeps the received body without automatic decompression. Seeing HTTP/2 in an imported recording does not change that replay capability. The [transport decision](../contracts/replay-transport.md) states the exact limits, and the [architecture audit](../ARCHITECTURE.md#http-stack-and-replay-scope) records the alternative assessment without claiming a client-library benchmark.

## Cancellation follows ownership

Imagine that the worker has copied three response bytes into its result when you cancel. We want to stop waiting without throwing those three bytes away. The value collecting the result therefore lives outside the future performing the exchange. The exchange borrows it while receiving data. Dropping that future ends the borrow, leaving the collected data available to return.

This drawing shows the ownership relationship during a send. The dashed relationship is temporary access; the solid arrows indicate responsibility for keeping a value alive.

```text
Worker ──owns──→ Partial result
                  status, headers, retained bytes
   │                       ↑
   └──owns──→ Exchange ┄borrows┘
                │
                └──owns──→ Connection and socket

Cancel: drop Exchange → drop Connection → close local socket
Keep:   Partial result → deliver one final result to the terminal
```

Dropping a future is a local action. It does not rewind instructions already run. If the server received the GET, it may continue working after we close our connection. Even local cleanup has a boundary: an operating-system hostname lookup can continue in the background after we stop waiting for it. It has no access to the draft and cannot deliver a later result into the interface.

The worker returns its result through a [channel](../GLOSSARY.md#channel), a communication path that transfers a value between threads. This channel has room for one result. The body stays with the worker while receiving, then moves with that result; it is not repeatedly copied into a queue of screen updates.

## A late result must not change a newer decision

Two events can occur close together: a response finishes and you press Esc. We need an explicit rule, rather than whichever label happens to be drawn last. If the terminal has already accepted completion, a later cancellation does not turn it into a cancelled request. Inside the worker, if cancellation and completion become ready in the same check, cancellation takes priority.

Each accepted send also receives an operation ID, a number that increases for each operation. The terminal accepts a result only while waiting for that ID, and accepts a terminal result only once. An old event therefore cannot replace the outcome of a newer operation. This identity is different from the source request ID: several new operations can all begin from the same recorded request.

While cancellation is pending, editing remains suspended. Once the worker has disposed of the connection and returned, Esc's cancellation path returns to the draft. The retained result is still available through o. It remains a new request's result; it never replaces the captured response.

## Predict the result before trying it

Consider a server that sends headers and three bytes, then stays silent. Predict what Netscope should retain after the deadline. Now consider a server that sends the entire body and finishes before you press Esc. Explain why the second response should remain complete.

For a local experiment, follow the [replay instructions](../DEVELOPMENT.md#try-a-local-send). Keep the default empty header selection, review the local destination, and send. The result should identify a new request, while returning to the inspector should show the original recording. The transport implementation [`wait_for`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L321) and terminal loop [`run_app`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L158) show how these ownership and waiting rules fit together. Exact limits and test boundaries live in the [transport decision](../contracts/replay-transport.md) and [replay contract](../contracts/m3-single-request-replay.md).

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Waiting without freezing the screen

**Level:** Newcomer


A server accepts your request and then sends nothing. How can Netscope still react when you press Esc?

<details>
<summary>Answer</summary>

Each Send runs on its own worker thread, so the terminal thread keeps reading keys. Inside the worker, a Tokio runtime drives the exchange as a future, a value representing unfinished work. A pending socket read lets the runtime also watch a cancellation signal and a deadline. Esc sends that signal. The worker stops waiting, drops the connection, and returns whatever bytes it already received.

**Learn more:** [Waiting does not need to occupy the terminal](../chapters/17-waiting-for-a-response.md#waiting-does-not-need-to-occupy-the-terminal) · [A future holds unfinished work](../chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work) · [`Worker`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L336)

</details>

### How does a synchronous terminal meet an async client?

**Level:** Practitioner


The terminal loop is synchronous, while the HTTP client is async (tokio and hyper). How do the two meet? Describe the threads, runtimes and channels involved.

<details>
<summary>Answer</summary>

Each Send creates a `Worker`, which spawns a dedicated OS thread named `netscope-replay`. That thread builds a current-thread tokio runtime and blocks on the exchange inside it. Three channels connect the worker to the terminal. A `tokio::sync::oneshot` carries cancellation from the terminal to the worker. A `std::sync::mpsc::sync_channel(1)` carries the single final `ReplayResult` back. For live SSE only, a `tokio::sync::watch` carries the newest cumulative snapshot, replacing the previous one instead of queueing every update.

The terminal loop waits at most 50 ms for a key, and on every pass it calls `progress` and `poll`, which use non-blocking reads. At most one worker runs at a time. Dropping a `Worker` sends cancellation and joins its thread. The terminal therefore stays synchronous and needs no async runtime of its own, and each replay gets a small runtime that is torn down when it finishes.

**Learn more:** [Why keep a separate worker at all?](../chapters/17-waiting-for-a-response.md#why-keep-a-separate-worker-at-all) · [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Why give each send its own thread and runtime?](../chapters/34-defending-the-design.md#why-give-each-send-its-own-thread-and-runtime) · [`start_mode`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L362-L428) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L445-L454)

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

**Learn more:** [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L178-L187) · [`complete`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L161-L181) · [`fallback`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L444-L454)

</details>

### Handing over a result

**Level:** Newcomer


Why does the worker send its finished result through a channel instead of sharing a growing response body with the terminal under a lock?

<details>
<summary>Answer</summary>

Only one part of the program needs to change the body at a time. The worker owns it while receiving, then moves it into the result and sends that through a channel with room for one value. The terminal owns what it receives. Nothing is edited by two threads at once, so there is no lock to hold and no consistency rule to protect. A mutex would only be worth it if two parts really had to change related data together.

**Learn more:** [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Protect several related changes together](../chapters/24-sharing-work.md#protect-several-related-changes-together) · [`sync_channel`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L377)

</details>

### How do the deadline and cancellation work?

**Level:** Practitioner


When does the replay deadline start, how long is it for a normal replay and for live SSE, does progress extend it, and what makes Esc take effect? In what order are cancellation, the deadline and completion checked?

<details>
<summary>Answer</summary>

The deadline starts at the instant the terminal accepted Send, before the worker thread exists, so thread setup counts against it. It is one absolute deadline covering connection, TLS, headers and the whole body: 10 seconds for a normal replay and 30 seconds for live SSE. Progress never extends it, and tests check that a slow stream of bytes does not restart the clock.

The worker runs the exchange inside a `tokio::select!` marked `biased`, which checks the cancellation channel first, then `sleep_until(deadline)`, then the work. When cancellation or the deadline wins, the losing futures are dropped, and dropping them closes the connection and its socket. Because the deadline is checked before the work, an already expired deadline never opens a connection. Bytes copied into the result before the stop are kept and reported with the `Cancelled` or `TimedOut` outcome.

**Learn more:** [Keep one deadline for the whole request](../chapters/17-waiting-for-a-response.md#keep-one-deadline-for-the-whole-request) · [Cancellation follows ownership](../chapters/17-waiting-for-a-response.md#cancellation-follows-ownership) · [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [`wait_for`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L321-L332) · [`TIME_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L30) · [`started`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L467)

</details>
