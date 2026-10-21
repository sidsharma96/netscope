# How does a server receive the text we submit?

You want to create a book entry on a local server. The title is `café`. Typing those four letters into a field feels like a complete description of the input, but the connection carries bytes. Before sending, the client must decide how those letters become bytes, how the server will interpret them, and where the request body ends.

A POST request carries the data we want the server to process. The destination determines what processing means: one endpoint might create a record, another might validate text and return an explanation. Selecting POST does not establish what that particular server will do. That is why Netscope keeps the destination, method, and body together in review before the explicit Send action.

## Text already has a byte representation

Rust's String stores text using [UTF-8](../GLOSSARY.md#utf-8). UTF-8 is an encoding: a rule for representing Unicode text with bytes. ASCII letters such as c, a, and f each use one byte. The precomposed é in this example uses two. The title therefore occupies five bytes even though it looks like four letters.

The drawing shows the example's UTF-8 bytes in hexadecimal, a notation that uses digits 0–9 and letters a–f to write byte values compactly. Each pair names one byte. It is an illustration of this text, not a record of a network exchange.

```text
Text                  c       a       f       é
                      │       │       │       │
UTF-8 bytes           63      61      66      c3 a9
                      └────────── 5 bytes ─────────┘

Content-Length: 5     counts body bytes, not displayed letters
```

We do not need to encode a String again to inspect these bytes. Its `as_bytes()` method lends a byte slice, a view of the existing storage. The String remains responsible for keeping that storage alive.

```rust
fn body_length(text: &str) -> usize {
    text.as_bytes().len()
}

fn main() {
    let title = String::from("café");
    let bytes = title.as_bytes();

    assert_eq!(bytes, &[0x63, 0x61, 0x66, 0xc3, 0xa9]);
    assert_eq!(body_length(&title), 5);
    assert_eq!(title, "café");
}
```

The parameter `text: &str` accepts borrowed UTF-8 text. The `&` in `body_length(&title)` lends access to the existing String; Rust can view its contents as the string slice the function expects. The function returns a `usize`, the integer type used for lengths and indexes. Calling `len()` on the borrowed byte slice counts its bytes. Calling `len()` directly on a String would also count bytes, but the extra step here makes the representation visible.

In the assertion, `0x` introduces a hexadecimal number. Square brackets make an array containing five byte values, and the leading `&` lends that array for comparison. Reading through `bytes` has not moved or changed `title`, so the last assertion can still inspect the original String. No second text buffer is needed just to count the bytes.

## A length describes the bytes we actually send

