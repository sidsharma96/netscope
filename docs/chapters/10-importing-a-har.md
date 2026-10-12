# Turning a capture into requests

You open the recording of our book request. The file contains text, but the screen needs a request with a method, a URL, a response, and timings it can interpret. How do we get from one to the other without losing information?

This is the importer's job. It reads the file, checks the fields the application needs, and builds the values the rest of the program will use. The interesting part is deciding what we can accept and what each accepted value means.

## Reading structure comes before understanding meaning

JSON can tell us that a field contains a number. It cannot tell us whether that number is a valid HTTP status. A parser reads JSON's structure; validation then applies the capture's rules.

Suppose the file contains `"status": 200`. Parsing recognises a field named status and the number 200. Validation accepts it as a response status. If the number is 700, the JSON is still readable, but the status is outside the range accepted by our inspector.

The difference matters for errors. A broken JSON document and an invalid field both prevent import, but they fail for different reasons. An error should locate the problem without asking the reader to guess which check failed.

<!-- import-flow -->

Each step adds understanding. None needs to send a request to the address in the capture; we are examining an existing record.

## Keep the original observation

Validation does not give us permission to tidy away inconvenient distinctions. Two headers with the same name remain two headers. An omitted body remains unrecorded. An optional timing of `-1` becomes an unavailable timing, while a zero remains a recorded duration.

We also keep the exporter's numbers when they disagree. If the available timing stages do not add up to the reported total, a warning makes that disagreement visible. Replacing the total with our own sum would erase the observation we are trying to investigate.

There is a similar reason to retain timestamp text alongside its parsed value. Two timestamps can use different time-zone offsets while naming the same instant. We compare the instants when sorting, and keep the original text for inspection.

## When two recordings describe the same body differently

Suppose a form submits two tags, one called `one` and another called `two`. An export may retain the body as text: `tag=one&tag=two`. Here each equals sign separates a field name from its value, and the ampersand separates the two fields. The repeated name matters: replacing the pair with a single value would lose one tag.

Another export may include a list of those fields as well as the text. The list is convenient for reading, while the text preserves the exporter's spelling. Having both is useful. Rejecting the whole request because it has two descriptions would prevent us from inspecting information the browser deliberately recorded.

Netscope retains each description independently. If the text and fields disagree, it does not silently rewrite either one. Nor does a field list establish the original bytes sent over the connection: file uploads, for example, need separators and file contents that the list may not contain. The inspector shows what was retained and leaves that limit visible.

Rust's enum, introduced in the [chapter about missing values](09-options-results-and-matching.md), lets us name the possible shapes of this data. A request body can be unrecorded, text alone, fields alone, or both text and fields. Each alternative owns the values it needs. When the inspector borrows the combined alternative, it can read both descriptions without moving either out of the request. Closing the inspector therefore leaves the imported record available for the next visit.

There is a second reason browser recordings can differ: the browser may still be collecting information when the export begins. A response body can appear in a record whose status is still zero or whose protocol label is empty. Zero here means no HTTP status was recorded; it does not tell us that the request failed. Likewise, an omitted response media type means we cannot identify it from that field. A media type is a label such as `text/plain` that describes how content should be interpreted.

These are specific acceptance rules, not permission to ignore every malformed field. An empty protocol string can represent an unrecorded label; a number in its place remains an error. A missing expiry date for a cookie tells us that the export supplies no date, not how long the browser will keep the cookie. Before accepting a variation, we decide what the reader can safely conclude from it.

## Decide when a failure blocks the request

An invalid required wait duration prevents us from accepting that request. A missing optional response body does not. We need a deliberate rule for each field, because accepting a capture and recovering every piece of it are different questions.

Consider a body stored as base64. Base64 represents bytes using a restricted alphabet of text characters. It lets a JSON string carry data that might not be readable text. Decoding reverses that representation.

If decoding succeeds, we check whether the bytes form valid UTF-8 text. Readable text can be displayed as text; other bytes receive a binary preview. If decoding fails, we can still inspect the method, URL, headers, and timings. The body remains stored in its encoded form with an explanation of the failure.

This is a recoverable body warning. In contrast, an invalid required field rejects the capture. These policies determine what the reader sees; the exact acceptance rules live in the [input contract](../contracts/m1-har-inspection.md).

## Build a complete session before displaying it

A session contains the requests loaded from one capture. During import, each accepted request goes into a vector in source order. Ownership moves into that vector, including the request's owned strings and body representation.

If a later required field fails validation, the function returns an error. The temporary vector has not become a usable session, and Rust cleans up its owned values when they leave scope. The application reports the failure instead of showing a partial list as though import had succeeded.

After every entry passes, the session can support a chronological view. A separate list of IDs determines display order, as described in the [collection chapter](08-structs-vectors-and-ids.md). Sorting the display does not change which entry an ID identifies.

In the implementation, `Result` expresses whether import produced a session or an error. The `?` operator stops a sequence of checks when a required step fails. Those language features make the policy visible in code; the policy comes from what we need the inspector to mean.

## Put a limit on the work

A local file can still be too large to process comfortably. Netscope limits input bytes, the number of entries, and the amount of decoded body data. Checking during reading also handles a file whose size changes after it is opened.

A body preview has a different purpose. We may retain more information than fits on a screen, so the preview shows a bounded portion and labels any truncation. A preview limit must not make a large captured body look empty or make an incomplete display look complete.

Text adds one more constraint: a UTF-8 character can occupy several bytes. A byte limit may land in the middle of a character. A slice is a borrowed view of part of the existing text. Before taking that slice, the preview moves the boundary back to a complete character. The [screen chapter](11-drawing-and-navigation.md) then explains how that text occupies terminal columns.

The particular limits are engineering choices recorded in the contract. Understanding why limits exist is more durable than memorising their current values.

## Try following a failure

Imagine three requests. The first is valid. The second has an invalid required wait duration. The third is valid. Explain what the application should display and what happens to the first request's temporary Rust value.

Now change the second request so only its base64 body is malformed. Explain why the outcome differs. Identify which information remains useful and which label the body needs.

## Follow the implementation when ready

The [importer](../../crates/netscope-import/src/lib.rs) implements these field checks, and the [session model](../../crates/netscope-core/src/session.rs) holds the accepted data. The [importer tests](../../crates/netscope-import/tests/har.rs) demonstrate both rejected captures and recoverable body warnings. The [browser compatibility reference](../HAR_COMPATIBILITY.md) records which native exports we have tested. The [verification record](../VERIFICATION.md) records the checks performed.

For parser details, consult [Chrono's timestamp API](https://docs.rs/chrono/latest/chrono/struct.DateTime.html), the [URL parser](https://docs.rs/url/latest/url/struct.Url.html), and the [base64 library](https://docs.rs/base64/latest/base64/). These are references for implementation work, not prerequisites for the explanation above.
