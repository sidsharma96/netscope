# Glossary

This page defines the terms the book uses that a newcomer might not know. Each entry gives the meaning used in this book and links to the section that teaches it. Chapters link a term here the first time they use it, where that helps.

Terms are in alphabetical order. A Rust type or keyword appears as it is written in code.

## ALPN

Application-Layer Protocol Negotiation, a TLS extension in which the client lists the application protocols it can speak, such as `h2` and `http/1.1`, and the server picks one. Netscope's replay offers only `http/1.1`. See [TLS: the handshake, SNI, ALPN, and certificate checks](chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks).

## Alternate screen

A separate terminal display that a full-screen program switches to while it runs. When the program leaves it, the shell's earlier output reappears unchanged. See [Leave the terminal usable](chapters/11-drawing-and-navigation.md#leave-the-terminal-usable).

## Arc

Rust's atomically reference-counted shared owner, written `Arc<T>`. Cloning an `Arc` adds another owner of the same value; it does not let several threads change that value safely. See [Share lifetime only when lifetime is the problem](chapters/24-sharing-work.md#share-lifetime-only-when-lifetime-is-the-problem).

## Async function

A function declared with `async fn`. Calling it creates a future and does not run the function's body until something polls that future. See [A future holds unfinished work](chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work).

## Atomic operation

