# Defending the design

You are showing Netscope to another developer. They select a request, change its destination, and send it. Then they ask why editing did not alter the recording, why the screen stayed responsive, and whether cancelling means the server did nothing. Explaining the buttons will not answer those questions. You need to follow the data and the work behind them.

This capstone brings the earlier lessons together. It assumes you have worked through ownership, errors, replay, and packet inspection. Its purpose is to help you explain a choice, examine an alternative, and recognise when the existing design needs to change.

## How to use this chapter

Read each situation and pause before the explanation. Try to describe the behavior in your own words. A useful answer identifies the relevant constraint, follows what happens during execution, and explains a consequence. If the question changes a requirement, reconsider the answer rather than repeating the original justification.

You do not need to memorise function names, limits, or measurements. Use the linked lessons to recover an idea you cannot yet explain. The closing exercises and the separate [expert questions](../questions/expert.md) let you practise follow-up questions. The [architecture record](../ARCHITECTURE.md) holds the detailed evidence behind the current choices.

<a id="foundations-from-first-principles"></a>
<a id="what-a-capture-file-contains"></a>
<a id="the-decision-tree"></a>
<a id="why-store-requests-in-a-vector-addressed-by-requestid"></a>

## Follow one request from its file to the screen

Someone asks why the screen cannot simply read the original JSON whenever it needs a field. That would save a conversion step, but would also make each screen responsible for interpreting the file. The list and the details could disagree about whether a missing body means empty text or unavailable content.

Netscope checks the capture during import and builds an owned session. That session keeps the request data alive. Drawing borrows it to read fields; navigation stores IDs that identify the selected requests. The importer handles the file format, while the screen works with values whose meanings have already been established.

The diagram follows responsibility for one request. Import produces a session that keeps the request data alive. The upward arrow shows the screen borrowing that data while it draws.

```text
Capture file ──read and validate──→ Session owns request data
                                           ↑
                                     borrowed access
                                           │
                                   Screen draws request
                                   Navigation remembers ID
```

A reviewer might then sort the list and ask whether request 0 has changed. It has not: sorting changes a separate list of IDs. The request records remain in their original order. The visible row number and the request's identity answer different questions.

This works because imported requests are not deleted or replaced while you browse. If deletion were added, an old numeric ID could name a different request after positions shifted. A design supporting deletion would need to preserve identity another way, for example by detecting when an ID refers to an earlier occupant of a position. More elaborate IDs solve a problem the current fixed session does not have.

The [ownership](03-storing-requests.md) and [collections](08-structs-vectors-and-ids.md) lessons explain the Rust mechanisms. [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197) shows how the complete program keeps data and display orders together.

<a id="why-compute-summaries-and-matches-once-at-import"></a>
<a id="why-is-binary-search-valid-in-redirect-and-retry-matching"></a>

## Explain a cost before calling it an improvement

The reviewer now opens a large capture and asks why Netscope calculates host summaries before displaying the first screen. The capture stays unchanged, so the same totals can be reused as the user moves around. Computing them once avoids repeating the count on each redraw.

That choice spends time before the first screen appears and memory while the session is open. Calculating a summary only when the user opens Hosts would postpone the work and avoid it for users who never visit that screen. Neither arrangement is automatically better; the useful comparison concerns startup, navigation, and the captures people actually inspect.

Matching possible redirects adds another cost. Grouping requests by destination narrows the search. Sorting each group by start time lets the search skip requests that began too early. It still has to examine later candidates that might fail another condition. If many requests fall within the time window and keep failing those conditions, the work can remain large. A limit on accepted matches does not necessarily limit rejected comparisons.

A defensible answer is therefore conditional: “We reuse summaries because this capture is fixed. If import becomes the slow part, I would measure its work and compare postponing selected analyses.” The [grouping lesson](12-grouping-requests.md) explains the tradeoff. Existing [performance observations](../LARGE_CAPTURES.md) belong beside their method and limitations, rather than becoming a universal speed claim.

<a id="why-give-each-send-its-own-thread-and-runtime"></a>
<a id="why-does-live-sse-reparse-its-whole-prefix-on-every-read"></a>

## Give the editable request and the send separate owners

The reviewer changes the captured URL to a local address. The original must remain available for comparison, so the draft owns a copy of the editable text. A shared read-only reference would let the editor inspect the original, but would not give it independently editable storage. Changing the original in place would change the evidence.

After review, a send needs a definite set of values. The worker owns those reviewed values while it opens a connection and receives a response. The terminal continues handling input. When ordinary replay ends, the worker hands the result back through a channel, which transfers a value between the two threads.

