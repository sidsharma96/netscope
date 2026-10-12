# How can a program remember a request after it closes?

You spend a few minutes preparing a request for a local book server. The address is right, the JSON body contains the title you wanted, and review shows the expected byte count. Then you close the program. Tomorrow, you would like to start with that same request rather than type it again.

The editable String values belonged to the running program. A later run does not inherit them. To carry information across that boundary, the program must write a representation of the request somewhere that survives its exit. A file gives the next run bytes it can read; it does not preserve the original Rust objects or bring a network connection back to life.

## Choose what should survive

A saved request needs its destination, method, headers, and body. A name helps you recognise it. It does not need the screen's scroll position or a worker that once sent it. Nor should opening the file count as permission to send again: the destination or your intention may have changed since yesterday.

This drawing separates the two runs. The middle box contains file bytes, while the outer boxes contain values owned by running programs. The arrows describe explicit save and open actions.

```text
Today's process              Local file               Tomorrow's process

Editable request ──save──→ JSON bytes on disk ──open──→ New editable request
       │                                                      │
Review, then Send                                      Review, then Send
       │                                                      │
Today's response                                       A new response
```

No arrow carries today's response into tomorrow's request. A saved draft is a recipe for an operation, not evidence that the operation happened. Netscope therefore opens it without a captured response and does not offer a comparison with an invented observation.

A file also outlives the privacy boundary of a terminal screen. Someone who can read it can read its body and URL. Netscope omits known credential headers when saving and previews what remains. That omission cannot identify every secret: an ordinary-looking query parameter or JSON member could contain one. Read the preview as the contents of a plain-text file you are choosing to keep.

## Turn values into a representation

Serialization means expressing program values in a format that can be stored or transmitted. Deserialization goes the other way: reading that representation to construct values. JSON is useful here because it names fields and can be inspected in a text editor. Serde supplies the Rust machinery that connects data types to formats; a format-specific library handles JSON's spelling. The [Serde overview](https://serde.rs/) describes this division.

Here is a smaller example with just a note title. It demonstrates the conversion in memory, without creating a file or actually restarting the program.

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Note {
    title: String,
}

fn main() -> Result<(), serde_json::Error> {
    let note = Note { title: String::from("café") };
    let encoded = serde_json::to_string(&note)?;
    drop(note);

    let reopened: Note = serde_json::from_str(&encoded)?;
    assert_eq!(reopened.title, "café");
    Ok(())
}
```

The `use` line brings two names into scope. The `#[derive(...)]` attribute asks Rust to generate implementations for the listed traits. A trait describes a capability a type supports; here the capabilities are being serialized and deserialized. The generated implementations know that a Note contains a String field named title.

`to_string(&note)` borrows the note and creates a separate String containing JSON. `drop(note)` then consumes the original note and releases its owned text. The encoded String remains alive because it is a different value. Reading it with `from_str` constructs a new Note with its own title. The annotation `reopened: Note` tells Rust which type the JSON should become.

Both conversion calls return a Result: either a successful value or an error. The `?` after each call extracts success or returns the error from main immediately. The return type names `serde_json::Error` as the possible failure, while `()` means successful completion carries no additional value. The last assertion checks that the newly constructed title has the expected text.

A request body that itself contains JSON is still a String inside the saved request. Quotes in that String must be escaped in the outer file. Reading the file removes that outer escaping and restores the body text. It does not parse and rewrite the body's number spelling or whitespace.

## A readable file can still be the wrong request

Suppose a file contains a well-formed JSON object with a method named DELETE. The parser can understand its punctuation, but this Netscope profile supports only GET and POST. Successful parsing and a supported request are separate checks.

A version field gives the format an explicit identity. Version 1 tells the reader which fields and meanings to expect. If a future version changes those meanings, an older program should report that it does not understand the version. Guessing could produce a different request from the one you meant to keep.

Netscope checks the file's size, structure, version, and request values before opening an editor. It rejects unexpected fields rather than quietly ignoring a possible misspelling. Those checks make failure recoverable: the original file remains available to inspect, and no network operation has begun.

## Replacing a file has a vulnerable middle

Imagine that yesterday's file is valid. Today you save an edited body, but a write fails halfway through. If the program had erased the old file first, you could be left with neither yesterday's request nor today's complete replacement.

The safer sequence keeps the old destination while building a separate file beside it. Read the diagram from top to bottom; replacement happens only after the new content has been written and synchronized.

```text
book.json: yesterday's complete request
    │
    ├── create book.json.netscope-tmp without replacing an existing file
    ├── write today's complete representation
    ├── synchronize the new file's contents
    │       failure here → keep yesterday's file
    └── rename the completed temporary file to book.json
            success → the destination names today's complete request
```

Rust's File value owns an open-file handle: access that the operating system has granted to a file. Leaving its scope closes that handle, but closing alone does not report every storage failure. The explicit synchronization step can return an error before replacement. See the standard library's [File and synchronization documentation](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all).

Keeping the temporary file beside the destination also keeps the rename within the same filesystem. Rename can replace an existing file, but it can fail, so the program must still handle its Result. The [rename documentation](https://doc.rust-lang.org/std/fs/fn.rename.html) explains the platform restrictions. This sequence protects against an ordinary partial-write failure; it is not a promise that every filesystem survives sudden power loss the same way.

If the process stops abruptly before replacement, a temporary file may remain. Netscope refuses to overwrite it automatically. That makes the conflict visible and leaves you a chance to inspect what was written.

## Predict what will return

Prepare a local request, press w from its draft, choose a name and path, and inspect the save preview. After confirming, close Netscope and reopen the file using the [saved-draft walkthrough](../DEVELOPMENT.md#save-and-reopen-a-draft).

Before opening, predict three things: whether the body will still contain its line breaks, whether an authorization header will be present, and whether a request will be sent. The body should survive exactly; a known credential header should be absent; opening alone should send nothing. Those outcomes follow from what the file contains and what opening is allowed to do.

The [saved-draft contract](../contracts/m3-saved-drafts.md) records the precise fields, limits, and filesystem assumptions. The [saved-value model](../../crates/netscope-core/src/saved.rs) handles representation and validation, while the [file operations](../../crates/netscope-tui/src/saved_file.rs) handle reading and replacement.
