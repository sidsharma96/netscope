# Browser HAR compatibility

Verified locally on macOS on October 11, 2026, using native browser exporters and controlled loopback traffic. These results establish support for the recorded cases, not every HAR a browser can produce.

| Browser | Export | Result |
| --- | --- | --- |
| Chrome 154.0.8037.98 | Native DevTools writer, 9 completed entries | Imports; text and form fields retained together; null cookie expiry accepted |
| Firefox 146.0.1 | Native Network Monitor exporter, 9 completed entries | Imports; JSON text with an empty parameter list and form text with populated fields retained |
| Firefox 146.0.1 | Native exporter during collection, 3 entries | Imports; status 0, empty protocol labels, and omitted response MIME type remain unknown |
| Chrome 154.0.8037.98 | HTTPS/H2 native writer, 6 entries | Decoded gzip/Brotli text and base64 binary; lowercase http/2.0 label |
| Firefox 146.0.1 | HTTPS/H2 native exporter, 6 entries | Decoded gzip/Brotli text and base64 binary; HTTP/2 label |
| Safari / Edge | Not recorded in this verification | Untested |

The completed exports cover a page load, favicon request, UTF-8 text, JSON POST, URL-encoded form POST with repeated names and an empty value, binary response, redirect followed by the browser, and a 404 response. The synthetic cookie is `netscope_fixture=synthetic`. Every captured destination is loopback. The original server uses plain HTTP. The additional HTTPS files cover HTTP/2 and gzip/Brotli responses. HTTP/3, caching, multipart uploads, large captures, and authenticated sessions remain untested. Existing synthetic tests cover additional importer behaviours, but are not evidence of native browser export compatibility for those scenarios.

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