A suggested alternative is to put a growing response behind a mutex, a lock that controls access to shared data. That can work if the screen needs to read the body while the worker is changing it. Both sides would then need to agree when the lock is held, and drawing could wait if the worker held it too long. For one final response, handing ownership over avoids that shared editing arrangement.

Live event observation has a different requirement: the screen must show progress before the response ends. Netscope publishes copies of the latest accumulated state. A newer copy can replace an older one because it still includes the earlier retained events. This uses bounded storage but copies and reparses data. If much larger streams became a requirement, processing each new piece once would deserve serious consideration.

The [replay](16-preparing-a-request.md), [sharing](24-sharing-work.md), and [live-events](27-observing-live-events.md) lessons give the mechanisms and alternatives. A channel, a lock, and an independent copy each express a different relationship between the data and its users.

## Know when an atomic operation would help

Each send receives a number so the terminal can recognise its result. Someone proposes an atomic counter because the program has more than one thread. The relevant question is who changes this particular counter. Only the terminal thread allocates operation numbers, so an ordinary value with that owner is sufficient.

An atomic operation changes one shared value indivisibly. If several threads allocated numbers, a separate read followed by a write could let two callers reserve the same number. An atomic increment would address that problem. It would not also make a shared response buffer safe to edit.

Compare-and-swap is a conditional atomic operation: change the value only if it still equals the expected value. It is useful for claiming a slot that may have changed since it was inspected. It does not automatically coordinate a whole request, its buffer, and its completion state. Those relationships still need a design.

This is how to answer a question about locks versus atomics without ranking them as universally faster or safer. Identify what must stay consistent and who can change it. The [sharing lesson](24-sharing-work.md) includes small examples you can run and explains the additional rules needed when one atomic value announces changes to other memory.

<a id="how-do-cancellation-and-the-deadline-decide-the-outcome"></a>
<a id="wall-clock-time-and-monotonic-time"></a>
<a id="crashes-clocks-and-slow-peers"></a>

## Explain cancellation from both sides of the connection

A server accepts a POST and then goes silent. The user presses Esc. Locally, Netscope can stop waiting, dispose of the exchange, and retain bytes already received. It cannot erase a request the server has already processed. Cancelling is therefore not an undo operation.

The terminal sends a cancellation signal to the worker. The worker waits for cancellation, a deadline, or completion. The partial result belongs outside the exchange being cancelled, so ending that exchange does not discard the accumulated response. The final result returns with the operation number, allowing the terminal to reject a result for another send.

A deadline covers the whole operation. If every new byte restarted the allowance, a server could keep the request alive by sending a little data before each timeout. One fixed deadline prevents that extension. Inside the worker, cancellation takes priority if cancellation and completion are ready in the same check; a completion already accepted by the terminal remains complete.

There is still an operating-system boundary. A hostname lookup may continue after Netscope stops waiting for it. That background lookup cannot alter the draft or deliver a later result into the interface. “The user can cancel” and “every underlying system operation ends immediately” are different promises.

The [waiting lesson](17-waiting-for-a-response.md) explains futures, deadlines, and cleanup. [`wait_for`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L321) is the small part of the implementation that chooses between the worker's outcomes.

<a id="what-a-har-timing-records"></a>
<a id="why-keep--1-missing-zero-and-nonstandard-values-apart"></a>
<a id="content-negotiation-and-accept-encoding"></a>
<a id="why-does-replay-force-identity-encoding"></a>

## Read missing and contradictory observations honestly

A reviewer points to a blank body and asks whether the server returned nothing. The answer depends on what was recorded. An explicitly empty body records a value of length zero; an omitted body supplies no content to compare. Making both cases an empty string would discard that distinction before the screen could explain it.

A timing of `-1` also needs its field's meaning. For an optional HAR timing, it means the stage did not apply to the request. A missing field gives no value, and zero is a reported duration. The model keeps these alternatives separate. Rust's enum makes those alternatives visible to code, but the language does not decide the meaning of HAR for us.

Now consider repeated requests. The same address after an error can suggest a retry, but it can also come from a refresh or another tab. An exact comparison of recorded fields establishes that those fields agree. It does not establish the user's intention. The same caution applies when comparing a new response with a captured one: matching retained bytes does not prove that both observations were collected under identical conditions.

A strong explanation identifies the observation before offering its interpretation. The [capture](02-reading-a-capture.md), [retry](14-recognizing-repeats.md), and [comparison](18-comparing-responses.md) lessons develop those distinctions through examples.

