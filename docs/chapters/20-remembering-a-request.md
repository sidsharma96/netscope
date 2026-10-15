# How can a program remember a request after it closes?

You spend a few minutes preparing a request for a local book server. The address is right, the [JSON](../GLOSSARY.md#json) body contains the title you wanted, and review shows the expected byte count. Then you close the program. Tomorrow, you would like to start with that same request rather than type it again.

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

[Serialization](../GLOSSARY.md#serialization) means expressing program values in a format that can be stored or transmitted. Deserialization goes the other way: reading that representation to construct values. JSON is useful here because it names fields and can be inspected in a text editor. Serde supplies the Rust machinery that connects data types to formats; a format-specific library handles JSON's spelling. The [Serde overview](https://serde.rs/) describes this division.

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

The `use` line brings two names into scope. The `#[derive(...)]` attribute asks Rust to generate implementations for the listed traits. A [trait](../GLOSSARY.md#trait) describes a capability a type supports; here the capabilities are being serialized and deserialized. The generated implementations know that a Note contains a String field named title.

`to_string(&note)` borrows the note and creates a separate String containing JSON. `drop(note)` then consumes the original note and releases its owned text. The encoded String remains alive because it is a different value. Reading it with `from_str` constructs a new Note with its own title. The annotation `reopened: Note` tells Rust which type the JSON should become.

Both conversion calls return a Result: either a successful value or an error. The `?` after each call extracts success or returns the error from main immediately. The return type names `serde_json::Error` as the possible failure, while `()` means successful completion carries no additional value. The last assertion checks that the newly constructed title has the expected text.

A request body that itself contains JSON is still a String inside the saved request. Quotes in that String must be escaped in the outer file. Reading the file removes that outer escaping and restores the body text. It does not parse and rewrite the body's number spelling or whitespace.

## A readable file can still be the wrong request

Suppose a file contains a well-formed JSON object with a method named DELETE. The parser can understand its punctuation, but this Netscope profile supports only GET and POST. Successful parsing and a supported request are separate checks.

A version field gives the format an explicit identity. Version 1 tells the reader which fields and meanings to expect. If a future version changes those meanings, an older program should report that it does not understand the version. Guessing could produce a different request from the one you meant to keep.

Netscope checks the file's size, structure, version, and request values before opening an editor. It rejects unexpected fields rather than quietly ignoring a possible misspelling. Those checks make failure recoverable: the original file remains available to inspect, and no network operation has begun.

## Why use a file instead of a database?

For one named request, a versioned JSON file is easy to inspect, copy, and reopen without starting another service. A database could also run locally and would become useful if we needed to search a large saved collection or update several records together. It would introduce a schema, the rules for stored records, and a migration policy for changing those rules.

The current file format makes a smaller promise. It carries one request's values between runs. It does not coordinate two people editing that request, remember a history of revisions, or guarantee that an unknown future format can be read. The version field lets us reject a format we do not understand instead of guessing its meaning. If saved collections or concurrent editing become requirements, the storage decision should change with them.

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

Keeping the temporary file beside the destination also keeps the [rename](../GLOSSARY.md#atomic-replacement) within the same filesystem. Rename can replace an existing file, but it can fail, so the program must still handle its Result. The [rename documentation](https://doc.rust-lang.org/std/fs/fn.rename.html) explains the platform restrictions. This sequence protects against an ordinary partial-write failure; it is not a promise that every filesystem survives sudden power loss the same way.

If the process stops abruptly before replacement, a temporary file may remain. Netscope refuses to overwrite it automatically. That makes the conflict visible and leaves you a chance to inspect what was written.

## Predict what will return

Prepare a local request, press w from its draft, choose a name and path, and inspect the save preview. After confirming, close Netscope and reopen the file using the [saved-draft walkthrough](../DEVELOPMENT.md#save-and-reopen-a-draft).

Before opening, predict three things: whether the body will still contain its line breaks, whether an authorization header will be present, and whether a request will be sent. The body should survive exactly; a known credential header should be absent; opening alone should send nothing. Those outcomes follow from what the file contains and what opening is allowed to do.

The [saved-draft contract](../contracts/m3-saved-drafts.md) records the precise fields, limits, and filesystem assumptions. The saved-value model [`SavedDraft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L10) handles representation and validation, while the file operations [`save`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/saved_file.rs#L36) handle reading and replacement.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Saving a draft

**Level:** Newcomer


You save a draft and reopen it tomorrow. What comes back, what is left out, and does opening it send anything?

<details>
<summary>Answer</summary>

The file is versioned JSON holding the name, URL, method, body format, body and headers. Known credential headers such as `Authorization` and `Cookie` are left out. Secrets elsewhere, for example in the URL's query string or the body, are saved as written. Opening the file validates it and starts an editor with no captured response. It never sends anything. Netscope writes a temporary file, syncs it, then renames it over the old one, so a failed write keeps yesterday's file.

**Learn more:** [Choose what should survive](../chapters/20-remembering-a-request.md#choose-what-should-survive) · [Replacing a file has a vulnerable middle](../chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle) · [`sensitive_saved_header`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L27)

</details>

### What does opening a saved draft check?

**Level:** Practitioner


A saved draft is a small JSON file. When you open one with `netscope --draft`, what does Netscope check before it shows the editor, and why does it reject a file with an extra field instead of ignoring the field?

<details>
<summary>Answer</summary>

`SavedDraft::decode` first refuses a file larger than 512 KiB. It then parses the JSON into a structure marked `deny_unknown_fields`, so a misspelled or unexpected field fails the load instead of being dropped silently. A dropped field could mean the request you reopen differs from the one you saved, and you would not be told.

`to_draft` then checks the meaning of the values. The version must be 1, the name must hold 1 to 128 bytes with no control characters, the method must be GET or POST, and a GET must use the plain, empty body. Headers are limited to 100 entries and 32 KiB in total, each must pass the same header rules as an edited draft, and a credential header such as `Authorization` or `Cookie` rejects the file. Saving runs the same checks through `encode`, so Netscope never writes a file it would refuse to open. The editor opens with no automatic send and no link to a captured request.

**Learn more:** [A readable file can still be the wrong request](../chapters/20-remembering-a-request.md#a-readable-file-can-still-be-the-wrong-request) · [`decode`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L139-L148) · [`to_draft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L74-L121)

</details>

### What a saved draft keeps, leaks, and survives

**Level:** Expert


What does a saved draft contain, what does Netscope strip, what can still leak, and how crash-safe is the write? What happens when the destination is a symlink or a stale temporary file exists?

<details>
<summary>Answer</summary>

The file is pretty-printed JSON with `version: 1`, a name, the URL, method, body format, body, and headers. Decoding rejects unknown fields and files over 512 KiB, and every load and save runs the full draft review.

Nine header names are dropped on save: Authorization, Proxy-Authorization, Cookie, Cookie2, Set-Cookie, X-Api-Key, Api-Key, X-Auth-Token, and X-Access-Token. A file that contains one of them is rejected on load, not silently accepted. The leak surface remains: the URL is saved verbatim, including query-string tokens and the fragment, and so are the body and any secret header outside the list, such as `X-CSRF-Token`.

The write refuses a destination that is not a regular file, including a symlink. It creates `<name>.netscope-tmp` with `create_new` and mode 0600, writes it, calls `sync_all`, and renames it over the destination. A failure removes the temporary file. A leftover temporary file makes `create_new` fail, so the save is refused. Two weaknesses remain: the parent directory is not synced after the rename, so a power loss can undo it, and the symlink check and the rename are separate steps.

**Learn more:** [Replacing a file has a vulnerable middle](../chapters/20-remembering-a-request.md#replacing-a-file-has-a-vulnerable-middle) · [Secrets inside capture and draft files](../chapters/34-defending-the-design.md#secrets-inside-capture-and-draft-files) · [`sensitive_saved_header`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/saved.rs#L27) · [`replace`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/saved_file.rs#L44-L73)

</details>
