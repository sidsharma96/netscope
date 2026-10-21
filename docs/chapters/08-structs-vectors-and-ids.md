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

Try these questions using the explanation above, then open the answers.

### Structs and vectors

What is the difference between a `struct` and a `Vec`, and how does Netscope use each for requests?

<details>
<summary>Answer</summary>

A `struct` groups named fields that belong to one thing, such as one request's method and URL. A `Vec<T>` is an ordered, growable list of values of one type, such as all the requests in a capture. The vector keeps order and allows repeats, which is also why duplicate headers can stay as separate entries. When a request is pushed into the vector, the vector takes ownership of it.

**Learn more:** [Group one request's fields](../chapters/08-structs-vectors-and-ids.md#group-one-requests-fields) · [Keep several values in a vector](../chapters/08-structs-vectors-and-ids.md#keep-several-values-in-a-vector)

</details>

### Separate types for separate IDs

A request ID and a host ID are both numbers. Why does Netscope wrap them in `RequestId` and `HostId` instead of using plain `usize` values?

<details>
<summary>Answer</summary>

`RequestId(usize)` and `HostId(usize)` are tuple structs, so the compiler treats them as different types. A function that expects a `RequestId` rejects a `HostId` at compile time, even when both hold the number 2. That catches a whole class of mix-ups before any test runs. The wrapper does not prove the request exists. A lookup must still check that.

**Learn more:** [Give different IDs different types](../chapters/08-structs-vectors-and-ids.md#give-different-ids-different-types) · [The compiler checks Rust's rules](../chapters/04-quality-and-tests.md#the-compiler-checks-rusts-rules) · [`RequestId`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L6)

</details>

### Sorting without renumbering

The list shows requests in time order, but request 0 still means the first entry in the file. How can both be true?

<details>
<summary>Answer</summary>

The session keeps requests in a vector in file order, and a request's ID is its position there. A separate vector holds IDs sorted by start time, with ties kept in file order. The screen reads that second vector. Sorting it never moves a request, so an ID always names the same entry, and a host filter or search shows a subset of IDs without renumbering anything.

**Learn more:** [Sorting should preserve identity](../chapters/08-structs-vectors-and-ids.md#sorting-should-preserve-identity) · [Filter the view while keeping identity](../chapters/12-grouping-requests.md#filter-the-view-while-keeping-identity) · [`chronological`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L210-L211)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="why-the-view-stores-request-ids"></a>

[Why the view stores request IDs](../questions/newcomer.md#why-the-view-stores-request-ids)

<a id="how-does-a-session-keep-several-orders"></a>

[How does a session keep several orders?](../questions/practitioner.md#how-does-a-session-keep-several-orders)

<a id="how-are-requests-sorted-and-hosts-grouped"></a>

[How are requests sorted and hosts grouped?](../questions/practitioner.md#how-are-requests-sorted-and-hosts-grouped)

</details>