<a id="how-tcp-numbers-its-bytes"></a>
<a id="how-http11-marks-where-a-message-ends"></a>
<a id="persistent-connections-and-connection-close"></a>
<a id="why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do"></a>
<a id="how-does-netscope-turn-tcp-segments-into-a-byte-stream"></a>
<a id="what-happens-when-two-segments-disagree-about-the-same-byte"></a>
<a id="why-refuse-transfer-encoding-together-with-content-length"></a>
<a id="how-can-a-response-be-framed-without-seeing-its-request"></a>

## Follow a packet gap into the application

Suppose one segment supplies `abc` and a later segment supplies `ghi`, with three missing byte positions between them. Removing the gap would produce `abcghi`, a sequence no recorded segment supplied. Filling it with zeros would invent different bytes. Netscope keeps the missing positions explicit and lets HTTP interpretation read only the uninterrupted beginning.

The illustration separates retained observations from the input that can safely reach the HTTP reader. The arrows represent data passed between analyses, not transmission times.

```text
Recorded positions:   a b c ? ? ? g h i
                      └───┘       └───┘
                      retained    retained
                         │
HTTP reader receives:  a b c
                       stops before the missing positions
```

If two accepted segments disagree about the same position, the analysis marks a conflict rather than choosing one copy. A receiver might have accepted one copy according to its own rules, but the inspector does not know enough to reproduce that decision. The later bytes remain inspectable even when message parsing stops.

There is a qualification worth being able to explain. The current code excludes a narrow one-byte pattern as a possible keep-alive probe before checking conflicts. That heuristic can also hide a genuine disagreement fitting the same pattern. Defending the broader design does not require defending every heuristic as certain; this is a limitation to disclose and a reason to consider a more explicit uncertain state.

Even a continuous byte sequence may lack application context. A response to HEAD carries no ordinary body, but Netscope's packet analysis does not pair requests with responses. It can therefore misread a declared length as body bytes. That is a known limitation, not proof that a complete sequence makes a complete HTTP conversation understandable. The [stream](30-putting-bytes-in-order.md) and [HTTP boundary](31-finding-http-boundaries.md) lessons explain the two stages separately.

<a id="tls-the-handshake-sni-alpn-and-certificate-checks"></a>
<a id="how-does-replay-use-tls-and-what-is-missing"></a>

## Explain what HTTPS verification actually establishes

A reviewer asks whether an encrypted response must have come from the intended server. Encryption protects the contents while they travel, but the client also needs to check whom it is talking to. TLS, the protocol used by HTTPS, combines encryption with checks based on the server's certificate.

A certificate associates a public key with names and is signed by an issuer. The public key can be shared; the corresponding private key must stay secret. Together they let the server produce a signature that a client can check without learning the private key.

The client checks a chain of issuers leading to a root it trusts, checks relevant certificate restrictions and dates, and checks that the destination name matches. The handshake also requires the server to demonstrate possession of the corresponding private key. Simply seeing a certificate would not be enough.

Two names you may meet in a review describe information exchanged during setup. SNI carries the requested hostname so a server hosting several sites can choose the relevant configuration. ALPN lets the client and server agree on an application protocol. Netscope offers HTTP/1.1 because that is the protocol its live sender speaks; an HTTP/2 label in a saved capture does not change that capability.

Netscope delegates TLS to rustls and uses a bundled set of trusted roots. It does not use the operating system's trust store or configure certificate revocation checks. Consequently, a company certificate trusted by a browser through local configuration may still fail in Netscope. A successful verification also says nothing about whether the application returned the correct book data.

