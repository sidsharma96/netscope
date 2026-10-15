# Who keeps a request alive?

You select a request in the list and open its details. Both screens need the same URL, headers, and response body. Copying all of that data for each screen would use more memory and create more values to keep consistent.

We can store the request once and let each screen read it. Rust asks us to make one part of that arrangement explicit: who is responsible for keeping the data alive, and who is only using it for a while?

## A string owns its bytes

Text occupies memory. Rust's `String` type manages a region of memory containing UTF-8 text. UTF-8 represents characters as bytes, and a character can use more than one byte. A region reserved for this text is often called a buffer.

A `String` keeps track of the buffer's location, the number of bytes currently used, and the available capacity. It also takes responsibility for releasing the buffer when it is no longer needed. That responsibility is what [ownership](../GLOSSARY.md#ownership) means in this example.

Consider giving a URL another name:

```rust
let url = String::from("https://example.test/api/books");
let request_url = url;
```

`String::from` creates an owned string from the quoted text. The second line transfers that string into `request_url`. Rust calls this a [move](../GLOSSARY.md#move). The text stays in the same allocated buffer, and `request_url` becomes responsible for it. The old name, `url`, can no longer be used to access the moved value.

This rule prevents two independent owners from both trying to release the same buffer. If we want a second owned copy of the text, we can request one with `clone`. That copies the string's contents and gives the copy its own storage.

## Follow the move

Follow the diagram and watch the owner change while the text buffer remains in place. The drawing explains ownership; it does not promise a particular physical memory address or compiler layout.

The figure below shows the same allocation at four moments. Read it from top to bottom.

*Figure 4.1. One allocation, changing ownership (illustrative).*

```text
Step 1   url ─────────────▶ [ https://example.test/api/books ]
         owner              one text buffer

Step 2   request_url ─────▶ [ https://example.test/api/books ]
         owner              same buffer, not cloned
         url                no longer usable

Step 3   request_url ─────▶ [ https://example.test/api/books ]
         owner                ▲
         &request_url ────────┘  draw_request reads the same bytes

Step 4   request_url ─────▶ [ https://example.test/api/books ]
         owner              the borrow has ended
```

1. The variable `url` owns the `String`. It tracks the location of one allocated text buffer.
2. Assignment moves ownership to `request_url`. The old name `url` can no longer be used. The text is not cloned.
3. A drawing function borrows a shared reference. It reads the existing bytes while `request_url` remains their owner.
4. The drawing function returns. Its borrow is no longer needed, and the owner can still use the string.

A local variable belongs to a [scope](../GLOSSARY.md#scope), the region of code in which its name is available. A function body is one example. When the owning value reaches the end of its lifetime, Rust normally runs its cleanup. For `String`, that releases the buffer.

Moving a value out of a function can give ownership to the caller. Leaving a function therefore does not always mean destroying everything it used. The useful question is where ownership went.

## Borrow while drawing

A screen needs permission to read the URL, but it does not need to take over its storage. A [shared reference](../GLOSSARY.md#borrow) gives it that access. Rust writes a reference with `&` in front of the type.

Here is a small teaching example. A `struct` groups named fields into one value; this one has only a URL field.

```rust
struct Request {
    url: String,
}

fn draw_request(request: &Request) {
    println!("{}", request.url);
}
```

The function receives `&Request`, a shared reference to a request. Calling this function lends it access to an existing value. The function can read the URL while the request's owner remains responsible for the data.

The dot in `request.url` selects the URL field. `println!` prints formatted text, and `{}` marks where that value appears. This small example writes one line; the application uses the same borrowing idea to draw a terminal screen.

A shared reference does not allow this function to replace the URL. Rust also checks that the reference cannot outlive the request it refers to. Together, those rules let a function read this data without taking ownership or using it after cleanup.

Some Rust types permit carefully controlled changes through shared references. We do not need those types for this request. The [screen chapter](11-drawing-and-navigation.md) introduces a mutable reference when selection actually needs to change.

## Why store IDs?

Reading a request is a brief operation. Remembering which request you selected lasts across many screen updates. We can keep an ID in the navigation state and look up the request whenever we need to draw it.

That ID identifies an entry in the session, the collection of requests loaded from one capture. Keeping a number here avoids retaining a reference between screen updates. The next collection chapter explains how IDs and lookup work, including what happens when an ID has no matching request.

## Try the compiler's rule

Put the two URL statements from the first example inside a small program. Add `println!("{}", url);` after the move. Predict what the compiler will say before running it.

Then compare borrowing the original string with cloning it. Which version creates another text buffer? Which value owns that buffer? Use the diagram to explain your answer.

<details>
<summary>Show the exercise explanation</summary>

Using `url` after the move fails because ownership transferred to `request_url`. A shared borrow lets you read the existing string while its owner remains valid. Calling `clone()` creates an independent string with its own buffer. That copy costs an allocation and a copy of the text bytes.

</details>

## Where this appears in Netscope

The session model [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197) owns request data, and the view [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66) borrows it while drawing. You can read [the Rust book's ownership chapter](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html) for a longer treatment of moves and cleanup.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### What a move does

**Level:** Newcomer


After `let url = String::from("https://example.test/api/books"); let request_url = url;`, can you still print `url`? What happened to the text?

<details>
<summary>Answer</summary>

No. The assignment moves ownership of the `String` to `request_url`, and the compiler rejects any later use of `url`. The text was not copied. It stays in the same buffer, and `request_url` is now responsible for releasing it. The rule exists so two owners never both try to free the same buffer.

**Common wrong answer.** "Assignment copies the string, so both names work." That is true for small number types, but a `String` owns heap memory and is moved.

**Learn more:** [A string owns its bytes](../chapters/03-storing-requests.md#a-string-owns-its-bytes) · [Follow the move](../chapters/03-storing-requests.md#follow-the-move)

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

### Borrowing versus cloning

**Level:** Newcomer


A drawing function needs to read a request's URL. A replay draft needs a URL it can edit. Which one should borrow and which one should clone, and why?

<details>
<summary>Answer</summary>

The drawing function should borrow with a shared reference such as `&Request`. It reads the existing bytes while the session stays the owner, and nothing is copied. The draft should clone. It must change its text while the captured URL stays as recorded, so it needs its own buffer. Cloning costs an allocation and a copy of the bytes, which is why Netscope clones only what the draft needs, not the whole capture.

**Learn more:** [Borrow while drawing](../chapters/03-storing-requests.md#borrow-while-drawing) · [Give the draft its own text](../chapters/16-preparing-a-request.md#give-the-draft-its-own-text)

</details>

### Why the view stores request IDs

**Level:** Newcomer


The screen must remember which request you selected across many redraws. Why does Netscope store a request ID for this instead of a reference to the request?

<details>
<summary>Answer</summary>

A reference is suited to short access, such as reading during one draw. Remembering a selection lasts across many key presses and redraws. An ID is a plain number the view state can own, and the program looks the request up in the session whenever it needs it. The lookup returns an `Option`, so an ID with no matching request becomes a visible absence instead of a crash.

**Learn more:** [Why store IDs?](../chapters/03-storing-requests.md#why-store-ids) · [`Session::request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L264)

</details>
