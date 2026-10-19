# Expert questions

These questions ask what breaks Netscope and what happens next. They cover failure modes, traps where the obvious answer is wrong, worst-case cost, the threat model, and changes you would make. Each answer is hidden until you open it. Try to answer aloud first, then compare, and follow the links to the section and the code that settle the point.

### A reused port, a repeated SYN, and a late ACK

**Level:** Expert

<!-- chapters: 30-putting-bytes-in-order -->

A client reuses the same source port for a second connection to the same server. Separately, a SYN is retransmitted. Separately, a pure ACK arrives after the FIN. Which directed epoch does each of these packets join?

<details>
<summary>Answer</summary>

Netscope keeps one active epoch per direction and decides on each packet in file order. A SYN starts a new epoch when the active one has no SYN, when its sequence number differs from the active anchor (a different ISN), or when the active epoch was already closed by a FIN or RST. Port reuse with a new SYN therefore starts a new epoch, and the second connection never merges into the first.

A retransmitted SYN with the same ISN on an open epoch matches the anchor, so it joins the existing epoch. A pure ACK after the FIN is not a SYN, so it joins the active epoch even though that epoch is closed. It adds no byte positions, because a segment with no payload and no FIN or RST is skipped when bytes are placed.

The weak case is reuse where the new connection's SYN was not captured. Its first packet is not a SYN, so it joins the old closed epoch, and its bytes are placed relative to the old anchor.

**Learn more:** [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`is_none_or`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L99-L101) · [`tuple_reuse_resets_directions_and_budget_does_not_merge_new_epochs_into_old`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L36)

</details>

### A capture that starts mid-connection

**Level:** Expert

<!-- chapters: 30-putting-bytes-in-order -->

A capture has no SYN for a direction, and a segment with a lower sequence number appears later in the file than the first one. What do `first_sequence`, the contiguous prefix, and the connection-start claim look like? How does this change when a SYN is present and a segment lies before it?

<details>
<summary>Answer</summary>

Without a SYN, the anchor is the sequence number of the first packet recorded for that direction. A later record with a lower sequence number gets a negative offset, so the epoch's span grows backwards to include it. `first_sequence` is the anchor plus that most negative offset, which makes it the lowest retained sequence number, not a claimed connection start. `syn_observed` is false, and the stream view says "Start unknown: capture has no SYN for this directed epoch". If a hole lies at the start of the span, the contiguous prefix is empty.

`observed_end` requires a SYN, so a close-delimited response in such an epoch is never treated as framed.

With a SYN, position 0 is ISN + 1, the first data byte. A segment whose offset is negative cannot belong to this connection's data, so it is skipped and counted in `skipped_segments`. A skipped segment also makes `observed_end` false.

