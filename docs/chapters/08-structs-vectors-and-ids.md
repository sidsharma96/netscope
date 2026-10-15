# Keeping requests together

A method and a URL describe the same request. If we store them as unrelated variables, every part of the program has to remember which values belong together. A capture with hundreds of requests makes that arrangement difficult to manage.

Rust gives us a way to group the fields of one request, and another way to keep many requests in order. We can then ask a separate question: how do we remember a request when its position on screen changes?

## Group one request's fields

A `struct` defines a type with named fields. Here is a small request type with only the two fields we need for this example:

```rust
struct Request {
    method: String,
    url: String,
}
let request = Request {
    method: String::from("GET"),
    url: String::from("https://example.test/api/books"),
};
assert_eq!(request.method, "GET");
```

The first pair of braces defines the fields and their types. The second constructs a particular request by giving each field a value. Commas separate the fields. The dot in `request.method` selects the field named `method`.

Both fields own their strings. When the request is cleaned up, Rust also cleans up those fields. The grouping therefore records both the relationship between the values and who owns them.

Our full request has more fields, but the idea is the same. You can understand this small example before reading every piece of a HAR record.

## Keep several values in a vector

A [vector](../GLOSSARY.md#vec), written `Vec<T>`, is an ordered collection that can grow. The `T` stands for its element type: `Vec<String>` contains strings, and `Vec<Request>` contains requests. All elements in one vector have the same type.

To see how a vector grows, start with method names:

```rust
let mut methods = vec![String::from("GET")];
methods.push(String::from("POST"));
assert_eq!(methods.len(), 2);
```

The `vec!` [macro](../GLOSSARY.md#macro) creates a vector containing one string. Calling `push` adds another element at the end and transfers ownership of that string to the vector. Calling `len` returns the number of elements. We need `mut` because adding an element changes the vector.

A vector preserves order and permits repeated values. That makes it useful for captured headers too. Two headers with the same name can remain two separate entries in their original order.

## A position may have no element

Positions begin at zero. In our two-element vector, GET is at position zero and POST is at position one. Position two does not exist.

The `get` method checks a position and returns an `Option`. A successful lookup contains a reference to the element; a failed lookup contains no element. The [next chapter](09-options-results-and-matching.md) explains how to handle those alternatives.

This check matters when the user presses Down in an empty capture. There is no request to select, even though zero is a perfectly valid integer. A number's type alone cannot tell us whether it names an existing element.

Rust also permits direct indexing, such as `methods[0]`. An out-of-range direct index causes a [panic](../GLOSSARY.md#panic), which stops ordinary execution. For a user-controlled selection, we use checked lookup and a defined empty state.

## Sorting should preserve identity

Imagine three requests recorded in this order: request 0 starts at time 30, request 1 at time 10, and request 2 at time 10. These are invented times for the explanation.

We want the screen to show the earlier requests first, but we still want request 0 to mean the first entry in the capture. A second vector can store the display order:

```text
Request data stays here       The screen reads these IDs
0: starts at 30                1 → first row
1: starts at 10                2 → second row
2: starts at 10                0 → third row
```

The two requests with equal times stay in source order. Sorting the IDs changes the view without moving or copying the stored requests. An error referring to request 0 still points to the same source entry.

This costs some extra memory for the ID vector. It is useful here because we load a session once and read it many times. Editing or deleting requests would require another decision about how IDs survive those changes.

## Give different IDs different types

A request ID and a host ID can both contain the number 2 while referring to different things. Wrapping the numbers in separate types lets the compiler catch accidental mixing:

```rust
struct RequestId(usize);
struct HostId(usize);
fn request_position(id: RequestId) -> usize {
    id.0
}
assert_eq!(request_position(RequestId(2)), 2);
```

These are tuple structs. Their fields use positions instead of names, so `id.0` reads the first field. `RequestId(2)` constructs a request ID containing the number 2.

A function expecting `RequestId` does not accept `HostId`, even when the contained numbers match:

```compile_fail
struct RequestId(usize);
struct HostId(usize);
fn select_request(id: RequestId) {}
select_request(HostId(2));
```

This example is supposed to fail at compilation. Separate types protect the meaning of an ID. A lookup must still check that the request exists, and the application must use the ID with the session it came from.

## Try another ordering

Write down four requests and their start times. Give two of them the same time. Keep the requests in source order and write a second list containing the IDs in chronological order.

Which request does the second displayed row refer to? Explain why using that row number as a source ID can select the wrong request.

## Where this appears in Netscope

The session model [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197) keeps request data and display order separately. The ordering tests [`chronological_order_retains_offset_ties`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/har.rs#L45) check that equal instants retain source order, including timestamps written with different time-zone offsets.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Why the view stores request IDs

**Level:** Newcomer


The screen must remember which request you selected across many redraws. Why does Netscope store a request ID for this instead of a reference to the request?

<details>
<summary>Answer</summary>

A reference is suited to short access, such as reading during one draw. Remembering a selection lasts across many key presses and redraws. An ID is a plain number the view state can own, and the program looks the request up in the session whenever it needs it. The lookup returns an `Option`, so an ID with no matching request becomes a visible absence instead of a crash.

**Learn more:** [Why store IDs?](../chapters/03-storing-requests.md#why-store-ids) · [`Session::request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L264)

</details>

### How does a session keep several orders?

**Level:** Practitioner


How does a `Session` identify a request, and how does it offer chronological order and per-host order without moving or copying any request?

<details>
<summary>Answer</summary>

A request's identity is `RequestId`, a wrapper around the entry's position in the HAR `entries` array. That number never changes, whatever order the screen shows. The `Session` owns one `Vec<Request>` in source order and keeps separate lists of IDs for each view: `chronological` is a `Vec<RequestId>`, `host_order` is a `Vec<HostId>` sorted by host name, and each host summary keeps its own `request_ids` in chronological order. Redirect and retry evidence are vectors indexed by the same `RequestId`.

Views borrow a request by looking up its ID, so sorting a view sorts small numbers and never moves the requests themselves. A lookup with an ID that is out of range returns `None` instead of panicking. A saved draft that has no captured request uses the sentinel `RequestId(usize::MAX)`, so it can never be mistaken for a real entry.

**Learn more:** [Sorting should preserve identity](../chapters/08-structs-vectors-and-ids.md#sorting-should-preserve-identity) · [Why store requests in a vector addressed by RequestId?](../chapters/34-defending-the-design.md#why-store-requests-in-a-vector-addressed-by-requestid) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6) · [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197-L205) · [`SAVED_SOURCE`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L151)

</details>

### Structs and vectors

**Level:** Newcomer


What is the difference between a `struct` and a `Vec`, and how does Netscope use each for requests?

<details>
<summary>Answer</summary>

A `struct` groups named fields that belong to one thing, such as one request's method and URL. A `Vec<T>` is an ordered, growable list of values of one type, such as all the requests in a capture. The vector keeps order and allows repeats, which is also why duplicate headers can stay as separate entries. When a request is pushed into the vector, the vector takes ownership of it.

**Learn more:** [Group one request's fields](../chapters/08-structs-vectors-and-ids.md#group-one-requests-fields) · [Keep several values in a vector](../chapters/08-structs-vectors-and-ids.md#keep-several-values-in-a-vector)

</details>

### How are requests sorted and hosts grouped?

**Level:** Practitioner


How exactly is chronological order computed, including the tie-break and the cost? How are timestamps with different UTC offsets compared? What is the host grouping key, does it include the port, and in what order are hosts listed?

<details>
<summary>Answer</summary>

`startedDateTime` must be an RFC 3339 timestamp with an explicit offset, or the import fails. The importer converts it to UTC seconds and nanoseconds, so timestamps written with different offsets compare as real instants. The chronological list is the request IDs sorted by `(instant, source index)`. Equal instants keep source order, and the sort costs O(n log n).

The host key is the URL host as normalised by the `url` crate: lowercased, with international names converted to their ASCII form and IPv6 addresses in brackets. The port and scheme are not part of the key, so `example.com:8443` and `https://example.com` share one host. Hosts receive IDs in the order they first appear, using a `BTreeMap` lookup, and each host's request IDs are filled in chronological order. The host view lists hosts sorted by name, byte by byte.

**Common wrong answer.** "Hosts are grouped by host and port." Netscope groups only by the normalised host name.

**Learn more:** [Choose what makes a group](../chapters/12-grouping-requests.md#choose-what-makes-a-group) · [Deterministic identity and display](../contracts/m1-har-inspection.md#deterministic-identity-and-display) · [`timestamp`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L212-L216) · [`Session::new`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L209-L234)

</details>

### Separate types for separate IDs

**Level:** Newcomer


A request ID and a host ID are both numbers. Why does Netscope wrap them in `RequestId` and `HostId` instead of using plain `usize` values?

<details>
<summary>Answer</summary>

`RequestId(usize)` and `HostId(usize)` are tuple structs, so the compiler treats them as different types. A function that expects a `RequestId` rejects a `HostId` at compile time, even when both hold the number 2. That catches a whole class of mix-ups before any test runs. The wrapper does not prove the request exists. A lookup must still check that.

**Learn more:** [Give different IDs different types](../chapters/08-structs-vectors-and-ids.md#give-different-ids-different-types) · [The compiler checks Rust's rules](../chapters/04-quality-and-tests.md#the-compiler-checks-rusts-rules) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6)

</details>
