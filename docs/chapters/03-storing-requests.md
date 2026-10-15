# Who keeps a request alive?

You select a request in the list and open its details. Both screens need the same URL, headers, and response body. Copying all of that data for each screen would use more memory and create more values to keep consistent.

We can store the request once and let each screen read it. Rust asks us to make one part of that arrangement explicit: who is responsible for keeping the data alive, and who is only using it for a while?

## A string owns its bytes

Text occupies memory. Rust's `String` type manages a region of memory containing UTF-8 text. UTF-8 represents characters as bytes, and a character can use more than one byte. A region reserved for this text is often called a buffer.

A `String` keeps track of the buffer's location, the number of bytes currently used, and the available capacity. It also takes responsibility for releasing the buffer when it is no longer needed. That responsibility is what ownership means in this example.

Consider giving a URL another name:

```rust
let url = String::from("https://example.test/api/books");
let request_url = url;
```

`String::from` creates an owned string from the quoted text. The second line transfers that string into `request_url`. Rust calls this a move. The text stays in the same allocated buffer, and `request_url` becomes responsible for it. The old name, `url`, can no longer be used to access the moved value.

This rule prevents two independent owners from both trying to release the same buffer. If we want a second owned copy of the text, we can request one with `clone`. That copies the string's contents and gives the copy its own storage.

## Follow the move

Step through the diagram and watch the owner change while the text buffer remains in place. The drawing explains ownership; it does not promise a particular physical memory address or compiler layout.

<!-- ownership-demo -->

A local variable belongs to a scope, the region of code in which its name is available. A function body is one example. When the owning value reaches the end of its lifetime, Rust normally runs its cleanup. For `String`, that releases the buffer.

Moving a value out of a function can give ownership to the caller. Leaving a function therefore does not always mean destroying everything it used. The useful question is where ownership went.

## Borrow while drawing

A screen needs permission to read the URL, but it does not need to take over its storage. A shared reference gives it that access. Rust writes a reference with `&` in front of the type.

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

<!-- ownership-answer -->

## Where this appears in Netscope

The session model [`Session`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L197) owns request data, and the view [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66) borrows it while drawing. You can read [the Rust book's ownership chapter](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html) for a longer treatment of moves and cleanup.
