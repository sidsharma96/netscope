# Implementation limitations

These notes describe current behavior for contributors investigating a failure or planning a change. The [capstone](chapters/34-defending-the-design.md) teaches how to reason about the consequences. Supported formats and fixed limits belong in the [behavior references](contracts/overview.md).

## Packet and message interpretation

Packet analysis keeps the two directions of a TCP connection separate. It does not associate an HTTP response with its request. A HEAD response can therefore be treated as if its declared Content-Length were a body length, consuming bytes from a subsequent response. CONNECT also needs request context the analysis does not supply. See [HTTP boundaries](chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous).

A new connection whose opening SYN was not captured can be grouped with earlier traffic using the same addresses and ports. The stream is a grouping under the current rules, not a verified complete connection history.

Reassembly ignores a differing one-byte segment at the last placed position when it fits a possible keep-alive probe. This avoids treating some probes as application data, but the pattern does not establish the sender's intent. A genuine conflicting byte with that shape may be excluded. The original packet remains inspectable. [`reassemble`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L89) implements the rule.

The chunk reader uses the same missing-line result when a size line has no ending and when it exceeds its limit. The display can consequently say it is waiting for a complete terminator when the real reason is an overlong line. [`line`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L45) is the shared check.

## Response waiting and trust

Live SSE reparses its accumulated text and copies the accumulated state as pieces arrive. Many very small pieces repeat work over the same bytes. The amount retained and the operation duration are limited, but that does not make this approach efficient for arbitrary streams. An incremental parser would process new bytes while retaining enough state to complete a split character, line, or event. See [live observation](chapters/27-observing-live-events.md#the-screen-needs-the-latest-observation-not-every-drawing-request).

TLS uses bundled trusted roots, with no configured certificate revocation checks or operating-system trust store. Connections try resolved addresses sequentially. An unreachable first address can consume the deadline even when another address would work. The [transport reference](contracts/replay-transport.md) describes the client configuration.

A worker panic produces a fallback result, but its diagnostic can print over the running terminal interface. The terminal-restoration hook applies to the thread that owns the terminal; worker diagnostics need a separate presentation path.

## Display and saved files

The terminal escape function handles control characters, including ESC. It does not escape every Unicode formatting character. Direction-changing or invisible characters can therefore make text appear different from its stored sequence. This is a display limitation to consider when inspecting an unfamiliar URL. See [`terminal_text`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L285).

Saved drafts omit known credential headers but retain the URL and body. Other header names can still carry secrets. Saving through a synchronized temporary file protects against ordinary partial-write failures, but the parent directory is not synchronized after renaming. A power loss can affect whether that rename persists. See [saving a request](chapters/20-remembering-a-request.md).

## Construction and resource use

Some public model constructors assume validated input. A request ID does not identify the session it came from, and a caller can construct a status outside the importer's accepted range. The host summary does not count such an invalid status. Import tests establish the importer path's behavior; they do not make arbitrary public construction valid.

Saved drafts use a special numeric request ID to mean that there is no captured source. An explicit optional source would express that absence more clearly. This is separate from how IDs identify real imported requests.

A source-file limit is not a measured maximum memory footprint. Import can retain the input buffer, copied packet bytes, reassembly state, and descriptions together. Performance work should measure representative and adversarial inputs, naming the build, machine, and procedure. The existing [large-capture observations](LARGE_CAPTURES.md) concern HAR input and do not establish a PCAP memory peak.
