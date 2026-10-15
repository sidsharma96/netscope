# Practitioner questions

These questions ask how Netscope works and why it takes each approach instead of the obvious alternative. They assume you have read the teaching chapters for the topic, or are willing to follow the links. Try to answer each question in your own words before opening the answer. Each answer ends with links to the section that teaches the idea and to the code that implements it.

### What does each crate own?

**Level:** Practitioner

<!-- chapters: 23-shaping-the-program, 04-quality-and-tests -->

The workspace has four crates: `netscope-core`, `netscope-import`, `netscope-net` and `netscope-tui`. What does each one own, which way do the dependencies point, and why does the core crate contain no file access, no network access and no async runtime?

<details>
<summary>Answer</summary>

`netscope-core` holds the data model and the pure analysis: sessions, requests and timings, redirect and retry suggestions, SSE and WebSocket evidence, packet types, TCP reassembly, wire HTTP framing, DNS and STUN recognition, replay drafts and the saved-draft format. Its only dependencies are `url`, `serde` and `serde_json`. `netscope-import` reads HAR and classic PCAP files into core types. `netscope-net` performs the one reviewed HTTP/1.1 exchange with tokio, hyper and rustls. `netscope-tui` depends on the other three and owns drawing, key handling and the `netscope` binary.

The dependencies point toward the data rules: the terminal crate depends on import and net, and both of those depend on core. Keeping I/O out of core makes its analysis deterministic and easy to test with plain values. It also gives a structural guarantee: the crates that parse captures have no network dependency, so opening a capture cannot send traffic. The workspace lints also forbid `unsafe` code and deny `unwrap` and `expect`.