An operation on a shared value that other threads see as indivisible, such as `fetch_add` on an `AtomicU64`. It protects that one value, not other data the program shares. See [Update one counter without losing an increment](chapters/24-sharing-work.md#update-one-counter-without-losing-an-increment).

## Atomic replacement

Writing a complete temporary file beside the destination, synchronizing it, and then renaming it over the destination. A reader sees either the old file or the new one, never half of each. Netscope saves drafts this way. See [Replacing a file has a vulnerable middle](chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle).

## Await

The `.await` expression, which lets one future wait for another. If the inner future is not ready, the outer one pauses and keeps the state it needs to resume at the same point. See [Awaiting keeps a place to continue](chapters/17-waiting-for-a-response.md#awaiting-keeps-a-place-to-continue).

## Base64

A text encoding that represents any bytes using 64 printable characters. A HAR file uses it for response bodies that are not plain text, and Netscope decodes it only up to a size limit. See [Decide when a failure blocks the request](chapters/10-importing-a-har.md#decide-when-a-failure-blocks-the-request).

## Binary search

Finding a position in a sorted list by repeatedly halving the range that could contain it. Redirect and retry matching use Rust's `partition_point` to find the first request that starts late enough. See [Why is binary search valid in redirect and retry matching?](chapters/34-defending-the-design.md#why-is-binary-search-valid-in-redirect-and-retry-matching).

## Body

The bytes that follow an HTTP message's headers. A HAR may record a body, omit it, or store it in a different form from the bytes that crossed the network. See [An empty body is still information](chapters/02-reading-a-capture.md#an-empty-body-is-still-information).

## BOM

A byte-order mark, the character U+FEFF placed at the start of a text. Netscope removes one leading BOM before reading HAR JSON or event-stream text. See [Keep small differences visible](chapters/25-reading-event-streams.md#keep-small-differences-visible).

## Boolean

A value that is either `false` or `true`, written `bool` in Rust. A comparison such as "do these two header lists match?" produces one. See [Turn a comparison into a Boolean value](chapters/14-recognizing-repeats.md#turn-a-comparison-into-a-boolean-value).

## Borrow

A temporary reference to a value that another variable owns. A shared borrow (`&T`) can read the value; a mutable borrow (`&mut T`) can change it, and Rust allows only one mutable borrow at a time. See [Borrow while drawing](chapters/03-storing-requests.md#borrow-while-drawing).

## Byte order

The order in which a number's bytes are stored. Big-endian puts the most significant byte first, as network headers do; little-endian puts it last. A PCAP file's magic number says which order its own header fields use. See [Two bytes need an agreed order](chapters/29-reading-packet-bytes.md#two-bytes-need-an-agreed-order).

## Candidate

A later request that is consistent with a relationship, such as a redirect follow-up or a retry, without proof that the client sent it for that reason. Netscope lists at most 20 candidates and says when there are more. See [Two plausible requests are still two requests](chapters/13-following-redirects.md#two-plausible-requests-are-still-two-requests).

## Capture

A recording of network activity. In this book it is either a HAR file exported by a browser or a PCAP file of packets. See [What a capture can tell us](chapters/02-reading-a-capture.md).

## Cargo

Rust's build tool and package manager. `cargo build` compiles the workspace, and `cargo install` places the finished program in Cargo's bin directory. See [Building creates another file](chapters/22-running-a-program.md#building-creates-another-file).

## Certificate

A signed document that binds a public key to names such as a hostname. A TLS client accepts a server only if its certificate chains to a trusted root and names the host the client connected to. See [TLS: the handshake, SNI, ALPN, and certificate checks](chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks).

## Channel

A way to pass values from one thread to another, with a sending end and a receiving end. Netscope's replay worker returns its one final result through a channel with room for one value. See [Hand the value over when responsibility changes](chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes).

## Chunked transfer coding

An HTTP/1.1 way to frame a body as a series of chunks, each preceded by its size in hexadecimal and ended by a zero-size chunk. It lets a sender start before it knows the total length. See [A length and a delimiter answer different questions](chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions).

## Clippy

Rust's linting tool. Netscope's checks run it with warnings treated as errors. See [Use linting for recurring code concerns](chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns).

## Clone

Making an independent copy of a value by calling `clone()`. Cloning a `String` allocates a new buffer and copies the text into it. See [A string owns its bytes](chapters/03-storing-requests.md#a-string-owns-its-bytes).

## Close-delimited body

A response body with neither a `Content-Length` nor chunked coding, which ends when the server closes the connection. Netscope accepts that ending only when it observed the whole direction from SYN to FIN with nothing missing. See [How HTTP/1.1 marks where a message ends](chapters/34-defending-the-design.md#how-http11-marks-where-a-message-ends).

## Closure

A function-like value written where it is used, which can refer to variables from the surrounding code. The terminal drawing library takes a closure each time it draws a frame. See [Repeat a small loop](chapters/11-drawing-and-navigation.md#repeat-a-small-loop).

## Compare-and-swap

An atomic operation that changes a value only if it still holds the value the caller expected. Rust calls it `compare_exchange`. Netscope does not use it. See [Change a value only if the assumption still holds](chapters/24-sharing-work.md#change-a-value-only-if-the-assumption-still-holds).

## Compiler

The program that checks Rust source code against the language's rules and turns it into an executable file. Rust's compiler is `rustc`, which Cargo runs for you. See [Building creates another file](chapters/22-running-a-program.md#building-creates-another-file).

## Compression pointer

In a DNS message, a two-byte field whose top two bits are set and whose other 14 bits give the offset where the rest of a name continues. A hostile packet can make pointers loop; Netscope stops after 64 label or pointer steps. See [How DNS compresses names](chapters/34-defending-the-design.md#how-dns-compresses-names).

## Conflict

A position in a reassembled TCP direction that two segments recorded with different byte values. Netscope marks the position and leaves it out of HTTP analysis instead of choosing one copy. See [Put bytes at positions before reading their meaning](chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning).

## Content negotiation

The exchange in which a client lists the forms it accepts, in headers such as `Accept` and `Accept-Encoding`, and the server picks one representation of the resource. See [Content negotiation and Accept-Encoding](chapters/34-defending-the-design.md#content-negotiation-and-accept-encoding).

## Content-Encoding

A response header naming a compression, such as `gzip` or `br`, applied to the representation itself. Replay asks for `identity`, meaning no encoding, so the bytes it receives are the bytes it shows. See [Content negotiation and Accept-Encoding](chapters/34-defending-the-design.md#content-negotiation-and-accept-encoding).

## Content-Length

An HTTP header that gives a body's length in bytes. The receiver reads exactly that many bytes after the headers. See [A length and a delimiter answer different questions](chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions).

## Content-Type

An HTTP header naming the media type of a body, such as `application/json` or `text/event-stream`. It tells the receiver how to interpret the bytes. See [Content type gives the server an interpretation](chapters/19-submitting-text.md#content-type-gives-the-server-an-interpretation).

## Contiguous prefix

The bytes of a reassembled TCP direction from its first position up to the first missing or conflicting position. Netscope's HTTP inspection reads only this prefix, so it never joins bytes across a hole. See [Put bytes at positions before reading their meaning](chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning).

## Cookie

A name and value that a server asks a browser to store and send back on later requests, often to identify a session. Netscope leaves cookie headers out of saved drafts. See [An address is only part of a request](chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request).

## Crate

One Rust library or program, the unit the compiler builds. Netscope's workspace has four crates: core, import, net, and tui. See [Make dependencies point toward the data rules](chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules).

## Datagram

A single UDP message with its own recorded length. Netscope reads each datagram on its own and never joins two into a stream. See [A datagram has its own boundary](chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary).

## Deadline

A fixed point in time after which an operation stops. Netscope sets one deadline when Send is accepted, 10 seconds for a replay and 30 seconds for live SSE, and progress never extends it. See [Keep one deadline for the whole request](chapters/17-waiting-for-a-response.md#keep-one-deadline-for-the-whole-request).

## Deadlock

A state in which two threads each hold a lock and wait for the lock the other holds, so neither can continue. See [Protect several related changes together](chapters/24-sharing-work.md#protect-several-related-changes-together).

## Deterministic test

A test that produces the same observable result every time it runs on the same controlled inputs, with no dependence on the clock, the network, or chance. See [Control the inputs that affect the answer](chapters/04-quality-and-tests.md#control-the-inputs-that-affect-the-answer).

## Directed epoch

Netscope's unit of TCP reassembly: one direction, from a source socket to a destination socket, during one connection lifetime. A SYN with a new initial sequence number, or a SYN after FIN or RST, starts a new epoch. See [A direction needs a beginning, and sometimes we do not have it](chapters/30-putting-bytes-in-order.md#a-direction-needs-a-beginning-and-sometimes-we-do-not-have-it).

## DNS

The Domain Name System, which turns a name such as `example.test` into network addresses. A HAR records how long the lookup took, and Netscope reads DNS answers from UDP datagrams on port 53 in a PCAP. See [Missing time is different from zero](chapters/02-reading-a-capture.md#missing-time-is-different-from-zero).

## Documentation test

A code example in documentation that the test suite compiles and, unless marked otherwise, runs. Several chapters of this book are included in the crates' documentation, so their Rust examples are checked this way. See [Use linting for recurring code concerns](chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns).

## Draft

An editable copy of a request, prepared for replay. Changing a draft never changes the recorded request it came from. See [Give the draft its own text](chapters/16-preparing-a-request.md#give-the-draft-its-own-text).

## Drop

The cleanup Rust runs when a value's owner goes out of scope. A type can define its own cleanup by implementing the `Drop` trait, as Netscope's terminal guard does. See [Leave the terminal usable](chapters/11-drawing-and-navigation.md#leave-the-terminal-usable).

## DTLS

Datagram Transport Layer Security, the form of TLS that runs over UDP datagrams. A WebRTC session description can carry a DTLS certificate fingerprint. See [Recognize a description without inventing its outcome](chapters/32-reading-signaling.md#recognize-a-description-without-inventing-its-outcome).

## Enum

A type whose value is exactly one of several named variants, each of which can carry data. `Option`, `Result`, and Netscope's `Timing` are enums. See [Represent the possibility of absence](chapters/09-options-results-and-matching.md#represent-the-possibility-of-absence).

## Environment variable

A named value that a process receives from the process that started it, such as `PATH`. See [The shell has to find the executable](chapters/22-running-a-program.md#the-shell-has-to-find-the-executable).

## Ethernet

The most common local network link. In a PCAP with link type 1, every frame starts with a 14-byte Ethernet header that names the next layer, such as IPv4 or IPv6. See [A boundary belongs to a particular layer](chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer).

## Executable

A file the operating system can start as a program. Building Netscope produces the `netscope` executable from its source code. See [Building creates another file](chapters/22-running-a-program.md#building-creates-another-file).

## Exporter

The tool that wrote a HAR file, usually a browser's developer tools. Different exporters fill optional fields differently, so Netscope reports what the exporter wrote rather than what it might have meant. See [Missing time is different from zero](chapters/02-reading-a-capture.md#missing-time-is-different-from-zero).

## FIN

The TCP flag that ends one direction's data. It uses one sequence number but carries no application byte. See [Sequence numbers eventually wrap](chapters/30-putting-bytes-in-order.md#sequence-numbers-eventually-wrap).

## Fixture

A saved test input with a known purpose, such as a small HAR or PCAP file under `tests/fixtures`. See [A test makes an expectation precise](chapters/04-quality-and-tests.md#a-test-makes-an-expectation-precise).

## Flow control

The HTTP/2 rule that limits how much body data a sender may send before the receiver grants more. Limits apply to each stream and to the whole connection. See [Sharing also needs limits](chapters/28-sharing-a-connection.md#sharing-also-needs-limits).

## Frame

In HTTP/2, a typed piece of a connection's traffic, such as HEADERS or DATA, labelled with the stream it belongs to. See [A label tells the receiver where a piece belongs](chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs).

## fsync

The operating-system call that forces a file's written data to storage. Rust exposes it as `File::sync_all`; Netscope calls it before renaming a saved draft into place. See [Replacing a file has a vulnerable middle](chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle).

## Future

A Rust value that represents an asynchronous computation that may not have finished. It makes progress only when polled, and dropping it stops the work. See [A future holds unfinished work](chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work).

## Gap

A run of positions in a reassembled TCP direction for which no segment supplied bytes. Netscope records a gap as missing and never fills it with zeros. See [Remember absence separately from the value](chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value).

## Grapheme cluster

A group of Unicode values that a reader sees as one character, such as `e` followed by a combining accent. Netscope's text editor moves and deletes by grapheme cluster. See [One visible letter can have two parts](chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts).

## Guard

A value whose job is cleanup: when it is dropped, it undoes something. Netscope's terminal guard restores the terminal's settings even after an error. See [Leave the terminal usable](chapters/11-drawing-and-navigation.md#leave-the-terminal-usable).

## HAR

HTTP Archive, a JSON file format that browsers export to record the requests and responses of a page load, with headers, bodies, and timings. Netscope reads HAR version 1.2. See [One request to follow](chapters/01-start-here.md#one-request-to-follow).

## Header

A named field in an HTTP message, such as `Content-Type: application/json`. Headers come after the start line and before the body. See [An address is only part of a request](chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request).

## Host

The name or address part of a URL, such as `example.test`. Netscope groups requests by host, and the grouping key leaves out the port. See [Choose what makes a group](chapters/12-grouping-requests.md#choose-what-makes-a-group).

## HPACK

HTTP/2's header compression format. It replaces repeated header fields with references to a static table and to a dynamic table that both sides update. See [Repeated headers can use remembered values](chapters/28-sharing-a-connection.md#repeated-headers-can-use-remembered-values).

## HTTP

Hypertext Transfer Protocol, the request and response protocol a browser uses to fetch web resources. A request names a method and a URL; a response carries a status, headers, and usually a body. See [One request to follow](chapters/01-start-here.md#one-request-to-follow).

## HTTP/1.1

The text-based version of HTTP in which each message is a start line, header lines, a blank line, and an optional body, sent in order on one connection. Netscope's replay and its packet framing inspector speak only HTTP/1.1. See [A message and the way it travels](chapters/15-reading-protocol-labels.md#a-message-and-the-way-it-travels).

## HTTP/2

A binary version of HTTP that splits messages into frames and carries many requests at once on one connection. Netscope reads HTTP/2 labels in a HAR but does not send or decode HTTP/2. See [A label tells the receiver where a piece belongs](chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs).

## Hyper

The Rust HTTP library that Netscope's replay uses to write a request and read the response on one HTTP/1.1 connection. See [Why let a library interpret HTTP?](chapters/17-waiting-for-a-response.md#why-let-a-library-interpret-http).

## ICE

Interactive Connectivity Establishment, the process WebRTC peers use to test candidate network paths to each other. Seeing ICE fields in a description does not show that a check succeeded. See [Recognize a description without inventing its outcome](chapters/32-reading-signaling.md#recognize-a-description-without-inventing-its-outcome).

## Idempotent

An HTTP property of a request: sending it twice has the same intended effect on the server as sending it once. It does not promise identical responses. See [Decide when to look for a repeat](chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat).

## Initial sequence number

The sequence number a TCP sender chooses for its SYN, often shortened to ISN. The first data byte is numbered ISN + 1. See [How TCP numbers its bytes](chapters/34-defending-the-design.md#how-tcp-numbers-its-bytes).

## IP

Internet Protocol, the layer that carries packets between addresses. IPv4 uses 32-bit addresses and IPv6 uses 128-bit addresses; each header states the packet's length and the transport protocol inside, such as TCP or UDP. See [A boundary belongs to a particular layer](chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer).

## Iterator

A value that produces a sequence of items one at a time, such as the characters of a string from `chars()`. See [A byte position needs a rule](chapters/21-moving-through-text.md#a-byte-position-needs-a-rule).

## JSON

JavaScript Object Notation, a text format for named fields, lists, text, numbers, `true`, `false`, and `null`. HAR files and saved drafts are JSON. See [One request to follow](chapters/01-start-here.md#one-request-to-follow).

## Link type

The number in a PCAP file header that says what the first bytes of every record are. Netscope reads type 1 (Ethernet), 101 (raw IP), 228 (raw IPv4), and 229 (raw IPv6). See [What a capture file contains](chapters/34-defending-the-design.md#what-a-capture-file-contains).

## Lint

An automated check for a code pattern that deserves attention, such as an ignored error. Clippy is Rust's lint tool. See [Use linting for recurring code concerns](chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns).

## Location

The response header that names a redirect's destination. Netscope requires exactly one `Location` header before it looks for a follow-up request. See [Read the instruction before finding a follow-up](chapters/13-following-redirects.md#read-the-instruction-before-finding-a-follow-up).

## Loopback

A network interface that delivers traffic back to the same machine, such as `lo0` on macOS. macOS loopback captures use link type 0, which Netscope does not read. See [Why refuse PCAPNG and other link types](chapters/34-defending-the-design.md#why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do).

## Macro

Rust code that writes other code when the program is compiled, called with an exclamation mark, such as `vec!` or `format!`. See [Keep several values in a vector](chapters/08-structs-vectors-and-ids.md#keep-several-values-in-a-vector).

## Magic number

A fixed value at the start of a file that identifies its format. A classic PCAP magic number also reveals the file's byte order and timestamp resolution. See [What a capture file contains](chapters/34-defending-the-design.md#what-a-capture-file-contains).

## Match

Rust's expression for handling each variant of an enum. The compiler rejects a `match` that leaves a variant unhandled. See [Handle the alternatives with match](chapters/09-options-results-and-matching.md#handle-the-alternatives-with-match).

## Media type

A name for a body's format, such as `text/plain` or `application/json`, also called a MIME type. It appears in the `Content-Type` header. See [Content type gives the server an interpretation](chapters/19-submitting-text.md#content-type-gives-the-server-an-interpretation).

## Monotonic clock

A clock that counts elapsed time from an arbitrary start and never goes backwards, unlike the wall clock. Rust's `Instant` is monotonic, and Netscope's deadlines and live event times use it. See [A clock needs a named starting point](chapters/27-observing-live-events.md#a-clock-needs-a-named-starting-point).

## Move

Transferring ownership of a value from one variable to another. After a move, the old name can no longer be used, and the data itself is not copied. See [A string owns its bytes](chapters/03-storing-requests.md#a-string-owns-its-bytes).

## Multiplexing

Carrying several independent exchanges interleaved on one connection, as HTTP/2 does with streams. See [A label tells the receiver where a piece belongs](chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs).

## Mutability

Whether a value may change after it is created. Rust variables cannot change unless declared with `mut`. See [Make changes explicit](chapters/07-values-and-functions.md#make-changes-explicit).

## Mutex

Short for mutual exclusion, a lock that lets only one thread at a time use the data it protects. Rust returns a guard that releases the lock when dropped. See [Protect several related changes together](chapters/24-sharing-work.md#protect-several-related-changes-together).

## Newtype

A struct that wraps one value to give it a distinct type, such as `RequestId(usize)`. The compiler then refuses to use a host ID where a request ID is expected. See [Give different IDs different types](chapters/08-structs-vectors-and-ids.md#give-different-ids-different-types).

## Not-applicable timing

A HAR timing written as `-1`. The HAR specification says this means the stage did not apply to the request, for example a DNS lookup on a reused connection. It is not zero and not unknown. See [Missing time is different from zero](chapters/02-reading-a-capture.md#missing-time-is-different-from-zero).

## Operation ID

The number Netscope gives each accepted Send. A result is accepted only if its ID matches the operation the editor is waiting for, so a late result cannot replace a newer one. See [A late result must not change a newer decision](chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision).

## Option

The Rust enum for a value that may be absent: `Some(value)` or `None`. See [Represent the possibility of absence](chapters/09-options-results-and-matching.md#represent-the-possibility-of-absence).

## Ownership

Rust's rule that every value has one owner responsible for releasing it. When the owner goes out of scope, the value is cleaned up. See [A string owns its bytes](chapters/03-storing-requests.md#a-string-owns-its-bytes).

## Panic

A Rust program's response to a bug it cannot continue past, such as indexing past the end of a vector. It stops ordinary execution of the thread. See [A position may have no element](chapters/08-structs-vectors-and-ids.md#a-position-may-have-no-element).

## PATH

The environment variable that lists the directories a shell searches when you type a command name. See [The shell has to find the executable](chapters/22-running-a-program.md#the-shell-has-to-find-the-executable).

## PCAP

The classic packet capture file format: a 24-byte file header followed by one record per captured frame. Netscope reads classic PCAP version 2.4. See [What a capture file contains](chapters/34-defending-the-design.md#what-a-capture-file-contains).

## PCAPNG

A newer, block-based packet capture format that Wireshark writes by default. Netscope refuses it. Saving the capture as classic PCAP, for example with `editcap -F pcap`, makes it readable if it uses one supported link type. See [Why refuse PCAPNG and other link types](chapters/34-defending-the-design.md#why-refuse-pcapng-and-other-link-types-and-what-must-a-user-do).

## Persistent connection

An HTTP/1.1 connection kept open after a response so the next request can reuse it, also called keep-alive. Netscope's replay opens a new connection for every send. See [Persistent connections and Connection: close](chapters/34-defending-the-design.md#persistent-connections-and-connection-close).

## Poll

To ask a future to do whatever work it can do now. It answers Ready with a result, or Pending if it must wait. See [A future holds unfinished work](chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work).

## Proxy

An intermediary that forwards HTTP requests on a client's behalf. Netscope's replay connects directly to the destination and ignores proxy settings. See [Why let a library interpret HTTP?](chapters/17-waiting-for-a-response.md#why-let-a-library-interpret-http).

## Pseudo-terminal

A program-controlled stand-in for a terminal window. Netscope's terminal checks use one to inspect mode changes without a person's terminal. See [Leave the terminal usable](chapters/11-drawing-and-navigation.md#leave-the-terminal-usable).

## Query string

The part of a URL after `?`, such as `?page=2`, which passes parameters to the server. It can contain secrets such as access tokens. See [An address is only part of a request](chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request).

## Question-mark operator

The `?` after an expression that returns a `Result` or `Option`. On success it gives the inner value; on failure it returns the error from the current function immediately. See [Stop when the next step cannot proceed](chapters/09-options-results-and-matching.md#stop-when-the-next-step-cannot-proceed).

## Ratatui

The Rust library Netscope uses to draw its terminal interface. See [Repeat a small loop](chapters/11-drawing-and-navigation.md#repeat-a-small-loop).

## Raw mode

A terminal setting in which each key reaches the program at once, without waiting for Enter and without being echoed. Netscope turns it on while running and its guard turns it off. See [Leave the terminal usable](chapters/11-drawing-and-navigation.md#leave-the-terminal-usable).

## Reassembly

Rebuilding one direction's TCP byte stream from captured segments by placing each segment's bytes at their sequence positions. See [Put bytes at positions before reading their meaning](chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning).

## Redirect

A response with status 301, 302, 303, 307, or 308 that tells the client to request another URL, named in the `Location` header. See [Read the instruction before finding a follow-up](chapters/13-following-redirects.md#read-the-instruction-before-finding-a-follow-up).

## Replay

Sending a new request built from a reviewed draft. A replay produces a new result shown beside the recording; it never changes the capture. See [Review the current draft, not an earlier one](chapters/16-preparing-a-request.md#review-the-current-draft-not-an-earlier-one).

## Request smuggling

An attack that relies on two HTTP servers disagreeing about where a request ends, for example one using `Content-Length` and the other `Transfer-Encoding`. Netscope refuses to frame a message that carries both, unless a 1xx, 204, or 304 status already rules out a body. See [Some apparent boundaries are ambiguous](chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous).

## RequestId

Netscope's identifier for a captured request: its position in the HAR's list of entries, wrapped in its own type. Sorting and filtering rearrange IDs, never the requests. See [Why store IDs?](chapters/03-storing-requests.md#why-store-ids).

## Result

The Rust enum for an operation that can fail: `Ok(value)` or `Err(error)`. See [Give failure a value too](chapters/09-options-results-and-matching.md#give-failure-a-value-too).

## Retransmission

A TCP sender sending bytes again because it believes they were lost. In a capture it appears as repeated sequence ranges, which can also have other causes. See [Interpreting a trace](chapters/33-interpreting-a-trace.md).

## Retry

Sending a request again after a failure, such as a 503 status. Netscope lists later identical requests as retry candidates without claiming the client intended a retry. See [Start with what repeated](chapters/14-recognizing-repeats.md#start-with-what-repeated).

## Review

The step between editing a draft and sending it, in which Netscope validates the draft and shows the exact request it would send. Nothing is sent without a review followed by an explicit Send. See [Review the current draft, not an earlier one](chapters/16-preparing-a-request.md#review-the-current-draft-not-an-earlier-one).

## RST

The TCP reset flag, which aborts a connection immediately. Netscope will not treat a direction that saw RST as cleanly ended. See [An observed end is a qualified result](chapters/30-putting-bytes-in-order.md#an-observed-end-is-a-qualified-result).

## Runtime

For asynchronous Rust, the library code that polls futures and wakes them when the operating system reports progress. Netscope uses a Tokio runtime inside each replay worker thread. See [A future holds unfinished work](chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work).

## Rustls

The Rust TLS library that Netscope's replay uses for HTTPS connections and certificate checks. See [Why let a library interpret HTTP?](chapters/17-waiting-for-a-response.md#why-let-a-library-interpret-http).

## Scope

The region of code where a name can be used, such as a function body or a block. A value owned by a variable is cleaned up when that variable's scope ends. See [Names have a scope](chapters/07-values-and-functions.md#names-have-a-scope).

## SDP

Session Description Protocol, a text format that describes a proposed real-time session, with lines such as `v=`, `o=`, and `m=`. Netscope recognizes SDP-like bodies but does not validate them. See [Recognize a description without inventing its outcome](chapters/32-reading-signaling.md#recognize-a-description-without-inventing-its-outcome).

## Sec-WebSocket-Accept

The header in which a server answers a WebSocket upgrade with a hash derived from the client's key. Netscope checks only that one non-empty value is present; it does not recompute the hash. See [An agreement has two sides](chapters/26-switching-protocols.md#an-agreement-has-two-sides).

## Segment

One TCP packet's worth of a direction's byte stream, with a header that gives the sequence number of its first byte. See [Putting bytes in order](chapters/30-putting-bytes-in-order.md).

## Sequence number

The 32-bit number TCP gives each byte of a direction's stream. A segment states the number of its first byte, so the receiver can place bytes in order however they arrive. See [Sequence numbers eventually wrap](chapters/30-putting-bytes-in-order.md#sequence-numbers-eventually-wrap).

## Serialization

Turning program values into a format that can be stored or sent, such as JSON. Deserialization reads that format back into values. Netscope uses the Serde library for both. See [Turn values into a representation](chapters/20-remembering-a-request.md#turn-values-into-a-representation).

## Session

Netscope's in-memory form of an imported HAR: every request in file order plus the orders, host groups, and relationship evidence computed from them. See [Build a complete session before displaying it](chapters/10-importing-a-har.md#build-a-complete-session-before-displaying-it).

## Snap length

The largest number of bytes a capture keeps from each frame. A frame longer than the snap length is cut, and its record shows a captured length smaller than its original length. See [What a capture file contains](chapters/34-defending-the-design.md#what-a-capture-file-contains).

## SNI

Server Name Indication, a TLS extension in which the client sends the hostname it wants in plain text so the server can choose the right certificate. See [TLS: the handshake, SNI, ALPN, and certificate checks](chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks).

## SSE

Server-Sent Events, a format in which a server keeps one HTTP response open and writes a series of text events into it, using the `text/event-stream` media type. See [Reading event streams](chapters/25-reading-event-streams.md).

## Status code

The three-digit number in an HTTP response that reports the outcome, such as 200 for success, 302 for a redirect, or 503 when a service is unavailable. See [Begin with an observation](chapters/01-start-here.md#begin-with-an-observation).

## Stream

In HTTP/2, one identified request and response exchange within a connection. It is a different thing from an SSE event stream, which is a text format inside one response. See [Sharing a connection](chapters/28-sharing-a-connection.md).

## String

Rust's owned, growable text type. A `String` owns a buffer of UTF-8 bytes. See [A string owns its bytes](chapters/03-storing-requests.md#a-string-owns-its-bytes).

## String slice

A borrowed view of UTF-8 text, written `&str`. It reads text that some other value owns. See [Read text without taking ownership of it](chapters/15-reading-protocol-labels.md#read-text-without-taking-ownership-of-it).

## Struct

A Rust type that groups named fields into one value, such as a request's method, URL, and status. See [Group one request's fields](chapters/08-structs-vectors-and-ids.md#group-one-requests-fields).

## STUN

Session Traversal Utilities for NAT, a UDP protocol that helps peers discover their public addresses during connection setup. Netscope recognizes STUN messages by their fixed header shape. See [A datagram has its own boundary](chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary).

## Subprotocol

A named set of rules that both sides of a WebSocket agree to use on top of WebSocket itself, chosen in the `Sec-WebSocket-Protocol` header. See [Choosing a language for the conversation](chapters/26-switching-protocols.md#choosing-a-language-for-the-conversation).

## SYN

The TCP flag that opens a direction and carries the initial sequence number. It uses one sequence number, so the first data byte comes one position later. See [Sequence numbers eventually wrap](chapters/30-putting-bytes-in-order.md#sequence-numbers-eventually-wrap).

## TCP

Transmission Control Protocol, which gives each direction of a connection an ordered byte stream on top of IP and resends lost data. See [A message and the way it travels](chapters/15-reading-protocol-labels.md#a-message-and-the-way-it-travels).

## Terminal escape sequence

A run of characters, usually starting with the ESC control character, that a terminal treats as a command, for example to move the cursor or change colours. Netscope shows control characters in captured text as visible escapes so they cannot act on your terminal. See [Hostile input files](chapters/34-defending-the-design.md#hostile-input-files).

## Thread

A sequence of instructions the operating system can schedule independently of others in the same program. Netscope keeps terminal input on one thread and gives each replay its own worker thread. See [Waiting does not need to occupy the terminal](chapters/17-waiting-for-a-response.md#waiting-does-not-need-to-occupy-the-terminal).

## Timing stage

One of the seven intervals a HAR records for a request: blocked, dns, connect, ssl, send, wait, and receive. `ssl` is part of `connect`, so the two must not be added together. See [What a HAR timing records](chapters/34-defending-the-design.md#what-a-har-timing-records).

## TLS

Transport Layer Security, the protocol that encrypts and authenticates a connection, as in HTTPS. Netscope does not decrypt TLS in captures; its replay uses TLS to reach HTTPS servers. See [TLS: the handshake, SNI, ALPN, and certificate checks](chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks).

## Tokio

The asynchronous runtime library that Netscope's replay worker uses to wait for the network, a deadline, and cancellation at the same time. See [A future holds unfinished work](chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work).

## Trait

A named set of capabilities that a type can implement, such as being serialized or being dropped with custom cleanup. See [Turn values into a representation](chapters/20-remembering-a-request.md#turn-values-into-a-representation).

## Transfer-Encoding

An HTTP/1.1 header that names how a body is framed for one hop, normally `chunked`. Netscope accepts only a single `chunked` value. See [Some apparent boundaries are ambiguous](chapters/31-finding-http-boundaries.md#some-apparent-boundaries-are-ambiguous).

## Trust root

A certificate authority that a TLS client trusts without further proof. Netscope's replay trusts only the Mozilla root list built into the program, not the operating system's store. See [TLS: the handshake, SNI, ALPN, and certificate checks](chapters/34-defending-the-design.md#tls-the-handshake-sni-alpn-and-certificate-checks).

## TUI

Terminal user interface, a full-screen interactive program drawn with text inside a terminal. Netscope's interface is a TUI. See [Keeping your place on screen](chapters/11-drawing-and-navigation.md).

## Tuple

A fixed-size group of values whose positions can have different types, written in parentheses, such as `(url, method)`. See [Turn a comparison into a Boolean value](chapters/14-recognizing-repeats.md#turn-a-comparison-into-a-boolean-value).

## Type inference

The compiler working out a value's type from how it is created and used, so you do not have to write it. See [A type can come from context](chapters/07-values-and-functions.md#a-type-can-come-from-context).

## UDP

User Datagram Protocol, which sends individual datagrams with no ordering or resending. DNS and STUN usually travel over UDP. See [A datagram has its own boundary](chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary).

## Unicode scalar value

One Unicode code point, the unit Rust's `char` type holds. A visible character can be made of several of them. See [One visible letter can have two parts](chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts).

## unwrap

A method that takes the value out of an `Option` or `Result` and panics if there is none. Netscope's lint settings deny it. See [Stop when the next step cannot proceed](chapters/09-options-results-and-matching.md#stop-when-the-next-step-cannot-proceed).

## URL

A web address such as `https://example.test/api/books?page=2`: a scheme, a host, an optional port, a path, an optional query string, and an optional fragment after `#`. See [An address is only part of a request](chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request).

## usize

Rust's unsigned integer type for lengths and positions. Its size matches the machine's pointer width. See [Awaiting keeps a place to continue](chapters/17-waiting-for-a-response.md#awaiting-keeps-a-place-to-continue).

## UTF-8

The standard way to store Unicode text as bytes, using one to four bytes per scalar value. Rust strings are always valid UTF-8. See [Text already has a byte representation](chapters/19-submitting-text.md#text-already-has-a-byte-representation).

## Vec

Rust's growable, ordered list type, written `Vec<T>`. Netscope keeps captured requests in a `Vec` in file order. See [Keep several values in a vector](chapters/08-structs-vectors-and-ids.md#keep-several-values-in-a-vector).

## VLAN tag

A four-byte field inserted into an Ethernet header to mark a virtual network. Netscope skips up to two VLAN tags before reading the IP layer. See [What a capture file contains](chapters/34-defending-the-design.md#what-a-capture-file-contains).

## Wall-clock time

The date and time of day as a computer's clock reports it. It can jump when the clock is corrected; HAR and PCAP timestamps are wall-clock readings. See [Wall-clock time and monotonic time](chapters/34-defending-the-design.md#wall-clock-time-and-monotonic-time).

## Watch channel

A channel that keeps only the newest value. Live SSE uses one to publish its latest cumulative snapshot, so the screen never falls behind a queue of updates. See [The screen needs the latest observation](chapters/27-observing-live-events.md#the-screen-needs-the-latest-observation-not-every-drawing-request).

## WebSocket upgrade

An HTTP/1.1 exchange in which the client asks to switch the connection to the WebSocket protocol and the server agrees with status 101. Netscope inspects these upgrade headers but not the messages that follow. See [An agreement has two sides](chapters/26-switching-protocols.md#an-agreement-has-two-sides).

## Worker

The separate thread that performs one replay. It owns the reviewed request and the response bytes, and hands one final result back to the terminal thread. See [Waiting does not need to occupy the terminal](chapters/17-waiting-for-a-response.md#waiting-does-not-need-to-occupy-the-terminal).

## Workspace

A group of related Rust crates that Cargo builds together with one lock file. Netscope's workspace holds its four crates. See [Use linting for recurring code concerns](chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns).

## Wrap-around

What happens when a fixed-size number passes its largest value and continues from zero. TCP sequence numbers wrap after 4,294,967,295, so Netscope compares them with wrapping arithmetic. See [Sequence numbers eventually wrap](chapters/30-putting-bytes-in-order.md#sequence-numbers-eventually-wrap).
