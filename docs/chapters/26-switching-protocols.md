# What happens after a connection changes its rules?

A page can receive book updates through SSE, but suppose it also needs to send an immediate message back through the same connection. WebSocket provides a conversation in both directions. Opening that conversation begins with a negotiation. A recording of the negotiation is useful, even when the later conversation is absent.

Imagine finding a request with status `101` rather than `200`. In a classic HTTP/1.1 WebSocket exchange, 101 means **Switching Protocols**: the server agrees to change the rules used on this connection. After the HTTP headers, the connection carries WebSocket frames, structured pieces of data with their own boundaries. Those frames are not an ordinary HTTP response body.

## An agreement has two sides

The client asks to upgrade using `Upgrade: websocket` and `Connection: Upgrade`. The first field names the proposed protocol. The second marks the upgrade instruction as applying to this connection. A server accepting the classic upgrade responds with the corresponding fields and status 101. Header names and these comma-separated tokens are compared without regard to letter case.

The following illustration shows the change in rules. It is not a timeline extracted from a HAR. The arrows show the direction of an exchange; the horizontal boundary marks the point after which reading another HTTP response would be the wrong interpretation.

```text
client                                  server
   |---- HTTP GET + upgrade request ------>|
   |<--- HTTP 101 + upgrade response ------|
   |                                       |
   |     connection now uses WebSocket     |
   |---- WebSocket data frame ------------>|
   |<--- WebSocket data frame --------------|
```

A client also supplies `Sec-WebSocket-Key`, the base64 encoding of a random 16-byte value, and version `13`. The server computes `Sec-WebSocket-Accept` as base64(SHA-1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")), where the long string is a fixed value from the specification. Only a server that understands WebSocket would compute that answer, so the check stops a client from treating an ordinary HTTP response, or a cached one, as an accepted upgrade. Netscope checks only that the accept field has the right shape; it does not compute the expected value. Seeing an accept field is weaker evidence than checking its calculated value. Neither check proves that a later application message arrived. The [WebSocket specification](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2) defines the acceptance checks.

Netscope examines the recorded header shape. It reports missing or inconsistent fields, but does not calculate the accept value, validate extensions, or reproduce a browser's acceptance policy. Read “expected classic upgrade header shape” with that limit in mind. A 403 response is an observed refusal, even if some upgrade headers remain in the export. Status zero means no HTTP status was recorded.

## Choosing a language for the conversation

WebSocket transports messages; it does not determine what their contents mean. Two applications may agree on a **subprotocol**, a named set of rules layered on top. A client might offer `books-v1, books-v2` and the server select `books-v1` in `Sec-WebSocket-Protocol`. The server's selected name must be one of those offered. These names are case-sensitive: `Books-v1` is a different name.

This gives us a small prediction. If the client offers only `books-v2` and the response selects `books-v1`, the recorded negotiation is inconsistent. If the response selects nothing, there may be no subprotocol agreement; its absence alone is not an error. Netscope checks the recorded selection without pretending to understand the application's message format.

## The exporter can stop at the agreement

Our local browser experiment sent a synthetic message after the upgrade. Both native HAR exports retained the 101 exchange, but neither supplied a usable frame history. Chrome retained no response text. Firefox retained an empty text value. An empty exported text value does not establish that the socket conversation was empty.

Even the reported body sizes differ. One exporter produced a negative size, which Netscope labels unavailable and preserves in a warning. A positive size from the other exporter is still not a count of WebSocket messages. These are observations from particular exports, not a universal description of browser behaviour. The [compatibility reference](../HAR_COMPATIBILITY.md) records their provenance and exact values.

A `ws://` URL identifies WebSocket; `wss://` adds TLS, the encryption and identity verification used by HTTPS. Netscope imports both for inspection. Its request sender supports HTTP and HTTPS GET/POST rather than a WebSocket conversation, so these captures cannot become replay drafts. Keeping that distinction prevents a “send again” action from silently changing the meaning of the operation.

## Different protocols need different questions

The classic 101 exchange is an HTTP/1.1 mechanism. HTTP/2 uses a [different negotiation](https://www.rfc-editor.org/rfc/rfc8441) for a conversation within one identified exchange; absence of HTTP/1.1 upgrade fields cannot be used to reject that arrangement. Netscope therefore labels candidates outside its classic inspection scope instead of applying the same checklist to every version.

Separating a candidate, its recorded status, and its checked evidence also helps ordinary Rust code stay understandable. An optional result says whether this inspection applies at all. Inside a present result, a summary and a list of notes can express uncertainty without forcing everything into a success/failure boolean. There is no need to invent a frame list just to fill a screen.

Try explaining a 101 capture with no body text to another reader. What agreement is visible? What messages remain unknown? Then change the selected subprotocol in an illustrative example. You should be able to predict the inconsistency without reading the source. The inspection code [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L28) and browser regression tests [`native_sse_and_websocket_exports_preserve_content_and_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/browser_exports.rs#L228) supply implementation evidence once those questions are clear.
