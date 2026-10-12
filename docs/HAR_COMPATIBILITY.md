# Browser HAR compatibility

Verified locally on macOS on October 11, 2026, using native browser exporters and controlled loopback traffic. These results establish support for the recorded cases, not every HAR a browser can produce.

| Browser | Export | Result |
| --- | --- | --- |
| Chrome 154.0.8037.98 | Native DevTools writer, 9 completed entries | Imports; text and form fields retained together; null cookie expiry accepted |
| Firefox 146.0.1 | Native Network Monitor exporter, 9 completed entries | Imports; JSON text with an empty parameter list and form text with populated fields retained |
| Firefox 146.0.1 | Native exporter during collection, 3 entries | Imports; status 0, empty protocol labels, and omitted response MIME type remain unknown |
| Safari / Edge | Not recorded in this verification | Untested |

The completed exports cover a page load, favicon request, UTF-8 text, JSON POST, URL-encoded form POST with repeated names and an empty value, binary response, redirect followed by the browser, and a 404 response. The synthetic cookie is `netscope_fixture=synthetic`. Every captured destination is loopback. The server uses plain HTTP; these files do not verify HTTPS, HTTP/2 or HTTP/3, caching, compressed responses, multipart uploads, large captures, or authenticated sessions. Existing synthetic tests cover additional importer behaviours, but are not evidence of native browser export compatibility for those scenarios.

## What the importer preserves

A request body may include text, parsed fields, or both. Both representations are validated and retained independently. Fields preserve order, repeated names, and empty values. The importer does not reconstruct text from fields or resolve disagreements. JSON/plain-text replay can use retained text under the existing review rules; form replay remains unsupported.

Chrome's null cookie expiry becomes no recorded expiry, as does an omitted expiry. Other incorrect expiry types still fail. Empty protocol strings are accepted as unrecorded labels; missing or non-string protocol fields still fail. An omitted response MIME type produces a warning and an empty model label; a present non-string MIME type still fails. The inspector makes these gaps visible. The [inspection contract](contracts/m1-har-inspection.md) specifies the complete policy.

Firefox's `HTTP/1` label remains exactly as exported and is not converted to HTTP/1.0 or HTTP/1.1. Status 0 in the early snapshot is not diagnosed as a failed request. The snapshot was taken before Network Monitor had collected the full request list, even though the page had already finished its own sequence.

## Record another controlled export

1. Run `python3 scripts/har_fixture_server.py` from the repository root. It prints an ephemeral loopback URL; stop it with Ctrl+C afterward.
2. Use a fresh browser profile and open its developer tools before navigating to that URL. Select the Network panel, enable recording, and clear earlier entries.
3. Navigate to the printed URL. Wait for the title to become `Netscope capture complete`, then wait for the Network panel to finish receiving entries and response bodies. Our complete files contain nine entries, including the favicon; that incidental browser request may vary across versions.
4. Use the browser's HAR export/save action with response content included. If exporting cookies is optional, the only cookie needed here is the synthetic one from this server. Inspect the file before adding it to the repository.
5. Run `cargo run -p netscope-tui --locked --offline -- /absolute/path/to/export.har`. Inspect the form, JSON body, binary preview, redirect, and 404. Redirecting output exercises summary mode instead of opening the terminal interface.

The helper reproduces the scenarios, not identical HAR bytes. Ports, timestamps, measurements, browser metadata, and collection order can change. Firefox documents its HAR actions in the [Network Monitor toolbar](https://firefox-source-docs.mozilla.org/devtools-user/network_monitor/toolbar/index.html) and [request list](https://firefox-source-docs.mozilla.org/devtools-user/network_monitor/request_list/index.html).

## Frozen regression evidence

The [fixture provenance](../tests/fixtures/har/browsers/README.md) records the exact capture method and file transformations. Our checked-in files were produced by invoking the browser-bundled export functions programmatically, rather than clicking the save menu. They exercise native serialization; the manual save-menu workflow above was not separately verified.

`cargo test --workspace --locked --offline` tests these fixed files without launching a browser or opening a socket. Assertions cover stable ordering, body representations, binary decoding, missing metadata, reviewed JSON body preservation, form replay rejection, command output, and inspector labels. Separate mutations deliberately introduce wrong types and disagreement; they are not presented as browser-generated observations. The [verification ledger](VERIFICATION.md) records the local check results.
