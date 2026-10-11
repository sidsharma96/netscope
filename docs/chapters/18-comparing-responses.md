# What does it mean for two responses to match?

The capture shows status 200 and a body containing `catalog`. Your new request also receives status 200, but the connection stops after `cat`. The three received letters agree with the beginning of the capture. Calling the responses identical would still be wrong: the new operation did not reach the end of its body.

A useful comparison tells us what agrees, what differs, and what remains unknown. These are different answers. The job is to preserve those distinctions as we move from a screen full of values to a conclusion about them.

## Establish which observations are beside each other

The left column belongs to the recorded request. The right column belongs to a new operation whose destination and headers you chose in the draft. An unchanged source request ID connects the two observations, but does not make their conditions identical. You may have replaced a public address with a local one, omitted a header, or sent the request after the server's data changed.

Netscope therefore begins the comparison with both destinations and status values. The recorded duration and the new elapsed duration have separate labels. A smaller number on the right is something you can observe, but the two numbers alone do not establish why it is smaller. This comparison does not calculate a speedup or attribute a change to a protocol.

A status can be compared only when both observations contain one. In a HAR recording, zero means no status was recorded. A replay that fails before receiving headers also has no status to compare. Neither is an HTTP response with status zero, and neither should be quietly converted to a successful empty response.

## Absence is different from emptiness

A captured body can be missing even when the capture includes a status and a reported size. That tells us something about what was recorded, not that the server sent no bytes. By contrast, an explicitly stored empty body gives us a representation with length zero.

The diagram separates these cases. Each box describes retained data, and the arrow names a conclusion that those data support. No arrow reconstructs anything the exporter omitted.

```text
Illustrative observations

No captured body ───────────────→ Cannot compare body bytes

Stored empty body +
complete empty replay ─────────→ Stored bytes match, length 0

Stored "catalog" +
partial replay "cat" ──────────→ Three shared bytes match
                                 The replay is still incomplete
```

Even when a complete replay matches every available captured byte, Netscope says that the stored bytes match. A capture's retained text is not proof that the exporter kept the entire original response. A preview adds another boundary: the screen may show only the beginning of a large stored representation. The comparison examines the available bytes beyond that preview too, so a difference after the visible excerpt is still reported.

## Ask a smaller question about a partial body

A prefix is the beginning of a sequence. The common comparison region is as long as the shorter available sequence. If one side contains three bytes and the other contains seven, there are three positions at which both sides have a byte.

We can ask whether those positions agree without claiming anything about the remaining four. If the overlapping bytes differ, we can report the first differing position. If no bytes overlap, there is no prefix evidence to compare. The outcome that stopped the replay stays visible beside this narrower conclusion.

Rust lets a function inspect this region through a slice. A slice is a view of consecutive elements owned elsewhere. `&[u8]` means a shared reference to a slice of bytes: `u8` is an unsigned integer with values from 0 through 255. The function can read the bytes without making another body-sized allocation or taking ownership of either body.

This small example answers only the prefix question. It deliberately leaves out response status, encoding, and completion; those are separate reasons the application may decline a body comparison.

```rust
fn shared_prefix_matches(left: &[u8], right: &[u8]) -> Option<bool> {
    let shared = left.len().min(right.len());
    if shared == 0 {
        return None;
    }
    Some(left[..shared] == right[..shared])
}

fn main() {
    assert_eq!(shared_prefix_matches(b"catalog", b"cat"), Some(true));
    assert_eq!(shared_prefix_matches(b"catalog", b"car"), Some(false));
    assert_eq!(shared_prefix_matches(b"catalog", b""), None);
}
```

The `main` function runs three checks on the helper function. The `b` before a quoted ASCII string makes a byte string, so the example compares bytes rather than doing text-aware matching. `len()` gives each slice's length, and `min()` chooses the smaller number. The range `..shared` starts at zero and stops before `shared`. Since that number is no larger than either length, both selected regions stay within their slices. Equality reads those regions; it does not copy their contents into new vectors.

`Option<bool>` gives three possible answers. `Some(true)` means the shared bytes agree, `Some(false)` means at least one differs, and `None` means there are no shared bytes to check. Returning ordinary `true` for two empty slices would be valid for a different question about equality. Here it would hide the absence of prefix evidence, which is the distinction we want the reader to see.

The first assertion returns true even though `catalog` and `cat` have different lengths. That is the most important limit of this function. Its name and output must be read as an answer about a shared prefix, not as permission to label the responses equal.

## Bytes need a common meaning

A displayed word can be represented in several ways. UTF-8 represents text as bytes; some characters need more than one byte. Netscope's reported difference position is a zero-based byte position, not a character number. Position zero names the first byte. A difference inside a multi-byte character should not be interpreted as the same numbered character on screen.

There is another distinction in a capture. Base64 lets an exporter place binary data into a text-based file. Decoding that container representation gives us bytes we can retain. HTTP content encoding, such as gzip, is a separate transformation of the response content. HAR describes text after HTTP decoding and conversion into UTF-8, with optional base64 encoding for storage. A new replay retains bytes without performing those text conversions or content decompression. Decoding base64 alone therefore does not establish that the two representations describe the same stage of processing. See the [HAR content description](https://webperfwg.org/specs/HAR/Overview.html#content).

The replay client asks for identity encoding and does not decompress a server's response. A server can still return encoded content. If either observation reports a content encoding other than identity, the current comparison declines to judge body bytes. Both previews remain inspectable, but their apparent similarity is not promoted into a comparison result. Supporting more representations would require an explicit decoding policy and limits on the decoded data.

## Match header names without erasing their values

`Content-Type` and `content-type` name the same HTTP field; field names are case-insensitive. Their values need their own rules. Netscope groups names without changing the stored value bytes, and compares repeated values in their recorded order. This is a comparison of stored values, rather than an attempt to determine every field's semantic equivalence. The distinction follows HTTP's separate rules for [field names and field values](https://www.rfc-editor.org/rfc/rfc9110.html#section-5).

For example, if two values under the same name appear as `one`, then `two` on the left and `two`, then `one` on the right, this comparison reports a difference. It does not sort those values to manufacture a match. Nor does it combine repeated fields or split values at commas. A name absent from one stored list is labelled there; if no replay headers arrived at all, the comparison says that instead.

Values remain hidden until you press v. Revealing them is a display choice and does not change the comparison. Esc returns to the result and resets masking, so a later visit does not unexpectedly reveal values you previously inspected.

## Try explaining the difference

Follow the [local-send walkthrough](../DEVELOPMENT.md#try-a-local-send), then press c on the result. The capture appears on the left and the new response on the right. Up and Down scroll both columns together. Find the two destinations, the status comparison, the body conclusion, and the header values. Press v to reveal headers, then Esc to return to the result.

Before running the small Rust example, predict its answer for `b"cat"` and `b"catalog"`. Now replace the second argument with `b""`. Explain why the function's answer changes even though neither case proves two whole responses equal.

The [comparison rules](../contracts/m3-response-comparison.md) record the exact scope. The [pure comparison code](../../crates/netscope-net/src/comparison.rs) computes conclusions from borrowed observations; the [paired view](../../crates/netscope-tui/src/comparison_view.rs) handles wrapping, masking, and scrolling. Neither path creates a network operation.