[Content-Length](../GLOSSARY.md#content-length) is an HTTP field that describes a content length in bytes. Counting letters would make the field wrong for this example. A receiver using a too-small declared length could reach what it believes is the end before receiving the entire intended body. The client must keep framing information consistent with the actual outgoing representation. HTTP defines this byte-count meaning in its [Content-Length rules](https://www.rfc-editor.org/rfc/rfc9110.html#section-8.6).

Netscope calculates this field from the reviewed UTF-8 body. It does not let a captured Content-Length override the new count. A body copied from a recording may have been edited, and its previous length would no longer describe it. Empty plain text is also a definite body: for POST, its declared length is zero.

GET stays bodyless in this replay profile. Changing the method to POST begins a new plain-text body. Changing back to GET clears it. This is a rule of the editor's supported profile, so the interface does not quietly keep text that it would then omit when sending.

## Content type gives the server an interpretation

The byte sequence alone does not tell the server whether it should read a title, a [JSON](../GLOSSARY.md#json) document, or some other representation. Content-Type supplies a [media type](../GLOSSARY.md#media-type), a label describing how the content should be interpreted. Netscope can submit UTF-8 plain text or JSON.

These two bodies submit different data:

```text
Plain text                     JSON
Content-Type:                  Content-Type:
text/plain; charset=utf-8      application/json

café                           {"title":"café"}
```

The left body is just the title. The right body is a JSON object with a member named title. Its braces, quotes, and colon are also body bytes and count toward Content-Length. JSON used between interoperable systems is encoded in UTF-8; its [encoding rules](https://www.rfc-editor.org/rfc/rfc8259.html#section-8.1) distinguish that byte representation from the structure expressed by JSON punctuation.

Changing a media-type label does not transform arbitrary text into that format. If you enter `café` and then choose JSON, the editor keeps your text, but review will reject it. A JSON string needs quotes, and an object needs the appropriate surrounding structure. Preserving the invalid draft gives you something to repair instead of discarding what you typed.

## Validation should not rewrite the submission

Consider the JSON body `{"price":1.00}`. Parsing it can establish that its structure is acceptable to the parser. Turning the parsed value back into text might change number spelling or whitespace, even if the resulting document describes the same value for some applications. A replay tool should not silently make that extra decision.

Netscope therefore parses JSON only to validate it. It keeps the original String and sends its original UTF-8 bytes. Whitespace, line breaks, and number spelling remain as entered. Validation requires one complete JSON value within the supported size, numeric, and nesting limits. These limits bound the work and stay visible as an explanation when review fails.

Passing that check does not establish that the server will accept the content. The server may require a title member, reject an unknown member, or interpret repeated member names in a particular way. The syntax check does not enforce the server's data rules or resolve duplicate names on its behalf.

## Review gives one operation stable data

While editing, text is provisional. Pressing Esc should abandon an unfinished edit; saving should replace the draft text. Opening review validates the saved draft. Only the separate Send action gives a worker its own reviewed values.

Read the following arrows as changes in responsibility for data. The editable field and draft are separate values, while the worker's review fixes what one operation is allowed to submit.

```text
Editing field ──save──→ Draft ──validate──→ Review
      │                  │                    │
      Esc                URL change           Send
      │                  │                    │
Discard edit        Clear body and       Worker owns reviewed text
                    selected headers          │
                                         Build owned body bytes
                                              │
                                         Write while deadline
                                         and Cancel remain active
```

The String's bytes can be borrowed while calculating a length or building the transport's owned body buffer. The connection then has data that stays alive throughout its asynchronous writes. It does not depend on a text field that the user might later replace. The original capture remains unchanged throughout this process.

Changing a destination clears the body as well as header choices. Text selected for one server should not quietly carry to another. The chosen method and format remain, so clearing a JSON body makes the draft incomplete until you supply a valid JSON value again. The editor explains that reset immediately.

Once sending starts, the same total deadline covers both writing the request and reading the response. A connection can stall while accepting the upload, before any response exists. Cancelling stops local waiting and disposes of that exchange. It cannot establish that the server received nothing, or reverse work the server has already performed. The result's reviewed body count describes the intended submission, not an acknowledgement of how many bytes the server processed.

## Try counting before sending

Use the [local POST walkthrough](../DEVELOPMENT.md#try-a-local-post) with the supplied echo server. From a draft, m changes the method, t switches plain text and JSON, and b opens body editing. Inside that field, Ctrl+n inserts a newline, Tab inserts a tab, Enter saves, and Esc abandons the edit. Letters such as q, m, and t are ordinary text while you are typing.

Start with plain text `café` and predict the reviewed byte count. Then use JSON `{"title":"café"}` and account for the extra punctuation. Review the local destination, method, content type, length, and body before sending. The server's response reports the received bytes, which lets you compare your prediction with a controlled observation.

The [body contract](../contracts/m3-request-bodies.md) records exact limits and supported captured representations. The draft model [`ReplayDraft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L65) owns and validates text; the transport [`request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-net/src/lib.rs#L99) constructs the request from that review. Those are separate responsibilities because a valid editable value and a completed network operation answer different questions.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Counting the bytes in café

You POST the plain-text body `café`. What Content-Length does Netscope send, and why?

<details>
<summary>Answer</summary>

Five. Content-Length counts bytes, not letters. Rust stores text as UTF-8, where `c`, `a` and `f` each take one byte and the precomposed `é` takes two. Netscope computes the length from the reviewed body bytes and never reuses a captured Content-Length, since an edited body would make it wrong. A JSON body such as `{"title":"café"}` also counts its braces, quotes and colon.

**Common wrong answer.** "Four, one per character."

**Learn more:** [Text already has a byte representation](../chapters/19-submitting-text.md#text-already-has-a-byte-representation) · [A length describes the bytes we actually send](../chapters/19-submitting-text.md#a-length-describes-the-bytes-we-actually-send)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="what-stops-a-capture-from-sending-traffic"></a>

[What stops a capture from sending traffic?](../questions/practitioner.md#what-stops-a-capture-from-sending-traffic)

<a id="what-happens-between-a-draft-and-the-wire"></a>

[What happens between a draft and the wire?](../questions/practitioner.md#what-happens-between-a-draft-and-the-wire)

</details>