The [fixture provenance](https://github.com/sidsharma96/netscope/blob/main/tests/fixtures/har/browsers/README.md) records the exact capture method and file transformations. Our checked-in files were produced by invoking the browser-bundled export functions programmatically, rather than clicking the save menu. They exercise native serialization; the manual save-menu workflow above was not separately verified.

`cargo test --workspace --locked --offline` tests these fixed files without launching a browser or opening a socket. Assertions cover stable ordering, body representations, binary decoding, missing metadata, reviewed JSON body preservation, form replay rejection, command output, and inspector labels. Separate mutations deliberately introduce wrong types and disagreement; they are not presented as browser-generated observations. The [verification ledger](VERIFICATION.md) records the local check results.

## HTTPS, HTTP/2, and compression observations

The two `*-h2.har` files each contain a page and five sequential fetches. The server logged HTTP/2 with ALPN `h2` for both browsers. ALPN is the protocol selection made during TLS connection setup. Chrome wrote `http/2.0`; Firefox wrote `HTTP/2`. Both exact strings now classify as HTTP/2 while retaining their original spelling.

| Controlled response | Server body bytes before compression | Server encoded body bytes | Exported content |
| --- | --- | --- | --- |
| Plain text | 1,280 | 1,280 | Same UTF-8 text in both browsers |
| Gzip text | 1,280 | 52 | Same decoded text in both browsers |
| Brotli text | 1,280 | 40 | Same decoded text in both browsers |
| Gzip binary | 4 | 24 | Base64 of the four decoded bytes in both browsers |

These numbers describe this fixed payload and server, not typical compression ratios. Content-Length matched the server's encoded body length. Chrome reported HAR response bodySize as -1; Firefox reported 234 for gzip text and 220 for Brotli text, rather than 52 and 40. Netscope retains those exporter values without relabelling them as compressed payload length or using them to calculate savings. The importer already avoided double decompression and required no change for these bodies.

Later Chrome entries record DNS/connect/SSL as -1, which HAR 1.2 defines as "does not apply to the current request"; Firefox records zero for the same fields in its later entries. A derived test additionally omits connect to check the missing state. No duration threshold is asserted. Firefox's page entry has SSL 4 ms and connect 0 ms; the existing consistency warning retains both observations without repair. The server log supplies independent evidence that each browser used one H2 session for the recorded sequence; Netscope does not infer that fact from the HAR timings.

### Reproduce the HTTPS scenarios

This exploratory helper needs Node.js with HTTP/2 support and OpenSSL; neither is needed to run the frozen Rust regression suite. The implementation uses Node's [secure HTTP/2 server API](https://nodejs.org/download/release/v24.12.0/docs/api/http2.html).

```sh
mkdir -p /tmp/netscope-har-tls
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 2 \
  -subj '/CN=localhost' -addext 'subjectAltName=DNS:localhost,IP:127.0.0.1' \
  -keyout /tmp/netscope-har-tls/key.pem -out /tmp/netscope-har-tls/cert.pem
node scripts/har_https_fixture_server.mjs \
  /tmp/netscope-har-tls/key.pem /tmp/netscope-har-tls/cert.pem
```

Use a fresh browser profile, open Network Monitor before navigating to the printed HTTPS URL, and allow that local test certificate in the isolated profile. Wait for the completion title and all six completed entries, then export with content included. The server prints negotiated protocol, a process-local session number, and body byte counts for each request. Stop with Ctrl+C. A browser may make extra discovery requests; retain and describe what it actually exports rather than editing the recording to fit a count.

For our automated recordings, Chrome used a certificate exception restricted to the test certificate's public-key fingerprint in a temporary profile. Firefox used the session's acceptInsecureCerts capability. No system trust store was changed and no key/certificate is checked in. This verifies encrypted H2 recording and import, not ordinary browser certificate trust or Netscope replay against that certificate. A separate Node H2 client trusted the exact generated certificate and verified the compressed payloads against their decoded bytes.

## Native SSE and WebSocket exports — October 13, 2026

Frozen Chrome 155.0.8059.40 and Firefox 146.0.1 exports add three-entry loopback page/SSE/WebSocket scenarios. [Provenance and hashes](https://github.com/sidsharma96/netscope/blob/main/tests/fixtures/har/browsers/README.md) record capture method and exact boundaries. Both preserve three complete SSE events, multiline UTF-8, inherited ID and retry setting, while removing a leading BOM from their retained text. Retained text has 123 bytes versus the server's 126. No event timestamps or browser dispatch history are inferred.

Native WS URLs required widening inspection to ws/wss, while replay remains HTTP(S). Chrome's 101 entry reports bodySize -202 and empty serverIPAddress. Only ws/wss 101 entries permit negative bodySize below -1 as unavailable with exact-value warning and empty address as absent. Firefox's 202-byte reported size and empty text do not establish later frame bytes. The helper sent a frame; neither export retained a usable frame history. Classic upgrade header shape is inspected without acceptance-digest, extension or browser-policy validation.

To record again, run `python3 scripts/stream_fixture_server.py`, use an isolated browser profile, open Network Monitor before visiting the printed URL, and wait for the completion title and three settled entries before exporting with content. Stop the helper with Ctrl+C. Dynamic port/timestamps/durations are observations rather than regression expectations. Frozen fixtures, not new browser processes, drive deterministic regression tests.

## Broad native campaign — October 13, 2026

The [120-scenario campaign](COMPATIBILITY_CAMPAIGN.md) adds 240 native Chrome/Firefox browser runs across REST, GraphQL-shaped HTTP, SSE and WebSocket, including HTTPS/H2 and WSS. Actual payload observations and server request logs remain separate from HAR fields. Frozen captures are rechecked offline in the normal suite.

Two compatibility fixes followed native evidence: finite blocked values strictly between -1 and zero are retained as non-duration exporter observations with warnings; empty ws/wss server addresses are treated as absent for status zero as well as 101. Other timing fields, blocked values below -1 and malformed ordinary addresses remain strict. The raw native values are not repaired.

The report records exported-text/Unicode differences, Firefox dispatch/open-event differences and expected capture budgets. Imported retained text is not proof of original wire bytes or browser dispatch. Production live SSE uses its own observed raw bytes. TLS exceptions were limited to disposable browsers; production-client certificate rejections remained enforced.
