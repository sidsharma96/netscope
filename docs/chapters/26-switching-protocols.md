# What happens after a connection changes its rules?

A page can receive book updates through SSE, but suppose it also needs to send an immediate message back through the same connection. [WebSocket](../GLOSSARY.md#websocket-upgrade) provides a conversation in both directions. Opening that conversation begins with a negotiation. A recording of the negotiation is useful, even when the later conversation is absent.

Imagine finding a request with status `101` rather than `200`. In a classic [HTTP/1.1](../GLOSSARY.md#http11) WebSocket exchange, 101 means **Switching Protocols**: the server agrees to change the rules used on this connection. After the HTTP headers, the connection carries WebSocket frames, structured pieces of data with their own boundaries. Those frames are not an ordinary HTTP response body.

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

The client also includes a random value when it asks to switch protocols. The server calculates a reply from that value, and the client checks the reply before continuing. This helps distinguish an answer to this WebSocket request from an unrelated HTTP response. It does not establish that a later application message arrived.

Netscope checks whether the recorded reply field is present, but does not verify its calculation. That limits the conclusion we can draw from its report. The [WebSocket specification](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2) defines the calculation for readers implementing a client.

The inspector reports missing or inconsistent agreement headers. A report that the expected headers are present is a narrower check than a browser accepting the connection. A 403 response is an observed refusal, even if some upgrade headers remain in the export. Status zero means no HTTP status was recorded.

## Choosing a language for the conversation

WebSocket transports messages; it does not determine what their contents mean. Two applications may agree on a **[subprotocol](../GLOSSARY.md#subprotocol)**, a named set of rules layered on top. A client might offer `books-v1, books-v2` and the server select `books-v1` in `Sec-WebSocket-Protocol`. The server's selected name must be one of those offered. These names are case-sensitive: `Books-v1` is a different name.

This gives us a small prediction. If the client offers only `books-v2` and the response selects `books-v1`, the recorded negotiation is inconsistent. If the response selects nothing, there may be no subprotocol agreement; its absence alone is not an error. Netscope checks the recorded selection without pretending to understand the application's message format.

## The exporter can stop at the agreement

Our local browser experiment sent a synthetic message after the upgrade. Both native HAR exports retained the 101 exchange, but neither supplied a usable frame history. Chrome retained no response text. Firefox retained an empty text value. An empty exported text value does not establish that the socket conversation was empty.

Even the reported body sizes differ. One exporter produced a negative size, which Netscope labels unavailable and preserves in a warning. A positive size from the other exporter is still not a count of WebSocket messages. These are observations from particular exports, not a universal description of browser behaviour. The [compatibility reference](../HAR_COMPATIBILITY.md) records their provenance and exact values.

A `ws://` URL identifies WebSocket; `wss://` adds TLS, the encryption and identity verification used by HTTPS. Netscope imports both for inspection. Its request sender supports HTTP and HTTPS GET/POST rather than a WebSocket conversation, so these captures cannot become replay drafts. Keeping that distinction prevents a “send again” action from silently changing the meaning of the operation.

## Different protocols need different questions

The classic 101 exchange is an HTTP/1.1 mechanism. HTTP/2 uses a [different negotiation](https://www.rfc-editor.org/rfc/rfc8441) for a conversation within one identified exchange; absence of HTTP/1.1 upgrade fields cannot be used to reject that arrangement. Netscope therefore labels candidates outside its classic inspection scope instead of applying the same checklist to every version.

Separating a candidate, its recorded status, and its checked evidence also helps ordinary Rust code stay understandable. An optional result says whether this inspection applies at all. Inside a present result, a summary and a list of notes can express uncertainty without forcing everything into a success/failure boolean. There is no need to invent a frame list just to fill a screen.

Try explaining a 101 capture with no body text to another reader. What agreement is visible? What messages remain unknown? Then change the selected subprotocol in an illustrative example. You should be able to predict the inconsistency without reading the source. The inspection code [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L28) and browser regression tests [`native_sse_and_websocket_exports_preserve_content_and_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/browser_exports.rs#L228) supply implementation evidence once those questions are clear.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### A WebSocket upgrade

A recorded HTTP/1.1 response has status 101 and headers agreeing to switch to WebSocket. What agreement does it record, and what does it leave unknown?

<details>
<summary>Answer</summary>

The response records the server agreeing to switch the connection to WebSocket. Later traffic would use WebSocket messages rather than an ordinary HTTP response body. The captured agreement does not tell us which messages followed. Netscope checks the presence and consistency of selected agreement headers, but does not verify the server’s calculated reply or reconstruct a conversation missing from the file.

**Common wrong answer.** "The agreement proves that the page received a message." A later message needs its own recorded evidence.

**Learn more:** [An agreement has two sides](../chapters/26-switching-protocols.md#an-agreement-has-two-sides) · [The exporter can stop at the agreement](../chapters/26-switching-protocols.md#the-exporter-can-stop-at-the-agreement) · [`websocket_evidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/websocket.rs#L28)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="what-does-netscope-inspect-in-sse-and-websocket-captures"></a>

[What does Netscope inspect in SSE and WebSocket captures?](../questions/practitioner.md#what-does-netscope-inspect-in-sse-and-websocket-captures)

<a id="a-wrong-sec-websocket-accept-value"></a>

[A wrong Sec-WebSocket-Accept value](../questions/expert.md#a-wrong-sec-websocket-accept-value)

</details>