The practical answer should state both the protection and its boundary. [`configuration`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L90) records the current trust and protocol choices; the [transport reference](../contracts/replay-transport.md) describes the supported exchange. The [TLS specification](https://www.rfc-editor.org/rfc/rfc8446#section-4.4) explains authentication in the handshake.

<a id="how-dns-compresses-names"></a>
<a id="can-a-malicious-dns-packet-hang-the-parser"></a>
<a id="how-are-memory-and-time-bounded-in-reassembly"></a>
<a id="threat-and-failure-model"></a>
<a id="hostile-input-files"></a>

## Give a failing input a predictable outcome

Someone asks whether Rust makes arbitrary capture files safe to read. Rust checks important memory and access rules, but the application must still decide which lengths, numbers, and combinations it accepts. A program can use valid Rust and allocate too much memory, repeat work for too long, or display misleading output.

For example, a DNS name can refer back to earlier bytes in its message. Following those references without a bound can repeat a cycle. Netscope checks referenced ranges and limits both decoded name length and pointer traversal. A pointer from a packet is only an input number; it does not receive the guarantees of a Rust reference until the program has checked the access it wants to make.

Likewise, a file-size limit does not equal a total-memory limit. Parsing may hold the original file, decoded values, and derived results at once. Reassembly may allocate positions for holes as well as observed bytes. A useful answer explains these allocations, then distinguishes a measured peak from an estimate based on limits.

Tests should target the claimed behavior. A tiny input can check that a missing byte remains missing. A generated large input can examine resource use. A controlled terminal process can check cleanup. None of those results replaces the others, and a successful build alone cannot establish them. The [testing lesson](04-quality-and-tests.md) explains how to choose an experiment that could expose a plausible mistake.

<a id="secrets-inside-capture-and-draft-files"></a>

## Preserve a saved request through failure

A user saves an edited draft over yesterday's file. If the write stops halfway, they should not lose both versions. Netscope writes a separate file beside the destination, synchronizes its contents, then renames it into place. Failures before replacement leave yesterday's destination intact.

This sequence makes ordinary write failures easier to handle, but power loss needs a more careful claim. The containing directory is not synchronized after replacement, so durability of the new directory entry is not established by synchronizing the file alone. The [saved-request lesson](20-remembering-a-request.md) explains the ownership and filesystem steps.

Saving also changes who may read the request. A plain-text file can outlive the terminal session. Netscope excludes several known credential headers, but a custom header, URL parameter, or body can still contain a secret. The save preview helps the user inspect that content; the exclusion list is not a guarantee that the file contains nothing private.

These are two different failure questions: whether the complete data survives and whether keeping that data is appropriate. A sound explanation addresses both without claiming that one file-writing technique solves the other.

<a id="known-gaps-and-pitfalls"></a>
<a id="questions-this-chapter-cannot-answer-yet"></a>

## Reconsider the design when the requirements change

Finally, imagine adding captures that grow continuously. The current once-built host summaries would become stale. Search results and redirect candidates would need updating. New data would need a clear owner while the interface reads earlier records, and the application would need a policy for how much history to retain.

Contrast that with adding another offline file format. If its importer can produce the same meanings, much of the existing inspector can remain unchanged. The difficult part is preserving differences the new format expresses: a field with a familiar name may still measure something else. The [architecture lesson](23-shaping-the-program.md) explains why boundaries should be judged by the changes they help contain.

A global singleton, one session instance accessed throughout the program, would make the current session a hidden shared dependency. It would still need rules for reading and changing its data. A factory, a function or object responsible for creating values, addresses construction rather than how many sessions may exist. The useful starting point is the lifetime and responsibility of each session, not choosing a pattern name first.

The capstone is complete when you can follow a changed assumption through its consequences. You may conclude that the current design fits, that an alternative deserves an experiment, or that a known weakness needs repair. The [implementation limitations](../LIMITATIONS.md) and [architecture record](../ARCHITECTURE.md) keep those open questions available for further work.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### A capture that keeps growing

Netscope now has to accept new requests while the user inspects older ones. Which assumptions behind its stored summaries, search results, and ownership would you revisit?

<details>
<summary>Answer</summary>

The current session is fixed after import, so derived counts and ID lists can be reused. Arriving requests would make those values stale. The design needs a way to update them, keep IDs stable, coordinate writes with readers, and bound retained history. One possible arrangement is an owner that accepts new records and publishes snapshots to the interface. Another is protected shared state. The choice depends on how quickly updates must appear and how much copying or waiting is acceptable. Adding a lock alone does not define those rules.

**Learn more:** [Capstone discussion](../chapters/34-defending-the-design.md#reconsider-the-design-when-the-requirements-change)

</details>

### Cancelling after a server accepts a request

A POST reached the server, but its response has not arrived. The user cancels. Explain what the client can stop, what data it can retain, and what it cannot promise about the server.

<details>
<summary>Answer</summary>

The client can stop waiting and dispose of the exchange. Response bytes already retained can survive because their owner is outside the cancelled exchange. The server may already have processed the POST and can continue working after the client closes its connection. Cancellation therefore cannot promise that no change occurred remotely. A result still belongs to its original operation number; it must not overwrite a later send.

**Learn more:** [Capstone discussion](../chapters/34-defending-the-design.md#explain-cancellation-from-both-sides-of-the-connection)

</details>

### A stream that looks complete

A packet analysis has no missing byte positions. Is that enough to call its displayed HTTP messages a correct conversation? Give a case where more context matters.

<details>
<summary>Answer</summary>

No. A continuous sequence removes one obstacle to reading, but does not supply the request context for a response. A HEAD response has no ordinary body even when Content-Length describes a representation. Netscope analyses directions separately, so it can treat that length as body bytes and consume the beginning of a later response. Capture completeness and correct application interpretation are separate questions. The keep-alive exclusion also needs disclosure when claiming that all disagreements were detected.

**Learn more:** [Capstone discussion](../chapters/34-defending-the-design.md#follow-a-packet-gap-into-the-application)

</details>