**Learn more:** [A direction needs a beginning, and sometimes we do not have it](../chapters/30-putting-bytes-in-order.md#a-direction-needs-a-beginning-and-sometimes-we-do-not-have-it) · [`first_sequence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L239) · [`without_syn_out_of_order_prefix_is_retained_but_not_claimed_as_connection_start`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/reassembly.rs#L78)

</details>

### A capture taken with a 96-byte snap length

**Level:** Expert

<!-- chapters: 29-reading-packet-bytes, 30-putting-bytes-in-order, 31-finding-http-boundaries -->

A capture was recorded with a snap length of 96 bytes, so every TCP payload is cut. What do the reassembled byte states look like, what does `contiguous_prefix` return, and what does the HTTP framing inspector see?

<details>
<summary>Answer</summary>

Each record keeps 96 bytes. After a 14-byte Ethernet header, a 20-byte IPv4 header, and a 20 to 32-byte TCP header, between about 30 and 42 payload bytes remain. The importer adds the layer note "Capture cut: fewer bytes retained than original packet".

The segment's declared payload comes from the IP length field, not from the captured length, so reassembly reserves the full sequence range the sender used. Only the retained bytes are written. The rest of each segment's range stays Missing. The stream becomes a run of Observed bytes, then a run of Missing positions, then Observed again for the next segment, and so on.

`contiguous_prefix` stops at the first Missing position, inside the first data segment. The HTTP inspector therefore sees a few dozen bytes. It usually stops with "No complete HTTP/1.1 start line in prefix" or a header-line error. It never joins the retained fragments across the holes, and a Missing position is never read as a zero byte.

**Learn more:** [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`retained`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L175-L179) · [`declared_payload`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L372) · [`contiguous_prefix`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L53-L60)

</details>

### Inputs the wire HTTP parser cannot frame

**Level:** Expert

<!-- chapters: 31-finding-http-boundaries -->

Give concrete inputs on which the wire HTTP framing inspector stops or misframes. Consider a response to HEAD, `101 Switching Protocols`, a PUT request, an HTTP/1.0 response, two identical `Content-Length` headers, and `Transfer-Encoding: gzip, chunked`.

<details>
<summary>Answer</summary>

The inspector reads each direction alone and never pairs a response with its request. That makes HEAD the most important case. A response to HEAD that carries `Content-Length: 1234` is framed as if 1234 body bytes follow. On a reused connection the next response's bytes are taken as that body, or the message stays unframed while it waits.

A `101` response is labelled "101 changes protocol; later HTTP framing unavailable", and inspection stops, because the later bytes belong to another protocol. A PUT request, or any method other than GET and POST, stops with "No supported GET/POST or HTTP/1.1 response start at prefix". An `HTTP/1.0` status line fails the same check, because only `HTTP/1.1` is accepted as a response start.

Two identical `Content-Length: 5` headers give "Content-Length outside supported single decimal subset". RFC 9110 allows repeated identical values, so this is stricter than the standard. `Transfer-Encoding: gzip, chunked` gives "Transfer coding outside supported single chunked subset". Obsolete line folding and whitespace before the colon fail as invalid header names.

**Learn more:** [How can a response be framed without seeing its request?](../chapters/34-defending-the-design.md#how-can-a-response-be-framed-without-seeing-its-request) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186) · [`lengths`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L325-L329) · [`transfer`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L248-L251)

</details>

### Races in the replay screen

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response, 24-sharing-work -->

What happens when (a) a result arrives in the same loop iteration as an Esc key, (b) a result for an earlier Send arrives, (c) the worker thread panics, and (d) the user presses q while a request is in flight?

<details>
<summary>Answer</summary>

(a) Completion is accepted before a later cancellation. At the top of each loop pass the terminal loop polls the worker first and sends the cancel signal only when no result is waiting. When a key arrives, it polls again before handling the key. If the cancel flag was already set when the result is accepted, `complete` returns to the draft and keeps the result. The message depends on the outcome: "Operation stopped; o opens its retained result" for `Cancelled`, and "Finished before Esc took effect; o opens its retained result" for any other outcome.

(b) Each Send gets a `u64` ID from a counter owned by `App`, which outlives editors. `complete` and `progress` ignore a result whose ID is not the running one. When the counter would overflow, sending is disabled with "Send IDs exhausted" instead of wrapping.

(c) A panic drops the worker's end of the `sync_channel`, so `try_recv` reports `Disconnected`. `poll` then returns a prebuilt fallback result with `Failed("Worker stopped without a result")`, exactly once.

(d) `q` outside text editing sets `quit`. The loop ends, the `Worker` is dropped, and its `Drop` sends cancel and joins the thread. The terminal guard then restores the screen.

**Learn more:** [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L178-L187) · [`complete`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L174) · [`fallback`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L444-L454)

</details>

### Cancelling while DNS is still resolving

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response -->

A user presses Esc while the replay's hostname is still being resolved. Does cancellation wait for DNS? Can quitting hang? What is left running?

<details>
<summary>Answer</summary>

`TcpStream::connect((host, port))` resolves the name through Tokio, which runs the blocking operating-system lookup (`getaddrinfo`) on a helper thread from its blocking pool. That call cannot be interrupted.

Cancellation does not wait for it. The biased `select!` in `wait_for` sees the cancel signal, returns `Cancelled` at once, and drops the connect future. After `block_on` returns, the worker calls `shutdown_timeout(Duration::ZERO)` on its runtime. A zero timeout means the runtime does not wait for blocking-pool threads. The worker thread then sends its result and exits, so the `join` in `Worker`'s `Drop` returns promptly. Quitting does not hang on DNS.

One thing is left behind: the helper thread keeps waiting for the operating system to answer the lookup, and its answer is then discarded. It holds no UI state. The code comment beside the call records this choice.

**Learn more:** [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [Cancellation follows ownership](../chapters/17-waiting-for-a-response.md#cancellation-follows-ownership) · [`shutdown_timeout`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L412-L413) · [`wait_for`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L321-L332)

</details>

### A company CA, a proxy, and an HTTP/2-only server

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response -->

A user replays a request to an internal HTTPS service whose certificate is signed by the company's own CA. The network requires an HTTP proxy, and the server speaks only HTTP/2. What happens in each case, and why?

<details>
<summary>Answer</summary>

The TLS configuration trusts only the Mozilla root set compiled in from `webpki-roots`. It does not read the operating system's trust store, and there is no option to add a CA or skip verification. A company-signed certificate therefore fails with "TLS certificate issuer is not trusted". Hostname verification is also always on.

The client connects directly to the URL's host with `TcpStream::connect`. It does not read `HTTP_PROXY` or `HTTPS_PROXY`, so on a network that blocks direct connections the attempt fails with "Could not connect to destination" or meets the 10-second deadline.

ALPN offers only `http/1.1`, and the hyper connection is HTTP/1.1 only. A server that refuses HTTP/1.1 in ALPN fails the handshake. A server that ignores ALPN receives an HTTP/1.1 request it cannot parse. Netscope has no HTTP/2 or HTTP/3 replay.

**Common wrong answer.** "It uses the system trust store and honours `HTTPS_PROXY`, like curl or a browser." Neither is true here. Both are disclosed gaps, not configuration choices the user can change.

**Learn more:** [How does replay use TLS, and what is missing?](../chapters/34-defending-the-design.md#how-does-replay-use-tls-and-what-is-missing) · [`configuration`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L90-L97) · [`TcpStream::connect`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L231) · [`tls_verifies_trust_and_hostname`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/tests.rs#L223)

</details>

### The worst case for redirect and retry matching

**Level:** Expert

<!-- chapters: 13-following-redirects, 14-recognizing-repeats -->

What is the worst-case time complexity of retry and redirect analysis for a capture with n entries? Which input triggers it, and what bounds it in practice?

<details>
<summary>Answer</summary>

Building each index is O(n log n). For each source, `partition_point` finds the first possible candidate in O(log k), and a forward scan follows. The scan stops at the 30-second window or after 20 accepted candidates.

For retries, a candidate with the same URL and method but a different header, cookie, or query list is skipped with `continue` and does not count toward the cap of 20. Take n GET requests to one URL, all inside 30 seconds, all with a retryable status such as 503, each with a changing request-ID header. Every source scans every later request and accepts none, so the work is O(n²) list comparisons. Redirect matching has the same shape, but only a method mismatch is skipped without counting, so it reaches O(n²) only when many same-URL requests use a method the status does not allow.

Both analyses run once, eagerly, inside `Session::new`. The import limit of 10,000 entries bounds the adversarial case at roughly 5 × 10^7 comparisons.

**Learn more:** [Why is binary search valid in redirect and retry matching?](../chapters/34-defending-the-design.md#why-is-binary-search-valid-in-redirect-and-retry-matching) · [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [`partition_point`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L68) · [`headers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L80-L85) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L235-L236)

</details>

### What a saved draft keeps, leaks, and survives

**Level:** Expert

<!-- chapters: 20-remembering-a-request -->

What does a saved draft contain, what does Netscope strip, what can still leak, and how crash-safe is the write? What happens when the destination is a symlink or a stale temporary file exists?

<details>
<summary>Answer</summary>

The file is pretty-printed JSON with `version: 1`, a name, the URL, method, body format, body, and headers. Decoding rejects unknown fields and files over 512 KiB, and every load and save runs the full draft review.

Nine header names are dropped on save: Authorization, Proxy-Authorization, Cookie, Cookie2, Set-Cookie, X-Api-Key, Api-Key, X-Auth-Token, and X-Access-Token. A file that contains one of them is rejected on load, not silently accepted. The leak surface remains: the URL is saved verbatim, including query-string tokens and the fragment, and so are the body and any secret header outside the list, such as `X-CSRF-Token`.

The write refuses a destination that is not a regular file, including a symlink. It creates `<name>.netscope-tmp` with `create_new` and mode 0600, writes it, calls `sync_all`, and renames it over the destination. A failure removes the temporary file. A leftover temporary file makes `create_new` fail, so the save is refused. Two weaknesses remain: the parent directory is not synced after the rename, so a power loss can undo it, and the symlink check and the rename are separate steps.

**Learn more:** [Replacing a file has a vulnerable middle](../chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle) · [Secrets inside capture and draft files](../chapters/34-defending-the-design.md#secrets-inside-capture-and-draft-files) · [`sensitive_saved_header`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L27) · [`replace`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/saved_file.rs#L44-L73)

</details>

### How live SSE stamps, decodes, and publishes events

**Level:** Expert

<!-- chapters: 27-observing-live-events, 25-reading-event-streams -->

During a live SSE observation, how are per-event times produced? What happens when a UTF-8 character is split across two reads, or when invalid bytes arrive? How does the screen see progress, and what does the parsing approach cost?

<details>
<summary>Answer</summary>

Before the body is read, the response must have status 200, exactly one `text/event-stream` Content-Type, and no content encoding other than identity. Each body frame is then appended to a buffer capped at 64 KiB.

`observe` finds the longest valid UTF-8 prefix. If the error is an incomplete final character, it waits for more bytes. If the bytes are invalid, it publishes the valid prefix and stops with an error. A stream that ends inside a character is also an error.

The whole valid prefix is parsed again with `parse_sse` on every frame. `observed.resize(count, elapsed)` gives every event completed by this frame the time since Send at which the frame arrived. Several events in one frame share a time, and the time is arrival here, not the server's send time. The 100th event or the 64 KiB limit ends the observation without waiting for the server to close.

Progress goes through a `watch` channel with `send_replace`, which keeps only the newest snapshot. The screen ignores snapshots for a stale Send ID. Reparsing is O(n) per frame, so the total is quadratic in the number of frames, bounded by 64 KiB.

**Learn more:** [Bytes can stop in the middle of a character](../chapters/27-observing-live-events.md#bytes-can-stop-in-the-middle-of-a-character) · [Why does live SSE reparse its whole prefix on every read?](../chapters/34-defending-the-design.md#why-does-live-sse-reparse-its-whole-prefix-on-every-read) · [`observe`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L51-L85) · [`validate_response`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L108)

</details>

### A wrong Sec-WebSocket-Accept value

**Level:** Expert

<!-- chapters: 26-switching-protocols -->

A HAR contains a 101 response whose `Sec-WebSocket-Accept` value is wrong for the request's `Sec-WebSocket-Key`. Every other header is shaped correctly. What summary does Netscope show?

<details>
<summary>Answer</summary>

It shows "Recorded 101 with expected classic upgrade header shape".

`websocket_evidence` checks only that exactly one non-empty `Sec-WebSocket-Accept` header is present. It does not compute the expected value, which is the base64 of SHA-1 over the key followed by a fixed GUID. A wrong value passes. The other checks are about shape: HTTP/1.1 on both sides, GET, Upgrade containing `websocket` and Connection containing `Upgrade` in both directions, one non-empty key, a version of exactly `13`, and a selected subprotocol that is one exact offered value.

The function's documentation says that challenge digests, extensions, origin policy, and later frames are not validated. The summary describes recorded header shape. It does not claim the handshake was valid.

**Common wrong answer.** "Netscope recomputes the SHA-1 accept value and flags the mismatch." It does not hash anything, so this case looks the same as a correct handshake.

**Learn more:** [An agreement has two sides](../chapters/26-switching-protocols.md#an-agreement-has-two-sides) · [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L26-L28) · [`accepts`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L89-L92)

</details>

### Which copy wins when segments overlap

**Level:** Expert

<!-- chapters: 30-putting-bytes-in-order, 33-interpreting-a-trace -->

Operating systems and tools such as Wireshark pick "first copy wins" or "last copy wins" for overlapping TCP segments. Which policy does Netscope use? Does it sort segments by capture timestamp before placing them? Defend the choice.

<details>
<summary>Answer</summary>

Neither policy. The first observed value fills a position. A later copy that agrees increments `repeated_bytes`. A later copy that disagrees turns the position into `Conflict`, and a conflict is permanent. The contiguous prefix stops at a conflict, so no HTTP analysis uses a disputed byte.

Segments are processed in record order, and capture timestamps are never read during reassembly. Positions come from sequence numbers only. Because of that, the state of each position and every agreed value are the same whatever order the records are in. Record order does change three things: the anchor of an epoch without a SYN, which segments survive the span budgets, and the repeat count at a conflicted position.

The defence is that a capture cannot tell which copy the receiver accepted. Checksums are not verified, receiver state is unknown, and real stacks disagree. An intrusion detector that picks a different copy from the target can be evaded. Marking the conflict keeps the ambiguity visible instead of hiding it behind a guess.

**Common wrong answer.** "Netscope sorts by timestamp and keeps the first copy, as most stacks do." It neither sorts by time nor picks a winner.

**Learn more:** [What happens when two segments disagree about the same byte?](../chapters/34-defending-the-design.md#what-happens-when-two-segments-disagree-about-the-same-byte) · [`ByteState::Conflict`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L223-L230)

</details>

### Why hyper instead of reqwest

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response -->

Replay uses `hyper::client::conn::http1` directly instead of a full client such as reqwest. Which behaviours does that avoid, and how does the code show that a redirect response is not followed?

<details>
<summary>Answer</summary>

Netscope builds the TCP or TLS stream itself and hands it to `http1::Builder`. That gives one connection per Send, dropped with the exchange, and no pool. It gives no automatic redirect following and no automatic decompression; the client always sends `Accept-Encoding: identity` and reports an encoded body as encoded. It reads no proxy variables, keeps no cookie jar, and adds no headers beyond the managed ones: Host, Accept-Encoding, and for POST, Content-Type and Content-Length. The builder also limits responses to 100 headers and 32 KiB of header data.

A full client applies its own policies by default, such as following redirects, pooling connections, reading proxy settings, and adding default headers. Each of those would make the sent request differ from the reviewed one. The design goal is that what you reviewed is what is sent, once.

The proof is a test. `redirects_and_disconnects_never_open_a_second_connection` serves a 302 with a Location header on a local listener, checks that the result is Complete, and then checks that the listener has no second connection waiting.

**Learn more:** [Why let a library interpret HTTP?](../chapters/17-waiting-for-a-response.md#why-let-a-library-interpret-http) · [Replay transport decision](../contracts/replay-transport.md) · [`Builder`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L261-L265) · [`redirects_and_disconnects_never_open_a_second_connection`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/tests.rs#L487)

</details>

### What "Stored bytes match" does not prove

**Level:** Expert

<!-- chapters: 18-comparing-responses -->

After a replay, how is the captured body compared with the replayed body? When does the comparison refuse to answer, and what does "Stored bytes match" fail to prove?

<details>
<summary>Answer</summary>

`compare_body` refuses in three cases. The replay has no status because no headers arrived. The captured body was not recorded or could not be decoded. Or either side names a Content-Encoding other than identity. The last case matters most: a browser usually stores the decoded text in the HAR, while replay keeps the encoded bytes, so there is no common representation to compare.

Otherwise it finds the first differing byte across the shared prefix. If the replay did not complete (cancelled, timed out, size-limited, or failed), the answer is about a prefix only: a difference offset, a matching prefix length, or no overlap. If it completed, the answer is a differing byte, one body empty and the other not, different lengths with a matching shared prefix, or equal. The captured side is the HAR text as UTF-8 bytes, or the decoded base64 bytes.

Equality does not prove the capture was complete, since HAR bodies are often cut or omitted. It does not prove the server state was the same, or that the browser's text matches the bytes that crossed the wire, since the browser may have changed the character encoding. The label says "capture completeness unproven".

**Learn more:** [Bytes need a common meaning](../chapters/18-comparing-responses.md#bytes-need-a-common-meaning) · [`compare_body`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/comparison.rs#L60-L91) · [`encoded`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/comparison.rs#L55-L58)

</details>

### Hostile text and the terminal

**Level:** Expert

<!-- chapters: 11-drawing-and-navigation -->

Captured URLs, headers, and filenames are controlled by whoever made the file. How does Netscope keep them from sending escape sequences to the terminal, what does it miss, and how is the terminal restored after an error or a panic?

<details>
<summary>Answer</summary>

`terminal_text` replaces every character for which `char::is_control` is true with its escaped form, so ESC becomes `\u{1b}`. The views and the non-interactive summary pass captured strings and filenames through it. Binary bodies are shown as hex, and previews are cut on character boundaries.

`is_control` covers only Unicode category Cc: U+0000 to U+001F and U+007F to U+009F. That includes ESC, so a captured value cannot start a terminal command. Format characters in category Cf pass through unchanged. The right-to-left override U+202E, the bidi isolates, and the zero-width space can make a URL display differently from its bytes.

The `Restore` guard enables raw mode and marks the alternate screen as entered before writing the escape that enters it, so a partial failure still triggers cleanup. Its `restore` runs on the normal path, and its `Drop` runs on an error return or during a panic unwind. A panic hook on the interface thread leaves the alternate screen and disables raw mode before the panic message is printed, so the message stays visible on the normal screen.

**Common wrong answer.** "All dangerous Unicode is escaped." Only Cc controls are; bidi and zero-width characters are a listed gap.

**Learn more:** [Hostile input files](../chapters/34-defending-the-design.md#hostile-input-files) · [Leave the terminal usable](../chapters/11-drawing-and-navigation.md#leave-the-terminal-usable) · [`terminal_text`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L285-L296) · [`Restore`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L42-L82)

</details>

### What stops a DNS compression loop

**Level:** Expert

<!-- chapters: 32-reading-signaling -->

How does the DNS name reader avoid compression-pointer loops and unbounded work? When is a UDP datagram treated as DNS or STUN, and what is out of scope?

<details>
<summary>Answer</summary>

The name reader requires each compression pointer to target an offset earlier than the pointer itself. That rule blocks forward jumps, but it does not stop a loop. A one-byte label at offset 12 followed by a pointer at offset 14 that targets 12 satisfies the rule and repeats forever under it alone. What ends that loop is the 255-byte name limit: each pass adds a label of at least 2 encoded bytes, so the name fails after 127 passes. A chain of pointers with no labels cannot loop, because every target is earlier than its pointer, and a separate count stops it after 127 jumps with "DNS name traversal limit". Rejection of label types 0x40 and 0x80 and checked reads for every slice bound the rest of the work. Because labels and jumps are counted separately, a legal name of 127 one-byte labels is accepted.

The header is limited to 16 questions and 32 answers, and a nonzero opcode is rejected. DNS is tried only when the UDP source or destination port is 53, and only for datagrams retained in full. STUN is recognised by two zero leading bits, the magic cookie 0x2112A442, a declared length that is a multiple of 4 and equals the payload minus 20, and at most 64 attributes.

DNS over TCP, mDNS on port 5353, DoH, DoT, the authority and additional sections, and EDNS are out of scope.

**Common wrong answer.** "Pointers must point backwards, so loops are impossible." A backward pointer can still return to a label that leads back to the same pointer; the 255-byte name limit is what terminates it.

**Learn more:** [Can a malicious DNS packet hang the parser?](../chapters/34-defending-the-design.md#can-a-malicious-dns-packet-hang-the-parser) · [A datagram has its own boundary](../chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary) · [`name`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L7-L56) · [`inspect_datagram`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L113-L126)

</details>

### Searching for API

**Level:** Expert

<!-- chapters: 11-drawing-and-navigation -->

A user presses `/` and searches for `API` to find `https://example.com/api/books`. Will it match? What does the search look at, when does it run, and how does it combine with the host filter?

<details>
<summary>Answer</summary>

It will not match. The search is a case-sensitive substring test, `str::contains`, on the raw recorded URL string. It does not look at headers or bodies, and it does no normalisation or pattern matching.

The query is applied when Enter is pressed, not on each keystroke. The query is limited to 1024 bytes. It filters the current host filter's chronological list, or every request when no host filter is set. The matches are rebuilt as a list of request IDs, and the previously selected request stays selected if it still matches. Otherwise the first row is selected. Pressing `a` clears both the query and the host filter.

Each Enter costs O(n × m) for n requests and a query of length m. The captured requests are never moved or copied; only the list of matching IDs changes.

**Common wrong answer.** "The search is case-insensitive and also looks at headers." It is neither.

**Learn more:** [Find a request without changing the capture](../chapters/11-drawing-and-navigation.md#find-a-request-without-changing-the-capture) · [URL search](../contracts/m4-url-search.md) · [`rebuild_search`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L164-L177)

</details>

### Limitations to disclose for incident work

**Level:** Expert

<!-- chapters: 23-shaping-the-program, 33-interpreting-a-trace -->

A security engineer is evaluating Netscope for incident work. Name the five most important limitations you would disclose, and say for each whether it is a deliberate scope choice or a gap.

<details>
<summary>Answer</summary>

Any five of these, each labelled correctly, make a good answer.

Scope: only classic PCAP is read. PCAPNG is rejected, and link types outside 0, 1, 101, 113, 228, 229 and 276 show "Link type unsupported". Scope: TLS is neither parsed nor decrypted, so HTTPS in a packet capture is opaque. Scope: IPv4 fragments are not reassembled, IPv6 extension headers are not walked, and checksums are not verified. Scope: wire HTTP is HTTP/1.1 with GET and POST requests only, and the two directions are not paired.

Gap: a response to HEAD is misframed, and port reuse without a captured SYN merges into the old epoch. Gap: replay trusts only the bundled root certificates and ignores proxies. Gap: saved drafts keep URL query tokens and unlisted secret headers, and the parent directory is not synced. Gap: terminal escaping misses bidi and zero-width characters. Gap: retry matching can be quadratic, and it misses real retries that change one header.

Scope: WebSocket support covers handshake header shape only, and SSE in a HAR has no timing. Scope: the import and reassembly budgets mean a large capture is analysed in part, and the views say so.

**Learn more:** [Known gaps and pitfalls](../chapters/34-defending-the-design.md#known-gaps-and-pitfalls) · [Put the limit next to the conclusion](../chapters/33-interpreting-a-trace.md#put-the-limit-next-to-the-conclusion) · [`layers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L189-L254)

</details>

### A 304 with both length headers

**Level:** Expert

<!-- chapters: 31-finding-http-boundaries -->

A packet capture contains `HTTP/1.1 304 Not Modified` with both `Transfer-Encoding: chunked` and `Content-Length: 10`, followed by another response on the same connection. How does Netscope label the 304, and is that right?

<details>
<summary>Answer</summary>

Netscope labels it "Status-defined header-only boundary", ends it at the empty line after its headers, and goes on to inspect the following response. That is right.

RFC 9112 section 6.3 lists the status rule before the Transfer-Encoding rule: a 1xx, 204, or 304 response has no body, whatever length headers it carries. In `message`, the status block runs first and returns early, so the check for both headers together never sees this message. The same holds for a 204 or a 1xx response with both headers. A 101 is the exception in another direction: it ends at its headers but stops inspection, because the protocol changes.

For any other status, or for a request, both headers together are labelled "Ambiguous Transfer-Encoding with Content-Length" and inspection stops. That combination is the classic request-smuggling pattern, because two parsers can disagree about where the body ends.

**Common wrong answer.** "Both headers are present, so the 304 is labelled ambiguous." The status rule comes first, as the RFC orders it. Earlier versions of Netscope did get this wrong.

**Learn more:** [Why refuse Transfer-Encoding together with Content-Length?](../chapters/34-defending-the-design.md#why-refuse-transfer-encoding-together-with-content-length) · [Some apparent boundaries are ambiguous](../chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous) · [`status`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L233-L243) · [`transfer`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L244-L247)

</details>

### Esc pressed just after the worker finished

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response, 24-sharing-work -->

A replay completes normally, but the user presses Esc at almost the same moment. Under what ordering can Esc arrive after the worker finished, and what does the screen say then?

<details>
<summary>Answer</summary>

The terminal loop polls the worker before it handles a key, so most late Esc presses arrive after the result has been accepted. The window is smaller. The key path polls and finds nothing yet. The worker then sends its Complete result. The loop handles Esc, which only sets the editor's cancel flag. On the next pass, the loop polls first, finds the result, and calls `complete`.

`complete` returns to the draft, because the user asked to leave the running view, and keeps the result, which opens with `o`. It chooses the message from the result's outcome, not from the cancel flag. No cancel signal was ever sent and the outcome is Complete, so the message is "Finished before Esc took effect; o opens its retained result". Only a `Cancelled` outcome produces "Operation stopped". The view test `esc_after_the_worker_finished_does_not_claim_a_stop` drives this order.

**Common wrong answer.** "The screen says the operation stopped." That was true of earlier versions, which chose the message from the cancel flag.

**Learn more:** [Known gaps and pitfalls](../chapters/34-defending-the-design.md#known-gaps-and-pitfalls) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`complete`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L174) · [`cancellation_requested`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L185-L187)

</details>

### One far-away segment pushes out a stream

**Level:** Expert

<!-- chapters: 30-putting-bytes-in-order, 29-reading-packet-bytes -->

A hostile capture records, as the first data segment of a direction, a segment whose sequence number is 200 KiB from the anchor. Ordinary segments near the anchor follow. What does reassembly keep, and why does record order matter here?

<details>
<summary>Answer</summary>

Each segment is checked in record order against two budgets before it is accepted. The epoch's span, from its lowest to its highest position, must stay within 256 KiB, and the sum of all accepted spans must stay within 8 Mi positions. A segment that would break either budget is skipped and counted in `skipped_segments`.

The far-away segment is accepted first, so the span already covers about 200 KiB of mostly Missing positions. Each later segment near the anchor fits only while the span stays under 256 KiB. Once a segment would push it past, that segment and any later one that extends the range are skipped. The epoch keeps the outlier and loses real data. A skipped segment also makes `observed_end` false, so close-delimited framing is never claimed.

The budgets exist because sequence numbers come from the file, and a dense array sized from a hostile claim would exhaust memory. The cost is that which bytes survive depends on record order. The views report the skipped count, so the loss is visible, not silent.

**Learn more:** [How are memory and time bounded in reassembly?](../chapters/34-defending-the-design.md#how-are-memory-and-time-bounded-in-reassembly) · [`STREAM_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L8) · [`TOTAL_STREAM_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L167-L173)

</details>

### A keep-alive probe that looks like an attack

**Level:** Expert

<!-- chapters: 33-interpreting-a-trace, 30-putting-bytes-in-order -->

A client sends a TCP keep-alive probe that carries one garbage byte at SND.NXT - 1, and later a server sends an RST segment with a short diagnostic text payload. How does Netscope's reassembly treat each, and what would you change?

<details>
<summary>Answer</summary>

A keep-alive probe repeats the last byte already sent. RFC 1122 allows it to carry one garbage byte, and Linux sends none, so this depends on the sender's stack. Without special handling, that byte would land on a position that already holds the real last byte, the values would differ, and the position would become `Conflict`, cutting the HTTP prefix one byte early. `reassemble` recognises the case: a one-byte segment with no SYN, FIN or RST, whose byte lands on the last position placed so far in file order and differs from it, is skipped and counted in `ignored_probes`. A differing one-byte segment anywhere else is still a conflict, so the rule cannot hide an injected byte in the middle of the stream.

An RST segment sets `reset_observed`, which already rules out an observed end. Its declared payload length is treated as zero, so the diagnostic text never enters the stream or extends the span, and the flow carries the note "Reset segment payload not placed in the stream".

**Common wrong answer.** "Both create a conflict, so the prefix ends early." Earlier versions did that; both cases are now handled before the overlap rule.

**Learn more:** [What happens when two segments disagree about the same byte?](../chapters/34-defending-the-design.md#what-happens-when-two-segments-disagree-about-the-same-byte) · [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [`declared`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L147-L156) · [`ignored_probes`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L203-L216) · [`ByteState::Conflict`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L223-L230)

</details>

### A replay that times out although IPv4 works

**Level:** Expert

<!-- chapters: 17-waiting-for-a-response -->

A host has both IPv6 and IPv4 addresses. Its IPv6 route silently drops packets, while IPv4 works. A browser loads the page quickly, but a Netscope replay to the same URL times out. Why, and what would you change?

<details>
<summary>Answer</summary>

`TcpStream::connect` resolves the host and tries each returned address in turn, moving on only after an attempt fails. A silently dropped SYN does not fail quickly. The operating system keeps retransmitting it for longer than Netscope's single 10-second deadline, which starts when Send is accepted and covers DNS, connection, TLS, and the whole response. If the IPv6 address comes first, the deadline expires while the client is still waiting on it, and the result is `TimedOut` without trying IPv4.

Browsers avoid this with Happy Eyeballs (RFC 8305): they start an IPv6 attempt, start an IPv4 attempt a short delay later if the first has not connected, and keep whichever connects first.

The change would be to race address families as RFC 8305 describes, inside the same deadline. Reporting which stage failed would also help, because today the result records only total elapsed time, not separate DNS, connect, and TLS durations.

**Learn more:** [How does replay use TLS, and what is missing?](../chapters/34-defending-the-design.md#how-does-replay-use-tls-and-what-is-missing) · [Keep one deadline for the whole request](../chapters/17-waiting-for-a-response.md#keep-one-deadline-for-the-whole-request) · [`TcpStream::connect`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L231-L233) · [`TIME_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L30)

</details>
