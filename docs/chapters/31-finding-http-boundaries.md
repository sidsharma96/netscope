# Where does an HTTP body end?

A plaintext response contains a book description, followed by another response on the same connection. Reading every remaining byte as the first body would consume the second message too. Reading one TCP packet as one body would fail whenever a packet cuts a message in two. HTTP needs its own framing rules: rules that identify where a message ends.

A HAR often retains an already assembled response body. It normally does not retain the original HTTP/1.1 chunk delimiters. Packet-derived plaintext can supply those bytes, but only when reassembly has a usable [contiguous prefix](../GLOSSARY.md#contiguous-prefix). Missing or conflicting TCP positions stop this interpretation before it guesses a later message start.

## Packet boundaries and message boundaries have different jobs

TCP divides a byte stream for transport. HTTP organizes that stream into messages. One segment can contain half a header, several body chunks, or the end of one response and the beginning of another. Neither layer promises that its boundaries match the other's.

This illustration shows a chunk crossing a packet boundary. The vertical cut is an invented capture split. Concatenating supported contiguous sequence ranges restores the bytes needed to read the chunk.

```text
HTTP chunk:      size line | abcdef | ending
packet pieces:   size line | ab     || cdef | ending
                                      ^ transport split
```

The split does not change a six-byte chunk into a two-byte chunk and a four-byte chunk. Conversely, one large packet does not turn several HTTP chunks into one. This is why Netscope derives chunk ranges from the wire grammar rather than creating them from retained body slices.

## A length and a delimiter answer different questions

A [Content-Length](../GLOSSARY.md#content-length) field declares the number of body bytes. Three means three bytes, not necessarily three visible characters; UTF-8 characters can occupy more than one byte. If only two declared bytes are present, the boundary remains unavailable. Later-looking text cannot be assumed to start another message until the current boundary is resolved.

With [chunked transfer encoding](../GLOSSARY.md#chunked-transfer-coding), each chunk begins with a hexadecimal size line. CRLF, the two-byte carriage-return/line-feed pair, ends that line. The declared number of data bytes follows, then another CRLF. A zero-sized chunk starts the ending section, which may contain trailer fields before a final empty line. The [HTTP/1.1 framing rules](https://www.rfc-editor.org/rfc/rfc9112#section-7.1) define that grammar.

Here is an illustrative wire representation. Each visible `\r\n` means those two bytes; it is not literal backslash text.

```text
3\r\nabc\r\n
2\r\nde\r\n
0\r\nX-End: yes\r\n\r\n
```

The application body becomes `abcde`. The size lines, data terminators and trailer belong to framing, so they are not appended to that body. The trailer remains recorded metadata. Stopping immediately after reading the zero line would miss whether its ending section is complete.

A chunk size line can carry extension text after a semicolon. Netscope retains printable extension text for inspection but does not validate every extension's grammar or meaning. RFC 9112 allows optional spaces or tabs between the hexadecimal digits and the semicolon (`3 ;ext`), and Netscope accepts them there. Whitespace after a size with no extension (`3 `) is not in the grammar, so Netscope reports that line as malformed. It also does not decompress gzip or Brotli after removing chunk framing. Transfer framing and content encoding are separate steps.

## Check the rule with a small byte example

The `b` before a quoted string makes a byte string in Rust. Escapes such as `\r` and `\n` place the actual control bytes in that input. This small example asks the bounded framing inspector about one complete chunked candidate.

```rust
use netscope_core::wire_http::inspect_http;
let wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n";
let found = inspect_http(wire, false);
assert_eq!(found.messages[0].body, b"abc");
assert!(found.messages[0].framed);
```

The second argument is false because this illustration does not supply a transport-close observation. Chunked framing can still finish from its explicit delimiters. The first assertion checks the application bytes after framing is removed; the second checks that the supported terminator was present. Neither assertion establishes that a real receiver accepted a response.

## Some apparent boundaries are ambiguous

Netscope's initial subset deliberately refuses [Transfer-Encoding](../GLOSSARY.md#transfer-encoding) together with Content-Length, repeated lengths, and unsupported transfer-coding combinations. The [message body length rules](https://www.rfc-editor.org/rfc/rfc9112#section-6.3) say what a receiver does instead: Transfer-Encoding overrides Content-Length, and a message carrying both "ought to be handled as an error" because the pair is a classic request-smuggling setup. Two parsers that disagree about which header wins disagree about where the next message starts. An inspector that picks one answer can show a boundary the real receiver never used, so Netscope labels the message `Ambiguous` and stops.

The same rules put status codes first. A 1xx, 204, or 304 response has no body whatever its headers say. Netscope applies that status rule before it compares Transfer-Encoding with Content-Length, so a 304 that carries both headers ends at its empty line and is not labelled ambiguous.

Response meaning can also depend on the corresponding request. A response to HEAD does not carry an ordinary response body even when Content-Length describes the representation's size. The packet inspector does not pair opposite directions or reconstruct all request context, so its response framing is explicitly conditional. A concrete consequence: if a HEAD response says `Content-Length: 500`, Netscope waits for 500 body bytes, and on a reused connection it reads the start of the next response as that body. It supports GET/POST request candidates, not a complete HTTP conversation validator.

Some status codes define a header-only boundary. A 101 upgrade instead changes the protocol; later bytes cannot be treated as another HTTP response. A response without explicit length may depend on an observed connection end. Netscope uses that interpretation only when its supported direction has an unambiguous observed SYN-to-FIN span. An arbitrary capture ending is insufficient.

## Preserve useful evidence when interpretation stops

An unfinished chunk does not erase earlier complete chunks. Their size-line and data ranges remain useful. A malformed terminator, missing trailer ending, inspection limit, or unknown framing gets a distinct explanation. The underlying packet and sequence bytes remain separate from the application-body preview.

Try removing the final empty line from the illustrative message. The body bytes `abcde` may already be available, while its framing remains unfinished. Then place a TCP gap inside a size line. No later chunk boundary should be invented. The wire inspector [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140), prefix-cut tests [`every_chunked_prefix_keeps_absence_distinct_from_termination`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/wire_http.rs#L16), and [local walkthrough](../DEVELOPMENT.md#inspect-packets-and-streams) let you check those predictions.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Chunked bodies

**Level:** Newcomer


How does a chunked HTTP/1.1 response mark where its body ends, and which bytes become the body?

<details>
<summary>Answer</summary>

Each chunk starts with its size in hexadecimal and a CRLF, then that many data bytes, then another CRLF. A chunk of size zero starts the ending section, which may hold trailer fields and ends with an empty line. Only the data bytes form the body. The size lines, CRLFs and trailers are framing. Packet boundaries do not matter, because Netscope reads chunks from the reassembled byte stream, not from individual packets.

**Learn more:** [A length and a delimiter answer different questions](../chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions) · [Packet boundaries and message boundaries have different jobs](../chapters/31-finding-http-boundaries.md#packet-boundaries-and-message-boundaries-have-different-jobs) · [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140)

</details>

### What does Netscope show for HTTPS in a PCAP?

**Level:** Practitioner


You open a PCAP of HTTPS traffic to port 443. What does Netscope show about the TLS handshake (ClientHello, server name, certificate), and what does it show about the HTTP inside it?

<details>
<summary>Answer</summary>

Nothing specific to TLS. Netscope has no TLS record or handshake parser for packet captures, so it does not extract the server name (SNI), list cipher suites or show certificates. The port 443 stream is reassembled like any other TCP stream, with its byte states and SYN, FIN and RST flags, and the packet view shows the layers and the bytes.

When the HTTP/1.1 parser examines that stream, it stops with "No supported GET/POST or HTTP/1.1 response start at prefix", because TLS records begin with a record-type byte such as 0x16 or 0x17, not with a request line. The HTTP inside is not visible: Netscope does not decrypt traffic and does not accept key-log files. TLS appears elsewhere in Netscope only as the HAR `ssl` timing and as the rustls client that replay uses.

**Common wrong answer.** "It shows the SNI from the ClientHello." No code in Netscope parses TLS handshake messages from a capture.

**Learn more:** [TLS: the handshake, SNI, ALPN and certificate checks](../chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks) · [Plaintext HTTP/1.1 subset](../contracts/m8-streams.md#plaintext-http11-subset) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186-L214)

</details>

### A capture taken with a 96-byte snap length

**Level:** Expert


A capture was recorded with a snap length of 96 bytes, so every TCP payload is cut. What do the reassembled byte states look like, what does `contiguous_prefix` return, and what does the HTTP framing inspector see?

<details>
<summary>Answer</summary>

Each record keeps 96 bytes. After a 14-byte Ethernet header, a 20-byte IPv4 header, and a 20 to 32-byte TCP header, between about 30 and 42 payload bytes remain. The importer adds the layer note "Capture cut: fewer bytes retained than original packet".

The segment's declared payload comes from the IP length field, not from the captured length, so reassembly reserves the full sequence range the sender used. Only the retained bytes are written. The rest of each segment's range stays Missing. The stream becomes a run of Observed bytes, then a run of Missing positions, then Observed again for the next segment, and so on.

`contiguous_prefix` stops at the first Missing position, inside the first data segment. The HTTP inspector therefore sees a few dozen bytes. It usually stops with "No complete HTTP/1.1 start line in prefix" or a header-line error. It never joins the retained fragments across the holes, and a Missing position is never read as a zero byte.

**Learn more:** [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`retained`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L157-L161) · [`declared_payload`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L342) · [`contiguous_prefix`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L53-L60)

</details>

### In what order does the wire HTTP parser apply framing rules?

**Level:** Practitioner


Which bytes does the HTTP/1.1 parser receive from a reassembled stream, and in what order does it apply the framing rules (Transfer-Encoding, Content-Length, status codes, a request without framing, a close-delimited response)? What limits apply?

<details>
<summary>Answer</summary>

The parser receives only the contiguous prefix of one direction, cut to 256 KiB, together with a flag saying whether the stream's end was observed. It parses up to eight messages and stops at the first one it cannot frame, without guessing a later one. A start line must be `HTTP/1.1` with a three-digit status from 100 to 599, or exactly `GET` or `POST`, a target and `HTTP/1.1`. Header lines may be at most 4 KiB, the header section 32 KiB, and the count 100 fields.

The rules then apply in order. First, a 1xx, 204 or 304 status ends at the headers whatever its length headers say, except that 101 changes protocol and stops parsing. Second, Transfer-Encoding and Content-Length together are reported as ambiguous and left unframed, because that combination is the classic request-smuggling pattern. Third, Transfer-Encoding must be exactly one `chunked`, with at most 1024 chunks. Fourth, Content-Length must be one decimal value up to 256 KiB. Fifth, a request with neither has no body. Last, a response with neither is close-delimited, and counts as framed only when the SYN-to-FIN span was observed without gaps.

**Learn more:** [A length and a delimiter answer different questions](../chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions) · [Some apparent boundaries are ambiguous](../chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous) · [Why refuse Transfer-Encoding together with Content-Length?](../chapters/34-defending-the-design.md#why-refuse-transfer-encoding-together-with-content-length) · [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140-L185) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186-L360)

</details>

### Inputs the wire HTTP parser cannot frame

**Level:** Expert


Give concrete inputs on which the wire HTTP framing inspector stops or misframes. Consider a response to HEAD, `101 Switching Protocols`, a PUT request, an HTTP/1.0 response, two identical `Content-Length` headers, and `Transfer-Encoding: gzip, chunked`.

<details>
<summary>Answer</summary>

The inspector reads each direction alone and never pairs a response with its request. That makes HEAD the most important case. A response to HEAD that carries `Content-Length: 1234` is framed as if 1234 body bytes follow. On a reused connection the next response's bytes are taken as that body, or the message stays unframed while it waits.

A `101` response is labelled "101 changes protocol; later HTTP framing unavailable", and inspection stops, because the later bytes belong to another protocol. A PUT request, or any method other than GET and POST, stops with "No supported GET/POST or HTTP/1.1 response start at prefix". An `HTTP/1.0` status line fails the same check, because only `HTTP/1.1` is accepted as a response start.

Two identical `Content-Length: 5` headers give "Content-Length outside supported single decimal subset". RFC 9110 allows repeated identical values, so this is stricter than the standard. `Transfer-Encoding: gzip, chunked` gives "Transfer coding outside supported single chunked subset". Obsolete line folding and whitespace before the colon fail as invalid header names.

**Learn more:** [How can a response be framed without seeing its request?](../chapters/34-defending-the-design.md#how-can-a-response-be-framed-without-seeing-its-request) · [`message`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L186) · [`lengths`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L325-L329) · [`transfer`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L248-L251)

</details>