**Learn more:** [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules) · [Package boundaries](../ARCHITECTURE.md#package-boundaries) · [`[dependencies]`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/Cargo.toml#L11-L14) · [`[workspace.lints.rust]`](https://github.com/sidsharma96/netscope/blob/main/Cargo.toml#L15-L23)

</details>

### Why is a timing an enum?

**Level:** Practitioner

<!-- chapters: 02-reading-a-capture, 09-options-results-and-matching -->

HAR timings are numbers. Why does Netscope model an optional timing as an enum instead of `Option<f64>`, or instead of storing 0 when the value is absent? Name the variants and say what each one means.

<details>
<summary>Answer</summary>

`Timing` has four variants. `Missing` means the exporter left the field out. `Unavailable` means the exporter wrote `-1`, which HAR 1.2 defines as "does not apply to this request", for example DNS on a reused connection. `Milliseconds(f64)` holds a finite, non-negative duration. `ExporterNegative(f64)` keeps a nonstandard negative value that some exporters write for `blocked`, without treating it as a duration.

Storing 0 for an absent value would invent a measurement: a reader could not tell "took no time" from "was not recorded". `Option<f64>` is better but still merges two different facts, an absent field and a field that does not apply. With four variants, every later calculation has to say what it does with each case, and the compiler checks that it does. The required `send`, `wait` and `receive` fields are plain `f64` values, because the importer rejects an entry that lacks them.

**Common wrong answer.** "-1 means the value is unknown." HAR uses -1 for a phase that did not happen for this request, which is a different claim from "unknown".

**Learn more:** [Missing time is different from zero](../chapters/02-reading-a-capture.md#missing-time-is-different-from-zero) · [Why keep -1, missing, zero and nonstandard values apart?](../chapters/34-defending-the-design.md#why-keep--1-missing-zero-and-nonstandard-values-apart) · [`Timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L11-L22) · [`from_optional_har`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L52-L61)

</details>

### How does a session keep several orders?

**Level:** Practitioner

<!-- chapters: 08-structs-vectors-and-ids, 03-storing-requests -->

How does a `Session` identify a request, and how does it offer chronological order and per-host order without moving or copying any request?

<details>
<summary>Answer</summary>

A request's identity is `RequestId`, a wrapper around the entry's position in the HAR `entries` array. That number never changes, whatever order the screen shows. The `Session` owns one `Vec<Request>` in source order and keeps separate lists of IDs for each view: `chronological` is a `Vec<RequestId>`, `host_order` is a `Vec<HostId>` sorted by host name, and each host summary keeps its own `request_ids` in chronological order. Redirect and retry evidence are vectors indexed by the same `RequestId`.

Views borrow a request by looking up its ID, so sorting a view sorts small numbers and never moves the requests themselves. A lookup with an ID that is out of range returns `None` instead of panicking. A saved draft that has no captured request uses the sentinel `RequestId(usize::MAX)`, so it can never be mistaken for a real entry.

**Learn more:** [Sorting should preserve identity](../chapters/08-structs-vectors-and-ids.md#sorting-should-preserve-identity) · [Why store requests in a vector addressed by RequestId?](../chapters/34-defending-the-design.md#why-store-requests-in-a-vector-addressed-by-requestid) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6) · [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197-L205) · [`SAVED_SOURCE`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L151)

</details>

### Is HAR import streaming or all-or-nothing?

**Level:** Practitioner

<!-- chapters: 10-importing-a-har, 09-options-results-and-matching -->

Describe the path from a HAR file to a `Session`. Is it streaming or whole-file? Does a bad entry fail the whole import or only that entry? Which budgets apply, and what may an error message contain?

<details>
<summary>Answer</summary>

The importer reads the whole file into memory, but it reads through `take(limit + 1)`, so the 32 MiB input budget is enforced while reading instead of after. It parses the bytes into a JSON tree that rejects duplicate keys, checks that `log.version` is `"1.2"` and that the creator is present, checks the entry count against 10,000, and then parses every entry. Any invalid field that Netscope consumes fails the whole import. There is no partial session. `Session::new` then builds the orders and indexes.

The third budget is 2 MiB of decoded bytes per response body. Recoverable oddities do not fail the import. An undecodable body, a missing MIME type or timings that do not add up become per-request warnings. An error carries a field path such as `log.entries[3].response.status` and a fixed reason. Captured values never appear in error messages, so a hostile file cannot put its own text into one.

**Common wrong answer.** "Bad entries are skipped and the rest load." A structural or field error rejects the file, because a list with silent gaps would misrepresent the capture.

**Learn more:** [Decide when a failure blocks the request](../chapters/10-importing-a-har.md#decide-when-a-failure-blocks-the-request) · [Put a limit on the work](../chapters/10-importing-a-har.md#put-a-limit-on-the-work) · [Resource and error policy](../contracts/m1-har-inspection.md#resource-and-error-policy) · [`Limits`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L19-L27) · [`import_reader`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L121-L134) · [`import_bytes`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L138-L180)

</details>

### What can the PCAP importer read?

**Level:** Practitioner

<!-- chapters: 29-reading-packet-bytes -->

Which container formats, link types and network and transport layers does the PCAP importer support? What happens to a packet whose layers it cannot parse?

<details>
<summary>Answer</summary>

Only classic PCAP version 2.4 is supported, in either byte order and with microsecond or nanosecond timestamps. A PCAPNG file is rejected with an explicit message. The supported link types are 1 (Ethernet), 101 (raw IP), 228 (raw IPv4) and 229 (raw IPv6), with up to two VLAN tags. IPv4 and IPv6 are parsed, and the transport layers are TCP and UDP. Fragmented IPv4 packets are not reassembled, IPv6 extension headers are not walked, and checksums are never verified.

Errors in the file's framing fail the import: a snap length of 0 or above 256 KiB, more than 20,000 records, a file over 32 MiB, a truncated record, a captured length above the snap length or original length, or a timestamp fraction out of range. Errors inside a packet do not. The packet keeps its bytes and gets a reason in its layer list, such as "Link type unsupported; packet bytes retained". A macOS loopback capture (link type 0) or a Linux `-i any` capture (link type 113) therefore opens, but every packet carries that reason.

**Learn more:** [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer) · [Why refuse PCAPNG and other link types?](../chapters/34-defending-the-design.md#why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do) · [`import_pcap`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L69-L170) · [`layers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L171-L200)

</details>

### What is the unit of TCP reassembly?

**Level:** Practitioner

<!-- chapters: 30-putting-bytes-in-order -->

What does Netscope reassemble TCP payload into, and how does it represent a byte position that was never seen, or one that was seen twice with different values?

<details>
<summary>Answer</summary>

The unit is a directed epoch: one direction (source socket to destination socket) of one connection lifetime. The two directions of a connection are never merged or paired. Each epoch has a `bytes` vector and a parallel `states` vector of the same length, where every position is `Missing`, `Observed` or `Conflict`.

A gap stays `Missing`. Its slot in `bytes` holds a placeholder 0 that means nothing, because a byte value is meaningful only where the state is `Observed`. If two observations of the same position agree, the state stays `Observed` and `repeated_bytes` goes up by one. If they disagree, the position becomes `Conflict`. `contiguous_prefix` returns bytes only up to the first position that is not `Observed`, so later analysis such as HTTP framing never crosses a hole or a disagreement.

**Learn more:** [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`ByteState`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L13-L20) · [`contiguous_prefix`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L53-L60)

</details>

### What stops a capture from sending traffic?

**Level:** Practitioner

<!-- chapters: 16-preparing-a-request, 19-submitting-text -->

Replay can send real traffic. What guarantees that opening a capture or a saved draft never sends anything, and which methods and body types can be replayed?

<details>
<summary>Answer</summary>

Network code lives only in `netscope-net`, and the only place that starts it is the terminal loop. That loop starts a worker only when `take_send` returns a pending send, and a pending send is set only when you press Enter or `s` on the Review screen. Drawing and editing never start I/O. `ReplayReview` is plain data with no way to send anything, and the draft is a separately owned copy, so editing it never changes the imported request. A saved draft opens in the editor with no automatic send.

Replay supports two methods. GET is accepted only when the capture reports a request body size of 0 and recorded no body. POST is accepted with `text/plain` or `application/json`, in UTF-8 (an optional `charset=utf-8` is the only parameter allowed), at most 16 KiB, and without a content encoding. WebSocket URLs are refused because WebSocket captures are inspection-only.

**Learn more:** [Give the draft its own text](../chapters/16-preparing-a-request.md#give-the-draft-its-own-text) · [Review before sending](../contracts/m3-single-request-replay.md#review-before-sending) · [`take_send`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L139-L141) · [`from_request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L126-L189) · [`Worker::start_at`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L157-L172)

</details>

### How does a synchronous terminal meet an async client?

**Level:** Practitioner

<!-- chapters: 17-waiting-for-a-response, 24-sharing-work -->

The terminal loop is synchronous, while the HTTP client is async (tokio and hyper). How do the two meet? Describe the threads, runtimes and channels involved.

<details>
<summary>Answer</summary>

Each Send creates a `Worker`, which spawns a dedicated OS thread named `netscope-replay`. That thread builds a current-thread tokio runtime and blocks on the exchange inside it. Three channels connect the worker to the terminal. A `tokio::sync::oneshot` carries cancellation from the terminal to the worker. A `std::sync::mpsc::sync_channel(1)` carries the single final `ReplayResult` back. For live SSE only, a `tokio::sync::watch` carries the newest cumulative snapshot, replacing the previous one instead of queueing every update.

The terminal loop waits at most 50 ms for a key, and on every pass it calls `progress` and `poll`, which use non-blocking reads. At most one worker runs at a time. Dropping a `Worker` sends cancellation and joins its thread. The terminal therefore stays synchronous and needs no async runtime of its own, and each replay gets a small runtime that is torn down when it finishes.

**Learn more:** [Why keep a separate worker at all?](../chapters/17-waiting-for-a-response.md#why-keep-a-separate-worker-at-all) · [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Why give each send its own thread and runtime?](../chapters/34-defending-the-design.md#why-give-each-send-its-own-thread-and-runtime) · [`start_mode`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L338-L404) · [`poll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L421-L430)

</details>

### Does Netscope say which request followed a redirect?

**Level:** Practitioner

<!-- chapters: 13-following-redirects, 14-recognizing-repeats -->

Netscope shows redirect and retry relationships between requests in a HAR. Does it claim that request B is the redirect follow-up or retry of request A? What does it report instead, and why?

<details>
<summary>Answer</summary>

It never claims a cause. `RedirectEvidence` and `RetryEvidence` each have three variants. `NotApplicable` means the source status is outside the analysis. `Unavailable(reason)` means the search could not run, for example because there is no usable `Location` header. `Candidates` lists up to 20 request IDs, with a `more` flag when further matches exist.

A HAR does not record which request caused another. Two identical GETs after a 503 could be a user reload, another tab or a client retry. Netscope therefore reports requests that are consistent with the relationship and leaves the choice to you. Keeping three variants also separates "the search could not run" from "the search ran and found nothing", which an empty list alone could not express.

**Learn more:** [Two plausible requests are still two requests](../chapters/13-following-redirects.md#two-plausible-requests-are-still-two-requests) · [Give different outcomes different Rust values](../chapters/13-following-redirects.md#give-different-outcomes-different-rust-values) · [`RedirectEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L7-L21) · [`RetryEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L7-L19)

</details>

### What does Netscope inspect in SSE and WebSocket captures?

**Level:** Practitioner

<!-- chapters: 25-reading-event-streams, 26-switching-protocols -->

For server-sent events and WebSocket, what does Netscope actually inspect in a HAR, and what does it explicitly not claim?

<details>
<summary>Answer</summary>

For a response whose MIME type is `text/event-stream` (letter case and parameters ignored), Netscope parses the retained response text into events. The view states "Order only · event arrival times and actual browser delivery are not recorded", because a HAR keeps the text but not when each event arrived. The last-event-ID buffer starts empty, since earlier connection state is unknown, and a final blank line does not prove the capture is complete.

For WebSocket, Netscope inspects only the classic HTTP/1.1 upgrade headers and reports a summary with notes. It does not recover frames or messages, does not verify the `Sec-WebSocket-Accept` digest, and does not check extensions or origin policy. A handshake reported over another HTTP version gets "Outside classic HTTP/1.1 upgrade inspection". Separately, live SSE replay can observe a real stream with local timing.

**Learn more:** [A blank line gives a boundary meaning](../chapters/25-reading-event-streams.md#a-blank-line-gives-a-boundary-meaning) · [The exporter can stop at the agreement](../chapters/26-switching-protocols.md#the-exporter-can-stop-at-the-agreement) · [`inspect_sse`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/sse.rs#L55-L73) · [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L28-L47)

</details>

### When do timing stages disagree with the total?

**Level:** Practitioner

<!-- chapters: 02-reading-a-capture, 10-importing-a-har -->

How does the importer decide to warn "Timing stages differ from reported total"? Which fields are added, which are left out and why, what tolerance applies, and how is a `blocked` value between -1 and 0 treated?

<details>
<summary>Answer</summary>

The importer adds `send`, `wait` and `receive`, then adds `blocked`, `dns` and `connect` only when each one is a measured `Milliseconds` value. Missing, not-applicable and nonstandard values contribute nothing, so no duration is invented. `ssl` is left out because HAR defines the TLS time as part of `connect`, and adding it again would count it twice. If the sum is not finite, or differs from the entry's `time` by more than 0.01 ms, the request gets a warning, not an error.

A second check warns when `ssl` is a duration but `connect` is not a duration at least as large. A `blocked` value strictly between -1 and 0 becomes `ExporterNegative` with its own warning, because some exporters write small negative values there. Any other negative value apart from -1 is an import error, and only `blocked` gets this tolerance.

**Learn more:** [Some timings contain other timings](../chapters/02-reading-a-capture.md#some-timings-contain-other-timings) · [Timing rules](../contracts/m1-har-inspection.md#timing-rules) · [`optional_timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L352-L372) · [`sum`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L507-L524)

</details>

### How are requests sorted and hosts grouped?

**Level:** Practitioner

<!-- chapters: 12-grouping-requests, 08-structs-vectors-and-ids -->

How exactly is chronological order computed, including the tie-break and the cost? How are timestamps with different UTC offsets compared? What is the host grouping key, does it include the port, and in what order are hosts listed?

<details>
<summary>Answer</summary>

`startedDateTime` must be an RFC 3339 timestamp with an explicit offset, or the import fails. The importer converts it to UTC seconds and nanoseconds, so timestamps written with different offsets compare as real instants. The chronological list is the request IDs sorted by `(instant, source index)`. Equal instants keep source order, and the sort costs O(n log n).

The host key is the URL host as normalised by the `url` crate: lowercased, with international names converted to their ASCII form and IPv6 addresses in brackets. The port and scheme are not part of the key, so `example.com:8443` and `https://example.com` share one host. Hosts receive IDs in the order they first appear, using a `BTreeMap` lookup, and each host's request IDs are filled in chronological order. The host view lists hosts sorted by name, byte by byte.

**Common wrong answer.** "Hosts are grouped by host and port." Netscope groups only by the normalised host name.

**Learn more:** [Choose what makes a group](../chapters/12-grouping-requests.md#choose-what-makes-a-group) · [Deterministic identity and display](../contracts/m1-har-inspection.md#deterministic-identity-and-display) · [`timestamp`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L212-L216) · [`Session::new`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L209-L234)

</details>

### Why not parse HAR into a plain JSON value?

**Level:** Practitioner

<!-- chapters: 10-importing-a-har -->

`serde_json::Value` is the obvious way to parse a HAR. What does Netscope do differently at the JSON level, and why? Cover duplicate keys, a UTF-8 byte-order mark, trailing data, nesting depth, and how a base64 body is kept from allocating beyond its budget.

<details>
<summary>Answer</summary>

`serde_json::Value` silently keeps the last of two members with the same key. A HAR with two `"status"` members could then show one value in Netscope and another in a different tool. Netscope deserialises through its own `Unique` visitor, which rejects a duplicate member. It strips one leading UTF-8 byte-order mark, and `parser.end()` rejects any data after the JSON value. Nesting depth relies on serde_json's default recursion limit, reported as "invalid JSON or parser depth limit exceeded".

For a base64 body, the importer first estimates the decoded size from the text length minus padding. If the estimate exceeds 2 MiB, the body becomes `Undecoded` with "decoded body limit exceeded" without decoding anything. It checks the real size again after decoding. Malformed base64 or an unknown encoding also become `Undecoded`, with a warning instead of an import failure. Decoded bytes that are valid UTF-8 become `DecodedText`; other bytes become `Binary`.

**Learn more:** [Reading structure comes before understanding meaning](../chapters/10-importing-a-har.md#reading-structure-comes-before-understanding-meaning) · [Hostile input files](../chapters/34-defending-the-design.md#hostile-input-files) · [`Unique`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L50-L53) · [`Unique::deserialize`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L142-L156) · [`response_body`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L321-L351)

</details>

### Which length bounds a TCP payload?

**Level:** Practitioner

<!-- chapters: 29-reading-packet-bytes, 30-putting-bytes-in-order -->

A TCP segment's payload length could come from the captured record length, the IP header's length field, or the snap length. Which one does Netscope use for `declared_payload`, which one for the retained byte range, and why does the difference matter?

<details>
<summary>Answer</summary>

The IP layer computes a declared end: the start of the IP header plus the IPv4 total length, or plus 40 and the IPv6 payload length. The retained end is the smaller of that declared end and the captured bytes. `declared_payload` is the declared end minus the IP and TCP headers, so it comes from the IP length field. The retained `payload` range runs from the end of the TCP header to the retained end, so it holds only bytes that are present in the file.

Two cases show why both are needed. Ethernet pads short frames to 60 bytes, and using the record length would turn that padding into fake TCP payload; the IP length excludes it. In a capture cut by its snap length, the declared length still says how much was sent, so reassembly reserves the whole sequence range and marks the uncaptured part `Missing` instead of pretending the segment was short. UDP works the same way, with a `complete` flag when the whole datagram was retained.

**Learn more:** [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer) · [How does Netscope turn TCP segments into a byte stream?](../chapters/34-defending-the-design.md#how-does-netscope-turn-tcp-segments-into-a-byte-stream) · [`declared_end`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L210-L240) · [`declared_payload`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L297-L306)

</details>

### What does Netscope show for HTTPS in a PCAP?

**Level:** Practitioner

<!-- chapters: 29-reading-packet-bytes, 31-finding-http-boundaries -->

You open a PCAP of HTTPS traffic to port 443. What does Netscope show about the TLS handshake (ClientHello, server name, certificate), and what does it show about the HTTP inside it?

<details>
<summary>Answer</summary>

Nothing specific to TLS. Netscope has no TLS record or handshake parser for packet captures, so it does not extract the server name (SNI), list cipher suites or show certificates. The port 443 stream is reassembled like any other TCP stream, with its byte states and SYN, FIN and RST flags, and the packet view shows the layers and the bytes.

When the HTTP/1.1 parser examines that stream, it stops with "No supported GET/POST or HTTP/1.1 response start at prefix", because TLS records begin with a record-type byte such as 0x16 or 0x17, not with a request line. The HTTP inside is not visible: Netscope does not decrypt traffic and does not accept key-log files. TLS appears elsewhere in Netscope only as the HAR `ssl` timing and as the rustls client that replay uses.

**Common wrong answer.** "It shows the SNI from the ClientHello." No code in Netscope parses TLS handshake messages from a capture.

**Learn more:** [TLS: the handshake, SNI, ALPN and certificate checks](../chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks) · [Plaintext HTTP/1.1 subset](../contracts/m8-streams.md#plaintext-http11-subset) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186-L214)

</details>

### How does a segment find its directed epoch?

**Level:** Practitioner

<!-- chapters: 30-putting-bytes-in-order -->

Walk through how `reassemble` decides which directed epoch a TCP segment belongs to and how it computes the segment's position. How does it handle sequence numbers that wrap around?

<details>
<summary>Answer</summary>

`reassemble` makes one pass in file order. Its key is the pair (source socket, destination socket), and a `BTreeMap` holds the current epoch for each key. If the segment has SYN set, its sequence number is increased by one, so position 0 is the first data byte after the SYN. A new epoch starts when the key has no active epoch, or when the segment is a SYN and the active epoch had no SYN, has a different starting number, or is already closed by FIN or RST. Otherwise the segment joins the active epoch. A new epoch's anchor is the adjusted sequence number of its first segment.

A segment's position is `sequence.wrapping_sub(anchor)` reinterpreted as a signed 32-bit number. Wrapping subtraction followed by the signed cast gives the correct signed distance across the 2^32 wrap, as long as the distance stays below 2^31 either way, which the 256 KiB span limit guarantees. Segments that carry no payload and no FIN or RST join the epoch but add no positions.

**Learn more:** [Sequence numbers eventually wrap](../chapters/30-putting-bytes-in-order.md#sequence-numbers-eventually-wrap) · [How TCP numbers its bytes](../chapters/34-defending-the-design.md#how-tcp-numbers-its-bytes) · [`reassemble`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L89-L126) · [`start`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L141-L146)

</details>

### What limits bound reassembly?

**Level:** Practitioner

<!-- chapters: 30-putting-bytes-in-order, 33-interpreting-a-trace -->

What are the flow, per-stream and total budgets in TCP reassembly? What happens to a packet or segment when each is exceeded, and what are the time and memory costs?

<details>
<summary>Answer</summary>

`FLOW_LIMIT` allows 128 directed epochs, `STREAM_LIMIT` allows a 256 KiB sequence span per epoch, and `TOTAL_STREAM_LIMIT` allows 8 MiB of spans in total. When a packet would need a 129th epoch, it is counted in `unassigned_packets`, and its key is removed from the active map, so later packets in that direction are unassigned too. A segment that would stretch its epoch past 256 KiB, push the total past 8 MiB, or start before the SYN anchor is skipped and counted in `skipped_segments`, and its FIN and RST flags are not recorded. An epoch with any skipped segment never reports an observed end.

Grouping costs O(P log F) for P packets and F epochs. Placing bytes costs time proportional to the payload bytes examined. Memory is about two bytes per retained position (the byte and its state), so at most around 16 MiB for 8 MiB of span, plus the packet bytes themselves. The PCAP limits of 20,000 packets and 32 MiB apply before any of this.

**Learn more:** [How are memory and time bounded in reassembly?](../chapters/34-defending-the-design.md#how-are-memory-and-time-bounded-in-reassembly) · [Sequence observations](../contracts/m8-streams.md#sequence-observations) · [`FLOW_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L6-L10) · [`unassigned_packets`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L102-L107) · [`skipped`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L165-L173)

</details>

### In what order does the wire HTTP parser apply framing rules?

**Level:** Practitioner

<!-- chapters: 31-finding-http-boundaries -->

Which bytes does the HTTP/1.1 parser receive from a reassembled stream, and in what order does it apply the framing rules (Transfer-Encoding, Content-Length, status codes, a request without framing, a close-delimited response)? What limits apply?

<details>
<summary>Answer</summary>

The parser receives only the contiguous prefix of one direction, cut to 256 KiB, together with a flag saying whether the stream's end was observed. It parses up to eight messages and stops at the first one it cannot frame, without guessing a later one. A start line must be `HTTP/1.1` with a three-digit status from 100 to 599, or exactly `GET` or `POST`, a target and `HTTP/1.1`. Header lines may be at most 4 KiB, the header section 32 KiB, and the count 100 fields.

The rules then apply in order. First, a 1xx, 204 or 304 status ends at the headers whatever its length headers say, except that 101 changes protocol and stops parsing. Second, Transfer-Encoding and Content-Length together are reported as ambiguous and left unframed, because that combination is the classic request-smuggling pattern. Third, Transfer-Encoding must be exactly one `chunked`, with at most 1024 chunks. Fourth, Content-Length must be one decimal value up to 256 KiB. Fifth, a request with neither has no body. Last, a response with neither is close-delimited, and counts as framed only when the SYN-to-FIN span was observed without gaps.

**Learn more:** [A length and a delimiter answer different questions](../chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions) · [Some apparent boundaries are ambiguous](../chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous) · [Why refuse Transfer-Encoding together with Content-Length?](../chapters/34-defending-the-design.md#why-refuse-transfer-encoding-together-with-content-length) · [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140-L185) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186-L360)

</details>

### What happens between a draft and the wire?

**Level:** Practitioner

<!-- chapters: 16-preparing-a-request, 19-submitting-text -->

Walk through what happens between editing a draft and bytes leaving the socket. Which validations run where, which headers may you not set, which headers does the client add, and how are URL credentials and fragments handled?

<details>
<summary>Answer</summary>

A draft starts with every captured header unselected, and headers named in the captured `Connection` header cannot be selected. Each selected or added header must have a token name, a value of printable ASCII or tabs, and at most 8 KiB per name or value. Names the client manages are refused: `host`, `content-length`, `content-type`, `content-encoding`, `transfer-encoding`, `connection`, `proxy-connection`, `keep-alive`, `te`, `trailer`, `upgrade`, `proxy-authorization` and `accept-encoding`. Changing the URL clears selections, added headers and the body.

`review` checks the body (GET must be empty; JSON must parse but is sent byte for byte) and the URL: at most 16 KiB, no whitespace, controls or backslashes, HTTP or HTTPS with a host, and no embedded credentials. It removes the fragment, keeping it only for display. `netscope_net::validate` then repeats the transport checks without I/O. On Send the review is rebuilt from the current draft. The client inserts `Host`, `Accept-Encoding: identity`, and for POST `Content-Type` and `Content-Length`; live SSE also inserts `Accept: text/event-stream`.

**Learn more:** [Review the current draft, not an earlier one](../chapters/16-preparing-a-request.md#review-the-current-draft-not-an-earlier-one) · [Choose outgoing headers explicitly](../contracts/m3-single-request-replay.md#choose-outgoing-headers-explicitly) · [`validate_replay_header`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L388-L424) · [`review`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L329-L383) · [`request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L99-L156)

</details>

### How do the deadline and cancellation work?

**Level:** Practitioner

<!-- chapters: 17-waiting-for-a-response -->

When does the replay deadline start, how long is it for a normal replay and for live SSE, does progress extend it, and what makes Esc take effect? In what order are cancellation, the deadline and completion checked?

<details>
<summary>Answer</summary>

The deadline starts at the instant the terminal accepted Send, before the worker thread exists, so thread setup counts against it. It is one absolute deadline covering connection, TLS, headers and the whole body: 10 seconds for a normal replay and 30 seconds for live SSE. Progress never extends it, and tests check that a slow stream of bytes does not restart the clock.

The worker runs the exchange inside a `tokio::select!` marked `biased`, which checks the cancellation channel first, then `sleep_until(deadline)`, then the work. When cancellation or the deadline wins, the losing futures are dropped, and dropping them closes the connection and its socket. Because the deadline is checked before the work, an already expired deadline never opens a connection. Bytes copied into the result before the stop are kept and reported with the `Cancelled` or `TimedOut` outcome.

**Learn more:** [Keep one deadline for the whole request](../chapters/17-waiting-for-a-response.md#keep-one-deadline-for-the-whole-request) · [Cancellation follows ownership](../chapters/17-waiting-for-a-response.md#cancellation-follows-ownership) · [How do cancellation and the deadline decide the outcome?](../chapters/34-defending-the-design.md#how-do-cancellation-and-the-deadline-decide-the-outcome) · [`wait_for`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L297-L308) · [`TIME_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L30) · [`started`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L454)

</details>

### How are retry candidates found?

**Level:** Practitioner

<!-- chapters: 14-recognizing-repeats -->

Which source requests are eligible for retry analysis, which statuses start the search, what must match for a later request to be a candidate, what time window applies, and how is the search implemented?

<details>
<summary>Answer</summary>

A source with status 0 gets "Unavailable", because a network failure cannot be assumed. Statuses other than 408, 429, 500, 502, 503 and 504 get "NotApplicable". The source must be a GET or HEAD with a reported body size of 0 and no captured body, its duration must be finite and non-negative, and its URL must parse conservatively as HTTP or HTTPS with the fragment removed.

The index is a `BTreeMap` from (URL, method) to request IDs in chronological order, built once at import. For each source, `partition_point` binary-searches that list for the first request that starts strictly after the source starts and at or after the source ends. The scan then moves forward and stops once a request starts more than 30,000 ms after the source's end. A candidate must have exactly the same recorded headers, cookies and query lists, in the same order and case. Up to 20 candidates are kept, and `more` records that another exists.

**Common wrong answer.** "Status 0 triggers a retry search because it means a network error." Netscope refuses to assume that.

**Learn more:** [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [Why is binary search valid in redirect and retry matching?](../chapters/34-defending-the-design.md#why-is-binary-search-valid-in-redirect-and-retry-matching) · [Eligibility and comparison](../contracts/m2-retry-suggestions.md#eligibility-and-comparison) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L27-L96)

</details>

### Which redirect candidates are accepted?

**Level:** Practitioner

<!-- chapters: 13-following-redirects -->

How is the redirect target derived, and which candidate methods are accepted for 301 and 302, for 303, and for 307 and 308? What makes the analysis return "Unavailable" instead of a list of candidates?

<details>
<summary>Answer</summary>

Only statuses 301, 302, 303, 307 and 308 are considered. The response must have exactly one `Location` header. With none, the result is "No Location header recorded; redirectURL metadata is not used", because HAR's `redirectURL` field is deliberately ignored. With more than one, no target is chosen. The value is trimmed of spaces and tabs and rejected if empty or if it contains whitespace, controls or backslashes. It is resolved against the source URL with `Url::join` and must give an HTTP or HTTPS URL; the fragment is removed.

For 303, a candidate must be GET or HEAD. For 301 and 302, it must use the same method, or GET when the source was POST, matching historical browser behaviour. For 307 and 308 the method must be the same. Candidates must have the same normalised URL, start after the source ends and within 30 seconds, and at most 20 are kept. Headers are not compared. A candidate URL with odd characters is simply left out of the index; only a bad source URL or target makes the result "Unavailable".

**Learn more:** [Read the instruction before finding a follow-up](../chapters/13-following-redirects.md#read-the-instruction-before-finding-a-follow-up) · [Evidence and matching](../contracts/m2-redirect-suggestions.md#evidence-and-matching) · [`method_matches`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L23-L32) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L34-L118)

</details>

### How does the offline SSE parser read text?

**Level:** Practitioner

<!-- chapters: 25-reading-event-streams -->

Describe how `parse_sse` handles line endings, a byte-order mark, multi-line data, the `id` and `retry` fields, a final block without a blank line, and its limits.

<details>
<summary>Answer</summary>

The parser examines at most 64 KiB, cut back to a character boundary, and records a text limit if that cut anything. It ignores one leading byte-order mark. A line ends at CR, LF or CRLF. A line starting with `:` is a comment and is counted. Any other line splits at its first colon, and one leading space is removed from the value. Each `data` line appends its value and a newline. A blank line dispatches an event if data is present, removing only the final newline, with the type defaulting to `message`; the event type resets at every blank line.

An `id` value sets the last-ID buffer unless it contains a NUL character, and each event records the buffer's value at its boundary. A `retry` value is kept only if it is all ASCII digits, and it is stored as the digit string, so a huge number cannot overflow. Unknown fields are counted as ignored. A final block with no blank line is returned as `pending` and produces no event. Parsing stops at the 101st event with an event limit.

**Learn more:** [A blank line gives a boundary meaning](../chapters/25-reading-event-streams.md#a-blank-line-gives-a-boundary-meaning) · [A file can stop before an event does](../chapters/25-reading-event-streams.md#a-file-can-stop-before-an-event-does) · [Retained text rules](../contracts/m5-captured-sse.md#retained-text-rules) · [`parse_sse`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/sse.rs#L76-L149)

</details>

### What do two HTTP version labels tell you?

**Level:** Practitioner

<!-- chapters: 15-reading-protocol-labels, 28-sharing-a-connection -->

A request is labelled `HTTP/1.1` and its response `h2`. Another entry has both labels `HTTP/2.0`. What does Netscope show for each, and why does it not conclude that the second entry shared a connection with other requests?

<details>
<summary>Answer</summary>

`RecordedProtocol::from_label` recognises a fixed set of spellings and maps everything else to "Unrecognized label". `h2` and `HTTP/2.0` both become HTTP/2. The protocol notes show each raw label beside its classification. For the first entry, the two recognised labels differ, so Netscope says "Labels disagree; no upgrade, downgrade, or cause is inferred". For the second, it says the labels agree on a recognised version and calls that exporter evidence. If either label is unrecognised, comparison is unavailable.

The general explanation that follows, such as "HTTP/2 can interleave frames from different streams on one TCP connection", describes what the protocol allows, not what happened. A HAR records no frames and no stream identifiers, and overlapping requests could have used separate connections. Claiming that requests shared a connection would need connection evidence the file does not contain.

**Learn more:** [A capability is not a measurement](../chapters/15-reading-protocol-labels.md#a-capability-is-not-a-measurement) · [Keep both labels when they differ](../chapters/15-reading-protocol-labels.md#keep-both-labels-when-they-differ) · [What our recordings establish](../chapters/28-sharing-a-connection.md#what-our-recordings-establish) · [`from_label`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/protocol.rs#L17-L25) · [`protocol_lines`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L1277-L1303)

</details>

### What does a host summary's duration mean?

**Level:** Practitioner

<!-- chapters: 12-grouping-requests -->

The host view shows status counts and a duration for each host. What exactly is that duration, how are entries with status 0 counted, and why is the summary computed once at import instead of when the screen is drawn?

<details>
<summary>Answer</summary>

The duration is the sum of every entry's reported `time` for that host. Requests can overlap, so the sum is not elapsed time: two simultaneous requests of 40 ms and 60 ms add up to 100 ms although only 60 ms passed. If the sum ever overflows to a non-finite number, the summary stores `None` instead of a misleading value.

Status counts use the HTTP classes 1xx to 5xx. Status 0, which HAR uses when no HTTP status was recorded, has its own "not recorded" count instead of being treated as a success or as a particular network failure. The summary also counts requests that have at least one warning, once per request.

`Session::new` records each entry once, in source order, when the capture is imported. Drawing then reads finished numbers, so scrolling or resizing does not repeat the work, and the counts cannot depend on which view happens to be showing.

**Learn more:** [A sum answers a different question from elapsed time](../chapters/12-grouping-requests.md#a-sum-answers-a-different-question-from-elapsed-time) · [Keep missing statuses visible](../chapters/12-grouping-requests.md#keep-missing-statuses-visible) · [Why compute summaries and matches once at import?](../chapters/34-defending-the-design.md#why-compute-summaries-and-matches-once-at-import) · [`HostSummary`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/hosts.rs#L23-L35) · [`record`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/hosts.rs#L46-L61)

</details>

### Why does live SSE refuse your Accept header?

**Level:** Practitioner

<!-- chapters: 27-observing-live-events -->

Live SSE observation is a separate mode from ordinary replay. Which requests may use it, why does it reject a selected `Accept` header instead of replacing it, and which responses does it accept before reading the body?

<details>
<summary>Answer</summary>

`validate_stream` first runs the ordinary replay validation, then requires GET and refuses any selected or added `Accept` header with "Live SSE manages Accept; remove the selected Accept header first". The client sets `Accept: text/event-stream` itself. Silently replacing your header would mean the request sent differs from the request you reviewed, so Netscope asks you to remove it instead.

Once headers arrive, `validate_response` requires status 200, exactly one `Content-Type` whose media type is `text/event-stream`, and no content encoding other than `identity`, because live SSE does not decompress. Only then does it start reading body bytes. The whole observation shares one 30-second deadline that starts when you press Send and is never extended by progress, and it also stops at 64 KiB or 100 events.

**Learn more:** [Explicit network boundary](../contracts/m6-live-sse.md#explicit-network-boundary) · [Stopping is part of the observation](../chapters/27-observing-live-events.md#stopping-is-part-of-the-observation) · [`validate_stream`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L24-L37) · [`validate_response`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L108-L134)

</details>

### Where does the cursor go after an edit?

**Level:** Practitioner

<!-- chapters: 21-moving-through-text -->

The replay editor stores its cursor as a byte position in a Rust `String`. Why is a byte position safe there, how do the arrow keys move it, and what happens after an insertion joins two characters into one?

<details>
<summary>Answer</summary>

A Rust `String` is UTF-8, so a character can take one to four bytes, and one visible character (a grapheme) can combine several code points, such as `e` followed by a combining accent, or an emoji sequence joined by zero-width joiners. The cursor stores a byte offset, but it only ever rests on a grapheme boundary. Left moves to the last boundary before the cursor, and right moves to the next boundary after it, using the `unicode-segmentation` crate's extended grapheme rules.

Insertion, backspace and delete can change where the boundaries are. Inserting a joiner between two emoji, for example, can merge them into one grapheme. After every edit, `settle` moves the cursor forward to the next valid boundary, so it never sits inside a character. Backspace removes the whole previous grapheme, not one byte. Insertion is refused if it would take the text past the 16 KiB body limit.

**Learn more:** [One visible letter can have two parts](../chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts) · [Editing can change the boundaries](../chapters/21-moving-through-text.md#editing-can-change-the-boundaries) · [`Cursor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L5-L26) · [`settle`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L27-L34)

</details>

### What does opening a saved draft check?

**Level:** Practitioner

<!-- chapters: 20-remembering-a-request, 16-preparing-a-request -->

A saved draft is a small JSON file. When you open one with `netscope --draft`, what does Netscope check before it shows the editor, and why does it reject a file with an extra field instead of ignoring the field?

<details>
<summary>Answer</summary>

`SavedDraft::decode` first refuses a file larger than 512 KiB. It then parses the JSON into a structure marked `deny_unknown_fields`, so a misspelled or unexpected field fails the load instead of being dropped silently. A dropped field could mean the request you reopen differs from the one you saved, and you would not be told.

`to_draft` then checks the meaning of the values. The version must be 1, the name must hold 1 to 128 bytes with no control characters, the method must be GET or POST, and a GET must use the plain, empty body. Headers are limited to 100 entries and 32 KiB in total, each must pass the same header rules as an edited draft, and a credential header such as `Authorization` or `Cookie` rejects the file. Saving runs the same checks through `encode`, so Netscope never writes a file it would refuse to open. The editor opens with no automatic send and no link to a captured request.

**Learn more:** [A readable file can still be the wrong request](../chapters/20-remembering-a-request.md#a-readable-file-can-still-be-the-wrong-request) · [`decode`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L139-L148) · [`to_draft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L74-L121)

</details>
