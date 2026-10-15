# Newcomer questions

These questions check the basic ideas: what Netscope does, what a capture records, and the Rust features the early chapters teach. The [practitioner questions](practitioner.md) ask how Netscope works and why it was built that way. The [expert questions](expert.md) ask about failure modes, limits and traps. Try to answer each question before you open it. The answer stays hidden until you click **Answer**, and each one links to the section that teaches it.

### What Netscope is for

**Level:** Newcomer

<!-- chapters: 02-reading-a-capture, 23-shaping-the-program -->

In one or two sentences, what does Netscope do, and what does it never do without an explicit action from you?

<details>
<summary>Answer</summary>

Netscope is a terminal program for inspecting recordings of network traffic. It reads HAR files exported by a browser and classic PCAP packet files, and shows what each recording contains, including what it leaves out. It can also send one new GET or POST request based on a recorded one, but only after you review a separate draft and press Send. Opening a capture or a saved draft never sends anything.

**Learn more:** [An observation and an explanation need different labels](../chapters/02-reading-a-capture.md#an-observation-and-an-explanation-need-different-labels) · [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules)

</details>

### What a HAR file is

**Level:** Newcomer

<!-- chapters: 02-reading-a-capture, 10-importing-a-har -->

What is a HAR file, and who decides what goes into it?

<details>
<summary>Answer</summary>

A HAR (HTTP Archive) file is a JSON document that a browser or another capture tool exports. It lists HTTP requests and their responses, with headers, bodies when the tool kept them, and reported timings. The exporter decides what to record, so a HAR can omit a body or a timing. Netscope reads the file and keeps those gaps visible. It does not fill them in.

**Learn more:** [Reading structure comes before understanding meaning](../chapters/10-importing-a-har.md#reading-structure-comes-before-understanding-meaning) · [What a capture can tell us](../chapters/02-reading-a-capture.md#what-a-capture-can-tell-us)

</details>

### Missing, minus one, and zero

**Level:** Newcomer

<!-- chapters: 02-reading-a-capture, 09-options-results-and-matching, 15-reading-protocol-labels -->

A HAR's DNS timing can be absent, `-1`, or `0`. What does each one mean, and how does Netscope keep them apart?

<details>
<summary>Answer</summary>

An absent field means the exporter recorded nothing. HAR 1.2 defines `-1` for an optional timing as "does not apply to the current request", for example DNS on a connection the browser had already opened. A `0` is a reported duration of zero at the exporter's precision. Netscope stores these as different variants of its `Timing` enum: `Missing`, `Unavailable` (shown as `Not applicable (-1)`), and `Milliseconds(0.0)`. None of them is turned into another.

**Common wrong answer.** "`-1` means the value is unknown, so treat it like zero." It means the stage did not happen for this request, and adding it as zero would invent a measurement.

**Learn more:** [Missing time is different from zero](../chapters/02-reading-a-capture.md#missing-time-is-different-from-zero) · [A present number still needs a meaning](../chapters/09-options-results-and-matching.md#a-present-number-still-needs-a-meaning) · [`Timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L11)

</details>

### Why TLS time is not added to connect time

**Level:** Newcomer

<!-- chapters: 02-reading-a-capture -->

A request reports 30 ms for `connect` and 20 ms for `ssl`. Why is the connection setup not 50 ms?

<details>
<summary>Answer</summary>

HAR counts the TLS handshake as part of the connection time. The 20 ms of `ssl` is already inside the 30 ms of `connect`, so adding them counts that 20 ms twice. Connection setup took 30 ms in total. Netscope shows TLS as a detail of connecting for this reason.

**Learn more:** [Some timings contain other timings](../chapters/02-reading-a-capture.md#some-timings-contain-other-timings)

</details>

### Building, installing and running

**Level:** Newcomer

<!-- chapters: 22-running-a-program -->

You built Netscope, but typing `netscope` in another folder prints "command not found". What are the separate steps between source code and a working command, and which one failed?

<details>
<summary>Answer</summary>

Building turns the Rust source into an executable file in the build directory. Installing copies that executable into an installation directory, such as Cargo's `bin` directory. Running it by its short name requires the shell to find it. The shell searches the directories listed in the PATH environment variable, in order. "Command not found" usually means the installation directory is not on PATH. Running the executable by its full path tells you whether the file itself works.

**Learn more:** [Building creates another file](../chapters/22-running-a-program.md#building-creates-another-file) · [The shell has to find the executable](../chapters/22-running-a-program.md#the-shell-has-to-find-the-executable)

</details>

### Where a relative capture path points

**Level:** Newcomer

<!-- chapters: 22-running-a-program -->

You run `netscope capture.har`. Netscope is installed in one folder and the shell is open in another. Which folder must contain `capture.har`?

<details>
<summary>Answer</summary>

The folder the shell is in, called the working directory. A relative path such as `capture.har` starts there, not next to the executable and not in the source checkout. The installed program also does not load the book or the sample captures from the repository, so you can keep your recordings anywhere and pass their path.

**Learn more:** [The shell has to find the executable](../chapters/22-running-a-program.md#the-shell-has-to-find-the-executable) · [Build tools and runtime needs differ](../chapters/22-running-a-program.md#build-tools-and-runtime-needs-differ)

</details>

### Exit status and version output

**Level:** Newcomer

<!-- chapters: 22-running-a-program -->

What exit statuses does Netscope return, and what does `netscope --version` prove?

<details>
<summary>Answer</summary>

Netscope exits with 0 for success, 1 for a file or runtime failure, and 2 for invalid arguments. A script can therefore tell an unreadable capture from a misspelled option without parsing the error text. `netscope --version` prints the package version compiled into that executable. It does not read the source directory, and two builds with the same version number can still contain different code.

**Learn more:** [Ask the installed program what it is](../chapters/22-running-a-program.md#ask-the-installed-program-what-it-is) · [`main`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/main.rs#L43)

</details>

### Naming a value with a type

**Level:** Newcomer

<!-- chapters: 07-values-and-functions -->

What does `let status: u16 = 200;` say, and why does `let status: u16 = "200";` fail to compile?

<details>
<summary>Answer</summary>

`let` introduces a variable named `status`. The part after the colon is its type: `u16`, an unsigned 16-bit whole number that holds 0 through 65,535. The value after `=` is 200. The second line puts text where the type requires a number, and Rust does not convert between them silently. Turning text into a number needs a parsing step that can fail, and the compiler rejects the line instead.

**Learn more:** [Give a value a name](../chapters/07-values-and-functions.md#give-a-value-a-name) · [A type can come from context](../chapters/07-values-and-functions.md#a-type-can-come-from-context)

</details>

### A function's last expression

**Level:** Newcomer

<!-- chapters: 07-values-and-functions -->

In `fn has_http_status(status: u16) -> bool { status != 0 }`, where does the returned value come from, and what changes if you add a semicolon after `status != 0`?

<details>
<summary>Answer</summary>

The arrow says the function returns a `bool`. The last expression in the body, `status != 0`, has no semicolon, so its value becomes the result. Adding a semicolon turns it into a statement whose value is discarded. The body then produces no `bool`, and the compiler reports that the function does not return the promised type.

**Learn more:** [Put one decision in a function](../chapters/07-values-and-functions.md#put-one-decision-in-a-function)

</details>

### Why `mut` is needed

**Level:** Newcomer

<!-- chapters: 07-values-and-functions, 11-drawing-and-navigation -->

Why does `let selected = 0_usize; selected += 1;` fail, and what does adding `mut` tell a reader?

<details>
<summary>Answer</summary>

Rust variables cannot be changed unless they are declared with `mut`. Writing `let mut selected` allows `+=` to store a new value. The keyword also tells the reader that this value is meant to change. In Netscope that separates values that describe the capture, which stay fixed, from view state such as the selected row, which changes as you press keys.

**Learn more:** [Make changes explicit](../chapters/07-values-and-functions.md#make-changes-explicit) · [Separate the capture from your place in it](../chapters/11-drawing-and-navigation.md#separate-the-capture-from-your-place-in-it)

</details>

### What a move does

**Level:** Newcomer

<!-- chapters: 03-storing-requests -->

After `let url = String::from("https://example.test/api/books"); let request_url = url;`, can you still print `url`? What happened to the text?

<details>
<summary>Answer</summary>

No. The assignment moves ownership of the `String` to `request_url`, and the compiler rejects any later use of `url`. The text was not copied. It stays in the same buffer, and `request_url` is now responsible for releasing it. The rule exists so two owners never both try to free the same buffer.

**Common wrong answer.** "Assignment copies the string, so both names work." That is true for small number types, but a `String` owns heap memory and is moved.

**Learn more:** [A string owns its bytes](../chapters/03-storing-requests.md#a-string-owns-its-bytes) · [Follow the move](../chapters/03-storing-requests.md#follow-the-move)

</details>

### Borrowing versus cloning

**Level:** Newcomer

<!-- chapters: 03-storing-requests, 16-preparing-a-request -->

A drawing function needs to read a request's URL. A replay draft needs a URL it can edit. Which one should borrow and which one should clone, and why?

<details>
<summary>Answer</summary>

The drawing function should borrow with a shared reference such as `&Request`. It reads the existing bytes while the session stays the owner, and nothing is copied. The draft should clone. It must change its text while the captured URL stays as recorded, so it needs its own buffer. Cloning costs an allocation and a copy of the bytes, which is why Netscope clones only what the draft needs, not the whole capture.

**Learn more:** [Borrow while drawing](../chapters/03-storing-requests.md#borrow-while-drawing) · [Give the draft its own text](../chapters/16-preparing-a-request.md#give-the-draft-its-own-text)

</details>

### Why the view stores request IDs

**Level:** Newcomer

<!-- chapters: 03-storing-requests, 08-structs-vectors-and-ids, 11-drawing-and-navigation -->

The screen must remember which request you selected across many redraws. Why does Netscope store a request ID for this instead of a reference to the request?

<details>
<summary>Answer</summary>

A reference is suited to short access, such as reading during one draw. Remembering a selection lasts across many key presses and redraws. An ID is a plain number the view state can own, and the program looks the request up in the session whenever it needs it. The lookup returns an `Option`, so an ID with no matching request becomes a visible absence instead of a crash.

**Learn more:** [Why store IDs?](../chapters/03-storing-requests.md#why-store-ids) · [`Session::request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L264)

</details>

### Structs and vectors

**Level:** Newcomer

<!-- chapters: 08-structs-vectors-and-ids -->

What is the difference between a `struct` and a `Vec`, and how does Netscope use each for requests?

<details>
<summary>Answer</summary>

A `struct` groups named fields that belong to one thing, such as one request's method and URL. A `Vec<T>` is an ordered, growable list of values of one type, such as all the requests in a capture. The vector keeps order and allows repeats, which is also why duplicate headers can stay as separate entries. When a request is pushed into the vector, the vector takes ownership of it.

**Learn more:** [Group one request's fields](../chapters/08-structs-vectors-and-ids.md#group-one-requests-fields) · [Keep several values in a vector](../chapters/08-structs-vectors-and-ids.md#keep-several-values-in-a-vector)

</details>

### Separate types for separate IDs

**Level:** Newcomer

<!-- chapters: 08-structs-vectors-and-ids, 04-quality-and-tests -->

A request ID and a host ID are both numbers. Why does Netscope wrap them in `RequestId` and `HostId` instead of using plain `usize` values?

<details>
<summary>Answer</summary>

`RequestId(usize)` and `HostId(usize)` are tuple structs, so the compiler treats them as different types. A function that expects a `RequestId` rejects a `HostId` at compile time, even when both hold the number 2. That catches a whole class of mix-ups before any test runs. The wrapper does not prove the request exists. A lookup must still check that.

**Learn more:** [Give different IDs different types](../chapters/08-structs-vectors-and-ids.md#give-different-ids-different-types) · [The compiler checks Rust's rules](../chapters/04-quality-and-tests.md#the-compiler-checks-rusts-rules) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6)

</details>

### Sorting without renumbering

**Level:** Newcomer

<!-- chapters: 08-structs-vectors-and-ids, 12-grouping-requests -->

The list shows requests in time order, but request 0 still means the first entry in the file. How can both be true?

<details>
<summary>Answer</summary>

The session keeps requests in a vector in file order, and a request's ID is its position there. A separate vector holds IDs sorted by start time, with ties kept in file order. The screen reads that second vector. Sorting it never moves a request, so an ID always names the same entry, and a host filter or search shows a subset of IDs without renumbering anything.

**Learn more:** [Sorting should preserve identity](../chapters/08-structs-vectors-and-ids.md#sorting-should-preserve-identity) · [Filter the view while keeping identity](../chapters/12-grouping-requests.md#filter-the-view-while-keeping-identity) · [`chronological`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L210-L211)

</details>

### Option and Result

**Level:** Newcomer

<!-- chapters: 09-options-results-and-matching -->

What is the difference between `Option` and `Result`? Give one place where each appears in Netscope.

<details>
<summary>Answer</summary>

`Option` says whether a value is present: `Some(value)` or `None`. A checked vector lookup returns one, and so does looking up a request by ID. `Result` says whether an operation succeeded: `Ok(value)` or `Err(error)`. Converting a HAR timing returns `Result<Timing, InvalidTiming>`, because `-2` is invalid input rather than an absent value. Both are ordinary enums the caller has to handle.

**Learn more:** [Represent the possibility of absence](../chapters/09-options-results-and-matching.md#represent-the-possibility-of-absence) · [Give failure a value too](../chapters/09-options-results-and-matching.md#give-failure-a-value-too) · [`Timing::from_optional_har`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L52)

</details>

### What `match` guarantees

**Level:** Newcomer

<!-- chapters: 09-options-results-and-matching, 13-following-redirects -->

Why does Netscope use `match` to choose a label for a timing, and what happens if someone adds a new `Timing` variant later?

<details>
<summary>Answer</summary>

`match` compares a value against patterns and runs the arm that fits. Rust requires the arms to cover every variant. If a new variant is added, every `match` that lists the existing variants stops compiling until someone decides how to show the new case. The display cannot quietly fall back to a misleading label. The same idea keeps "search unavailable" and "search found nothing" apart in the redirect analysis.

**Learn more:** [Handle the alternatives with match](../chapters/09-options-results-and-matching.md#handle-the-alternatives-with-match) · [Give different outcomes different Rust values](../chapters/13-following-redirects.md#give-different-outcomes-different-rust-values)

</details>

### The question-mark operator

**Level:** Newcomer

<!-- chapters: 09-options-results-and-matching, 10-importing-a-har, 04-quality-and-tests -->

What does `?` do after a call that returns a `Result`, and why does Netscope not call `unwrap` there instead?

<details>
<summary>Answer</summary>

If the result is `Ok`, `?` takes out the value and execution continues. If it is `Err`, `?` returns that error from the current function at once. The importer uses this to stop at the first invalid required field. `unwrap` would panic on `Err` and end the program, which is wrong for an expected problem such as a bad file. The workspace lints deny `unwrap` and `expect` for this reason.

**Learn more:** [Stop when the next step cannot proceed](../chapters/09-options-results-and-matching.md#stop-when-the-next-step-cannot-proceed) · [Use linting for recurring code concerns](../chapters/04-quality-and-tests.md#use-linting-for-recurring-code-concerns) · [`unsafe_code`](https://github.com/sidsharma96/netscope/blob/main/Cargo.toml#L16)

</details>

### When an import fails

**Level:** Newcomer

<!-- chapters: 10-importing-a-har -->

A HAR has three entries. The second has an invalid required wait time. In a second file, only the second entry's base64 body is malformed. What does Netscope show in each case?

<details>
<summary>Answer</summary>

In the first file the import fails. Netscope reports an error that names the field's location, and it shows no request list, not even the valid first entry. In the second file the import succeeds. The broken body is kept in its encoded form with a warning, and the request's method, URL, headers and timings stay inspectable. A required field decides whether the capture is accepted, while a body problem is recoverable.

**Learn more:** [Decide when a failure blocks the request](../chapters/10-importing-a-har.md#decide-when-a-failure-blocks-the-request) · [Build a complete session before displaying it](../chapters/10-importing-a-har.md#build-a-complete-session-before-displaying-it) · [`import_bytes`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L138)

</details>

### Selection, scroll and focus

**Level:** Newcomer

<!-- chapters: 11-drawing-and-navigation -->

In a wide window the request list and its details are both visible. Pressing Down could select the next request or scroll the body. What decides which one happens?

<details>
<summary>Answer</summary>

Focus decides. Focus is the pane that receives movement keys, and selection is the request whose details are shown. They are separate values in the view state. Tab moves focus between the list and the details, and Enter focuses the selected request's details. The bright border marks focus and the filled row marks selection. Scroll position is a third value that records your place within the text.

**Learn more:** [Which pane should a key change?](../chapters/11-drawing-and-navigation.md#which-pane-should-a-key-change) · [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66)

</details>

### Searching for a URL

**Level:** Newcomer

<!-- chapters: 11-drawing-and-navigation -->

You press `/`, type `API`, and press Enter. Will it find `https://example.com/api/books`?

<details>
<summary>Answer</summary>

No. The search is an exact, case-sensitive substring match on the captured URL text, so `API` does not match `api`. It looks only at URLs, not headers or bodies, and punctuation and percent-encoding are compared as plain text. The search applies when you press Enter, not as you type. It also combines with any host filter, and pressing `a` clears both.

**Common wrong answer.** "Search ignores case, like a browser's find." Netscope compares the stored bytes exactly.

**Learn more:** [Find a request without changing the capture](../chapters/11-drawing-and-navigation.md#find-a-request-without-changing-the-capture) · [`r.url.contains`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L176)

</details>

### Leaving the terminal usable

**Level:** Newcomer

<!-- chapters: 11-drawing-and-navigation, 04-quality-and-tests -->

Netscope switches the terminal into raw mode and an alternate screen. How does it restore the terminal if drawing fails partway through?

<details>
<summary>Answer</summary>

A guard value owns the cleanup. When setup changes the terminal, the guard records those changes. A normal exit restores them explicitly and reports any error. If the function returns early after an error, the guard's `Drop` implementation runs when it leaves scope and attempts the same restoration. A separate pseudo-terminal check runs the real program and inspects the terminal modes after it exits.

**Learn more:** [Leave the terminal usable](../chapters/11-drawing-and-navigation.md#leave-the-terminal-usable) · [Check the rule and the visible result](../chapters/04-quality-and-tests.md#check-the-rule-and-the-visible-result) · [`Restore`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L42)

</details>

### What a host summary counts

**Level:** Newcomer

<!-- chapters: 12-grouping-requests -->

Two requests to the same host start together and report 40 ms and 60 ms. The host view shows 100 ms. Does the host's traffic take 100 ms?

<details>
<summary>Answer</summary>

No. The host view adds each request's reported total once, and labels the value as a sum of reported durations. The two requests overlap, so the time from their common start to the later finish is 60 ms. The sum answers a different question. Requests are grouped by the URL's normalised host name, so the scheme and port do not split a group.

**Learn more:** [A sum answers a different question from elapsed time](../chapters/12-grouping-requests.md#a-sum-answers-a-different-question-from-elapsed-time) · [Choose what makes a group](../chapters/12-grouping-requests.md#choose-what-makes-a-group) · [`HostSummary`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/hosts.rs#L23)

</details>

### Redirect candidates

**Level:** Newcomer

<!-- chapters: 13-following-redirects -->

Request #0 got a 302 to `/books`, and requests #1 and #2 both ask for `/books` shortly afterwards. Does Netscope say which one the browser followed?

<details>
<summary>Answer</summary>

No. A HAR does not record which request caused another. Netscope lists both as possible follow-ups because both match the stated rules: the resolved Location target, a compatible method, and a start after #0's reported end and within 30 seconds of it. Even a single candidate is a suggestion, not proof. "No candidates" means only that nothing met the rules.

**Common wrong answer.** "The first matching request is the follow-up." Choosing it adds a claim the recording does not support.

**Learn more:** [Two plausible requests are still two requests](../chapters/13-following-redirects.md#two-plausible-requests-are-still-two-requests) · [A search needs a stated boundary](../chapters/13-following-redirects.md#a-search-needs-a-stated-boundary) · [`RedirectEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L7)

</details>

### Possible retries

**Level:** Newcomer

<!-- chapters: 14-recognizing-repeats -->

After a 503, the same GET appears again. When does Netscope list it as a possible retry, and what does it avoid claiming?

<details>
<summary>Answer</summary>

Netscope searches only after statuses 408, 429, 500, 502, 503 and 504, and only for a GET or HEAD reported with no body. A later request qualifies if it has the same method and URL, exactly the same recorded headers, query fields and cookies, and starts within 30 seconds after the first one ended. It is a candidate, not a confirmed retry. A refresh or another tab can produce the same entry.

**Learn more:** [An address is only part of a request](../chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request) · [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [`RetryEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L7)

</details>

### What an HTTP/2 label proves

**Level:** Newcomer

<!-- chapters: 15-reading-protocol-labels, 28-sharing-a-connection -->

Two overlapping requests are both labelled HTTP/2. Did they share one connection?

<details>
<summary>Answer</summary>

The label cannot tell you. HTTP/2 supports carrying several exchanges on one connection, but a version label describes a capability, not an event. A HAR does not contain the frames or stream identifiers that would show the sharing. Overlapping times are consistent with one shared connection and also with two separate ones. Proving it needs connection evidence, such as a server log.

**Common wrong answer.** "HTTP/2 always multiplexes, so they shared a connection."

**Learn more:** [A capability is not a measurement](../chapters/15-reading-protocol-labels.md#a-capability-is-not-a-measurement) · [What our recordings establish](../chapters/28-sharing-a-connection.md#what-our-recordings-establish)

</details>

### Two protocol labels that disagree

**Level:** Newcomer

<!-- chapters: 15-reading-protocol-labels -->

A request's protocol says HTTP/1.1 and its response says HTTP/3. What does Netscope show?

<details>
<summary>Answer</summary>

It shows both original strings, classifies each one on its own, and labels the disagreement. It does not pick the newer value or call it an upgrade, since the two strings do not establish that story. Recognition is exact: `HTTP/2`, `HTTP/2.0`, `http/2.0` and `h2` count as HTTP/2, while unrecognised text such as `HTTP/4` stays visible as captured data.

**Learn more:** [Keep both labels when they differ](../chapters/15-reading-protocol-labels.md#keep-both-labels-when-they-differ) · [`RecordedProtocol`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/protocol.rs#L3)

</details>

### Readable text with a gzip header

**Level:** Newcomer

<!-- chapters: 15-reading-protocol-labels, 18-comparing-responses -->

A captured response has `Content-Encoding: gzip`, but its body in the HAR is plain readable text. Is the file broken?

<details>
<summary>Answer</summary>

No. The browser received compressed bytes, decompressed them, and saved the readable result. The header still describes the response as it arrived. Netscope does not apply gzip again, because the browser already did that step. Base64 in a HAR is a separate storage encoding, and Netscope decodes it once. For the same reason, replay comparison refuses to judge body bytes when either side reports a non-identity content encoding.

**Learn more:** [Why the body can be larger than the download](../chapters/15-reading-protocol-labels.md#why-the-body-can-be-larger-than-the-download) · [Bytes need a common meaning](../chapters/18-comparing-responses.md#bytes-need-a-common-meaning)

</details>

### Streams and multiplexing

**Level:** Newcomer

<!-- chapters: 28-sharing-a-connection -->

What is an HTTP/2 stream, and how does it differ from an SSE event stream?

<details>
<summary>Answer</summary>

An HTTP/2 stream is one identified request and response exchange inside a connection. Its data travels in frames that carry the stream's identifier, so frames from several streams can be interleaved. That sharing is called multiplexing. An SSE event stream is a text format inside one response body. A single HTTP/2 stream can carry an SSE response with many events, so each event is not a new request.

**Learn more:** [How can two requests share one connection?](../chapters/28-sharing-a-connection.md#how-can-two-requests-share-one-connection) · [A label tells the receiver where a piece belongs](../chapters/28-sharing-a-connection.md#a-label-tells-the-receiver-where-a-piece-belongs)

</details>

### Flow control and header compression

**Level:** Newcomer

<!-- chapters: 28-sharing-a-connection -->

What do HTTP/2 flow control and HPACK each do, and can Netscope recover either from a HAR?

<details>
<summary>Answer</summary>

Flow control limits how much body data a sender may send, with separate budgets for each stream and for the whole connection. WINDOW_UPDATE frames raise those budgets. HPACK compresses headers by referring to entries in a static table and a dynamic table that changes as headers are sent. A HAR keeps only the final decoded header names and values and request timings. Netscope therefore cannot reconstruct flow-control waits, table contents, or compression savings.

**Learn more:** [Sharing also needs limits](../chapters/28-sharing-a-connection.md#sharing-also-needs-limits) · [Repeated headers can use remembered values](../chapters/28-sharing-a-connection.md#repeated-headers-can-use-remembered-values)

</details>

### Draft, review and send

**Level:** Newcomer

<!-- chapters: 16-preparing-a-request -->

You press `r` on a captured request. What do you get, and what has to happen before anything goes over the network?

<details>
<summary>Answer</summary>

You get a draft: a separate, editable copy of the URL with no captured headers selected. Editing it never changes the recording. Opening a review validates the current draft and shows the effective destination, headers, timeout and size limit. Review does no network work. Only Enter or `s` from a valid review sends the request. Changing the URL clears selected headers and the body, so a choice made for one destination does not travel to another.

**Learn more:** [A destination change also changes a decision](../chapters/16-preparing-a-request.md#a-destination-change-also-changes-a-decision) · [Review the current draft, not an earlier one](../chapters/16-preparing-a-request.md#review-the-current-draft-not-an-earlier-one) · [`ReplayDraft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L65)

</details>

### Waiting without freezing the screen

**Level:** Newcomer

<!-- chapters: 17-waiting-for-a-response, 24-sharing-work -->

A server accepts your request and then sends nothing. How can Netscope still react when you press Esc?

<details>
<summary>Answer</summary>

Each Send runs on its own worker thread, so the terminal thread keeps reading keys. Inside the worker, a Tokio runtime drives the exchange as a future, a value representing unfinished work. A pending socket read lets the runtime also watch a cancellation signal and a deadline. Esc sends that signal. The worker stops waiting, drops the connection, and returns whatever bytes it already received.

**Learn more:** [Waiting does not need to occupy the terminal](../chapters/17-waiting-for-a-response.md#waiting-does-not-need-to-occupy-the-terminal) · [A future holds unfinished work](../chapters/17-waiting-for-a-response.md#a-future-holds-unfinished-work) · [`Worker`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L312)

</details>

### Handing over a result

**Level:** Newcomer

<!-- chapters: 24-sharing-work, 17-waiting-for-a-response -->

Why does the worker send its finished result through a channel instead of sharing a growing response body with the terminal under a lock?

<details>
<summary>Answer</summary>

Only one part of the program needs to change the body at a time. The worker owns it while receiving, then moves it into the result and sends that through a channel with room for one value. The terminal owns what it receives. Nothing is edited by two threads at once, so there is no lock to hold and no consistency rule to protect. A mutex would only be worth it if two parts really had to change related data together.

**Learn more:** [Hand the value over when responsibility changes](../chapters/24-sharing-work.md#hand-the-value-over-when-responsibility-changes) · [Protect several related changes together](../chapters/24-sharing-work.md#protect-several-related-changes-together) · [`sync_channel`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L353)

</details>

### Operation numbers

**Level:** Newcomer

<!-- chapters: 24-sharing-work -->

Each Send gets an operation number. Why, and why is it an ordinary counter rather than an atomic one?

<details>
<summary>Answer</summary>

The number lets the terminal accept a result only for the operation it is currently waiting on, so a late result from an older send cannot replace a newer one. Only the terminal thread hands out these numbers, so no other thread competes for them and an atomic counter is unnecessary. The application owns the counter, so closing an editor does not restart the sequence. When the numbers run out, sending is refused instead of wrapping back to an old number.

**Learn more:** [Update one counter without losing an increment](../chapters/24-sharing-work.md#update-one-counter-without-losing-an-increment) · [A late result must not change a newer decision](../chapters/17-waiting-for-a-response.md#a-late-result-must-not-change-a-newer-decision) · [`next_send_id`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L208)

</details>

### What "stored bytes match" means

**Level:** Newcomer

<!-- chapters: 18-comparing-responses -->

After a complete replay, the comparison says "stored bytes match". Does that mean the server sent the same response as before?

<details>
<summary>Answer</summary>

Not necessarily. It means every byte the capture retained equals the replay's bytes. The capture may have kept a shortened or converted body, since browsers can transcode text when exporting. The server's data and conditions may also have changed in ways the body does not show. If the replay stopped early, Netscope compares only the shared prefix and keeps the stopping reason beside that narrower answer.

**Common wrong answer.** "The responses are identical." The comparison speaks only about the bytes both sides have.

**Learn more:** [Absence is different from emptiness](../chapters/18-comparing-responses.md#absence-is-different-from-emptiness) · [Ask a smaller question about a partial body](../chapters/18-comparing-responses.md#ask-a-smaller-question-about-a-partial-body) · [`compare_body`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/comparison.rs#L60)

</details>

### Counting the bytes in café

**Level:** Newcomer

<!-- chapters: 19-submitting-text, 21-moving-through-text -->

You POST the plain-text body `café`. What Content-Length does Netscope send, and why?

<details>
<summary>Answer</summary>

Five. Content-Length counts bytes, not letters. Rust stores text as UTF-8, where `c`, `a` and `f` each take one byte and the precomposed `é` takes two. Netscope computes the length from the reviewed body bytes and never reuses a captured Content-Length, since an edited body would make it wrong. A JSON body such as `{"title":"café"}` also counts its braces, quotes and colon.

**Common wrong answer.** "Four, one per character."

**Learn more:** [Text already has a byte representation](../chapters/19-submitting-text.md#text-already-has-a-byte-representation) · [A length describes the bytes we actually send](../chapters/19-submitting-text.md#a-length-describes-the-bytes-we-actually-send)

</details>

### Moving the cursor by one character

**Level:** Newcomer

<!-- chapters: 21-moving-through-text -->

The text `é` can be stored as `e` followed by a combining accent. If you press Left once after it, where should the cursor go?

<details>
<summary>Answer</summary>

Before the `e`, past the whole visible letter. The two Unicode values form one grapheme cluster, a group treated as one unit for movement and deletion. A one-byte step could land inside an encoded character, and a one-`char` step would separate the letter from its accent. Netscope stores the cursor as a byte position but moves it only between grapheme boundaries, and checks the boundaries again after each edit.

**Learn more:** [One visible letter can have two parts](../chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts) · [A byte position needs a rule](../chapters/21-moving-through-text.md#a-byte-position-needs-a-rule) · [`Cursor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L5)

</details>

### Bytes, characters and terminal columns

**Level:** Newcomer

<!-- chapters: 21-moving-through-text, 11-drawing-and-navigation -->

Why can't Netscope use a string's byte length to decide where to wrap a header on screen?

<details>
<summary>Answer</summary>

Bytes, visible characters and terminal columns are different lengths. `A` takes one byte and one column. `界` takes three bytes and usually two columns. A combining mark takes bytes but no column. Netscope groups text into graphemes and measures their width with a Unicode width library before wrapping. Control characters are first shown as visible escapes, which take several columns.

**Learn more:** [A line of text has more than one length](../chapters/11-drawing-and-navigation.md#a-line-of-text-has-more-than-one-length) · [Storage positions are not screen columns](../chapters/21-moving-through-text.md#storage-positions-are-not-screen-columns)

</details>

### Saving a draft

**Level:** Newcomer

<!-- chapters: 20-remembering-a-request -->

You save a draft and reopen it tomorrow. What comes back, what is left out, and does opening it send anything?

<details>
<summary>Answer</summary>

The file is versioned JSON holding the name, URL, method, body format, body and headers. Known credential headers such as `Authorization` and `Cookie` are left out. Secrets elsewhere, for example in the URL's query string or the body, are saved as written. Opening the file validates it and starts an editor with no captured response. It never sends anything. Netscope writes a temporary file, syncs it, then renames it over the old one, so a failed write keeps yesterday's file.

**Learn more:** [Choose what should survive](../chapters/20-remembering-a-request.md#choose-what-should-survive) · [Replacing a file has a vulnerable middle](../chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle) · [`sensitive_saved_header`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L27)

</details>

### Events inside one SSE response

**Level:** Newcomer

<!-- chapters: 25-reading-event-streams -->

A captured `text/event-stream` body contains several `data:` lines. What marks the end of one event, and can Netscope tell you when each event arrived?

<details>
<summary>Answer</summary>

A blank line ends an event. Consecutive `data` lines before it are joined with newlines into one event's data. Text after the last blank line is shown as an unfinished block, not as an event, because the end of the file is not a boundary. A HAR keeps only the body text, so Netscope can show event order but not arrival times.

**Learn more:** [A blank line gives a boundary meaning](../chapters/25-reading-event-streams.md#a-blank-line-gives-a-boundary-meaning) · [A file can stop before an event does](../chapters/25-reading-event-streams.md#a-file-can-stop-before-an-event-does) · [`parse_sse`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/sse.rs#L76)

</details>

### A WebSocket upgrade

**Level:** Newcomer

<!-- chapters: 26-switching-protocols -->

A HAR contains a request answered with status 101. What happened on that connection, and what can Netscope show about it?

<details>
<summary>Answer</summary>

101 Switching Protocols means the server agreed to stop speaking HTTP/1.1 on that connection and start WebSocket. Later traffic is WebSocket frames, not an HTTP body. Netscope checks the recorded handshake headers, such as `Upgrade`, `Connection`, the key, version 13 and any chosen subprotocol. It does not compute the expected `Sec-WebSocket-Accept` value, and it shows no messages, because the exports do not contain them.

**Common wrong answer.** "Netscope verifies the accept hash." It checks only that one non-empty accept field is present.

**Learn more:** [An agreement has two sides](../chapters/26-switching-protocols.md#an-agreement-has-two-sides) · [The exporter can stop at the agreement](../chapters/26-switching-protocols.md#the-exporter-can-stop-at-the-agreement) · [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L28)

</details>

### Times in a live SSE observation

**Level:** Newcomer

<!-- chapters: 27-observing-live-events, 17-waiting-for-a-response -->

During live SSE, Netscope shows a time beside each event. Where does that clock start, and what does the time measure?

<details>
<summary>Answer</summary>

The clock starts when you accept Send, so connection setup, TLS and waiting for headers all count. An event's time is when the worker first saw its complete boundary in received bytes. Events that arrive in the same read share one time. It is a local observation, not when the server created the event or when a browser delivered it. The observation stops after 30 seconds, 64 KiB, or 100 events.

**Learn more:** [A clock needs a named starting point](../chapters/27-observing-live-events.md#a-clock-needs-a-named-starting-point) · [Stopping is part of the observation](../chapters/27-observing-live-events.md#stopping-is-part-of-the-observation) · [`STREAM_TIME_LIMIT`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/streaming.rs#L10)

</details>

### Captured length and original length

**Level:** Newcomer

<!-- chapters: 29-reading-packet-bytes -->

A PCAP record says the original packet was 120 bytes but only 80 were captured. What does Netscope do with the missing 40?

<details>
<summary>Answer</summary>

It treats them as unavailable, never as zeros. The capture tool stored only the first 80 bytes, often because of a snap length setting. Netscope keeps the headers and payload prefix that fit, and adds the note "Capture cut: fewer bytes retained than original packet". Later, TCP reassembly marks the uncaptured positions as missing, so no analysis reads past them.

**Learn more:** [Which bytes were actually recorded?](../chapters/29-reading-packet-bytes.md#which-bytes-were-actually-recorded) · [Check availability before indexing](../chapters/29-reading-packet-bytes.md#check-availability-before-indexing) · [`import_pcap`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/pcap.rs#L69)

</details>

### Reading a two-byte number

**Level:** Newcomer

<!-- chapters: 29-reading-packet-bytes -->

A TCP header holds the port as bytes `01 bb`. What number is that, and why can't Netscope use the computer's own byte order?

<details>
<summary>Answer</summary>

It is 443: the first byte is the high part, so 1 × 256 + 187. Network headers use this big-endian order whatever machine reads them, and Rust's `u16::from_be_bytes` applies it explicitly. A classic PCAP file's own header fields can use either byte order, marked by its magic bytes. The reader therefore needs one rule for the file container and another for the packets inside it.

**Learn more:** [Two bytes need an agreed order](../chapters/29-reading-packet-bytes.md#two-bytes-need-an-agreed-order) · [A boundary belongs to a particular layer](../chapters/29-reading-packet-bytes.md#a-boundary-belongs-to-a-particular-layer)

</details>

### Sequence numbers and gaps

**Level:** Newcomer

<!-- chapters: 30-putting-bytes-in-order -->

TCP segments arrive with `abc` at positions 0 to 2 and `ghi` at positions 6 to 8. What does Netscope build, and what does the HTTP parser see?

<details>
<summary>Answer</summary>

TCP sequence numbers give each byte a position in one direction's stream. Netscope places bytes at their positions and keeps a separate state for each one: Missing, Observed or Conflict. Here positions 3 to 5 are Missing, and they are not filled with zeros or squeezed out. HTTP analysis reads only the contiguous observed prefix, `abc`, and stops at the gap. It never joins the pieces into `abcghi`.

**Learn more:** [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning) · [Remember absence separately from the value](../chapters/30-putting-bytes-in-order.md#remember-absence-separately-from-the-value) · [`ByteState`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/reassembly.rs#L13)

</details>

### Chunked bodies

**Level:** Newcomer

<!-- chapters: 31-finding-http-boundaries -->

How does a chunked HTTP/1.1 response mark where its body ends, and which bytes become the body?

<details>
<summary>Answer</summary>

Each chunk starts with its size in hexadecimal and a CRLF, then that many data bytes, then another CRLF. A chunk of size zero starts the ending section, which may hold trailer fields and ends with an empty line. Only the data bytes form the body. The size lines, CRLFs and trailers are framing. Packet boundaries do not matter, because Netscope reads chunks from the reassembled byte stream, not from individual packets.

**Learn more:** [A length and a delimiter answer different questions](../chapters/31-finding-http-boundaries.md#a-length-and-a-delimiter-answer-different-questions) · [Packet boundaries and message boundaries have different jobs](../chapters/31-finding-http-boundaries.md#packet-boundaries-and-message-boundaries-have-different-jobs) · [`inspect_http`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/wire_http.rs#L140)

</details>

### SDP and STUN

**Level:** Newcomer

<!-- chapters: 32-reading-signaling -->

Netscope reports SDP-like text with an ICE username fragment and a STUN candidate in the same capture. Did the call's audio flow?

<details>
<summary>Answer</summary>

The capture does not show that. SDP is a text description that proposes a session. STUN messages help check possible connection paths. Netscope recognises an SDP body by a narrow rule (`application/sdp`, a first line of `v=0`, and the expected fields) and a STUN datagram by its header signature and framing. Neither proves that a connectivity check succeeded or that media flowed.

**Learn more:** [Recognize a description without inventing its outcome](../chapters/32-reading-signaling.md#recognize-a-description-without-inventing-its-outcome) · [A datagram has its own boundary](../chapters/32-reading-signaling.md#a-datagram-has-its-own-boundary) · [`inspect_datagram`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/datagrams.rs#L113)

</details>

### A DNS answer with several addresses

**Level:** Newcomer

<!-- chapters: 33-interpreting-a-trace, 32-reading-signaling -->

A DNS response in a PCAP lists three addresses for one name. Does that show a load balancer, and which address did the client use?

<details>
<summary>Answer</summary>

Neither. The answer records the addresses the resolver returned. It does not say how they were chosen, which one the client tried, or whether it retried another. Netscope shows the supported UDP questions and A and AAAA answer fields only. It labels them as answer fields, not proof of resolver choice or load balancing.

**Common wrong answer.** "Several addresses mean load balancing." The answer alone does not establish a balancing algorithm.

**Learn more:** [Several addresses do not explain how one was selected](../chapters/33-interpreting-a-trace.md#several-addresses-do-not-explain-how-one-was-selected) · [Keep the observation's owner clear](../chapters/32-reading-signaling.md#keep-the-observations-owner-clear)

</details>

### Repeated bytes in a trace

**Level:** Newcomer

<!-- chapters: 33-interpreting-a-trace, 30-putting-bytes-in-order -->

Two TCP segments carry the same sequence range with the same bytes. Is that a retransmission after packet loss?

<details>
<summary>Answer</summary>

It might be, but the trace does not say so. The sender may have sent again, or the capture setup may have recorded the same packet twice. Netscope counts agreeing overlaps as repeated bytes and marks positions where copies disagree as conflicts. It does not label a repeat as loss recovery, because the cause is not in the recorded fields.

**Common wrong answer.** "Repeated bytes always mean loss and retransmission."

**Learn more:** [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [Put bytes at positions before reading their meaning](../chapters/30-putting-bytes-in-order.md#put-bytes-at-positions-before-reading-their-meaning)

</details>

### The TCP window field

**Level:** Newcomer

<!-- chapters: 33-interpreting-a-trace -->

A TCP header shows a window of 512. Can you read that as "the receiver will accept 512 more bytes"?

<details>
<summary>Answer</summary>

Not in general. With window scaling, the receiver of the window multiplies the 16-bit field by 2 to the power of a shift count from 0 to 14. Each side announces its shift only in its SYN segment. A capture that starts after the handshake cannot know the shift, and Netscope does not parse TCP options at all. It shows the raw field and claims no byte capacity. The window is also not the sender's congestion window.

**Learn more:** [Repeated bytes are visible; their cause may not be](../chapters/33-interpreting-a-trace.md#repeated-bytes-are-visible-their-cause-may-not-be) · [Sender congestion state is not a header field](../chapters/33-interpreting-a-trace.md#sender-congestion-state-is-not-a-header-field)

</details>

### The four crates

**Level:** Newcomer

<!-- chapters: 23-shaping-the-program -->

Netscope's workspace has four crates. What does each one do, and which ones can send network traffic?

<details>
<summary>Answer</summary>

`netscope-core` holds the data model and the pure analysis rules, such as sessions, timings, drafts and reassembly. `netscope-import` reads HAR and PCAP files into core types. `netscope-net` performs the one reviewed HTTP exchange. `netscope-tui` draws the terminal, handles keys and builds the `netscope` command. Only `netscope-net` sends traffic. Core depends on no terminal or network crate, so its rules can be tested without either.

**Learn more:** [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules) · [Keep a file format away from a screen's decisions](../chapters/23-shaping-the-program.md#keep-a-file-format-away-from-a-screens-decisions)

</details>

### Computing summaries once

**Level:** Newcomer

<!-- chapters: 23-shaping-the-program, 12-grouping-requests -->

Netscope builds host summaries and redirect and retry candidates when it loads a capture. Why is that a sound choice here, and when would it stop being one?

<details>
<summary>Answer</summary>

A loaded capture never changes while you browse it, so answers computed once stay correct. Storing them costs memory for counts and lists of request IDs, not copies of request bodies, and it avoids recounting on every key press. If requests could arrive while you watched, as in a live capture, the stored summaries would go stale. They would need updating, rebuilding, or computing on demand.

**Learn more:** [Spend memory to avoid repeating a question](../chapters/23-shaping-the-program.md#spend-memory-to-avoid-repeating-a-question) · [Why calculate a group before drawing it?](../chapters/12-grouping-requests.md#why-calculate-a-group-before-drawing-it)

</details>

### What a test proves

**Level:** Newcomer

<!-- chapters: 04-quality-and-tests -->

The program compiles and draws every row. Why does Netscope still need a test that feeds in a `-1` timing, and why are most tests built on small invented captures?

<details>
<summary>Answer</summary>

The compiler checks Rust's rules, not whether the program means the right thing. Turning `-1` into zero is valid Rust and still wrong, and only a test that states the expected `NotApplicable` result catches it. Small invented fixtures make the expected result easy to explain. Local data also keeps tests deterministic: the same inputs give the same result, with no public server whose answer or speed can change.

**Learn more:** [A test makes an expectation precise](../chapters/04-quality-and-tests.md#a-test-makes-an-expectation-precise) · [Control the inputs that affect the answer](../chapters/04-quality-and-tests.md#control-the-inputs-that-affect-the-answer)

</details>
