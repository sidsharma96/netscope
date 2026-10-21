# How can we change a request without changing the recording?

You find a GET request for a list of books and want to try it against a server on your own machine. The recorded address is useful as a starting point, but it is also evidence of what happened earlier. Editing that evidence would make the capture harder to trust.

We need two things to exist at once: the original request and a separate [draft](../GLOSSARY.md#draft) we can change. Before sending anything, we also need a way to examine what those changes mean. A mistyped address should remain editable, and a [header](../GLOSSARY.md#header) chosen for one destination should not quietly travel to another.

Opening a [review](../GLOSSARY.md#review) prepares a possible action. It does not perform that action. Only an explicit Send starts the new request. This chapter follows the data up to that boundary; the next chapter explains what happens while we wait for an answer.

## Understand the field before changing it

Suppose you want to send a title to a local book service. Four fields describe the request. Method names the operation: GET asks for a resource, while POST sends data for the server to process. URL identifies the destination. Headers provide additional information, such as a preferred response format. Body contains the data you want to send.

Select a field to see its value and explanation. Editing changes the draft. Review shows the destination and information the new request would carry; sending is a separate action. The [usage guide](../UI_LAYOUT.md#edit-and-review) supplies the controls so we can concentrate here on what those actions mean.

## Give the draft its own text

A draft begins as a copy of selected information. It does not need another copy of the entire capture, its response bodies, or every timing. It needs its own URL text, header choices, and the identity of the request it came from.

We previously saw that assigning a String to a new variable moves ownership. A move would leave the original variable unable to use that String. Here, both the capture and the draft must remain usable independently, so the draft needs a copy of the text. Rust's `clone()` method makes that copy for a String.

The diagram shows ownership, not a precise memory layout. Each arrow points to text owned by that value. The two buffers initially contain equal text, but changing the second one does not change the first.

```text
Illustrative ownership before editing

Captured URL ──owns──→ "https://example.test/books"
Draft URL    ──owns──→ "https://example.test/books"

After editing only the draft

Captured URL ──owns──→ "https://example.test/books"
Draft URL    ──owns──→ "http://localhost:8080/books"
```

Here is the same distinction in a small Rust example:

```rust
let captured_url = String::from("https://example.test/books");
let mut draft_url = captured_url.clone();

draft_url.clear();
draft_url.push_str("http://localhost:8080/books");

assert_eq!(captured_url, "https://example.test/books");
assert_eq!(draft_url, "http://localhost:8080/books");
```

`mut` allows the draft variable to be changed. `clear()` removes the string's contents, while `push_str()` appends borrowed text to it. These methods operate on the draft's buffer. Neither requires changing the captured string.

Cloning has a cost: it creates independently owned storage and copies the text bytes. That cost is justified here by the behaviour we want. Cloning every response in the capture would spend memory without helping us edit this URL. Choosing what to copy is part of designing the feature, not just satisfying the compiler.

## Invalid text still belongs in an editor

While replacing an address, you might temporarily have just `http:` in the field. Rejecting every keystroke until the entire address is valid would make ordinary editing difficult. Instead, the editor keeps the draft text and explains why it cannot yet be reviewed.

There are two different questions: “What has the user typed?” and “Is that a supported destination?” Keeping them separate lets you repair the text without losing it. Netscope checks for a complete HTTP or HTTPS URL with a host and rejects embedded credentials, controls, whitespace, and backslashes. It also places a size limit on editable text.

Once the address is valid, the review shows the effective destination produced by parsing it. For example, a default port or a path containing `..` can have a simpler serialized form. A fragment, the part after `#`, is disclosed separately because it is not included in the HTTP request destination. The original text remains in the draft so you can return and edit it.

These are validation rules for this editor. They are not evidence that the server exists or is reachable. No connection is attempted while validating a draft.

## A destination change also changes a decision

Suppose you explicitly select an Authorization header for one destination, then change the URL to a local server. Keeping that header selected would carry a choice into a different context. Netscope therefore clears the request body and selected or manually added headers whenever the saved URL text changes, and immediately explains the reset.

Every draft starts with no captured headers selected. Repeated captured headers remain separate choices. Fields managed by the HTTP client, such as Host, Content-Type, and Content-Length, cannot be copied as ordinary user headers. A blocked entry includes its reason, so a disabled choice does not look like a broken key.

All header values are hidden in the draft list and review until you explicitly reveal them. Masking every value avoids guessing which custom header might contain private information. Names remain visible so you can navigate. When you enter a new value, the editing field shows what you type; leaving that field restores masking.

These choices describe the request we intend to prepare. They do not recreate a browser's cookie storage or login state, and they do not make the draft identical to the original request.

## Review the current draft, not an earlier one

A review is a freshly validated view of the current draft. Summary answers the first question: where will this request go, and how much information will it carry? It shows the parsed destination, chosen and automatic header counts, body size, and response limits. Left and Right switch to Headers and Body without sending anything.

Headers separates fields you chose from fields the client supplies. For example, Host identifies the destination, and Content-Length states the number of body bytes. Captured fields left out of the new request appear separately. Header values remain masked until you press v. Body shows the outgoing text, with control characters displayed as escapes so a newline cannot disguise the structure of the screen.

The size is measured in UTF-8 bytes, the units sent over the connection, rather than letters. For example, `café` contains four letters but takes five bytes because `é` uses two. The Body view and the automatically supplied Content-Length therefore describe the same bytes. Choosing JSON changes the format and requires valid JSON; it does not translate ordinary prose into JSON.

Read this diagram from left to right. Each arrow changes what you inspect; only the last arrow starts a connection.

```text
Draft fields → Review: Summary ↔ Headers ↔ Body → explicit Send
     ↑                   │
     └──── Esc to edit ──┘
```

Review describes the intended request. Header spelling and order can differ when the client writes it to the connection; this is not a recorded packet transcript.

Returning to edit leaves the review. You must review again after changing the draft; there is no old approval carried into a new destination. Only Enter or s from a valid review starts sending. Opening the review itself does no network work.

After sending, the result belongs to the accepted request. If you return to editing and change the draft, an older retained result must still name its own destination. Otherwise the screen could suggest that an answer came from a server which has not been contacted. This is the same ownership distinction as the two independent URL strings: equal-looking values can have different responsibilities and different lifetimes.

This separation matters during sending. A draft can change while someone is preparing it. An accepted operation needs a definite set of values to work with. The next chapter follows that operation through waiting and cancellation.

## Try the boundary yourself

Open the sample from the [walkthrough](06-first-milestone.md), inspect it, and press r. Choose a header, then change the URL. The header selection should clear because the destination changed. Review the valid draft without sending, then return to the inspector. Its captured URL should still be the original one.

The draft model [`ReplayDraft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L65) owns the editable data, and [`ReplayEditor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L31) handles the interaction. The [next chapter](17-waiting-for-a-response.md) follows an accepted send while the server is still working.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Draft, review and send

You press `r` on a captured request. What do you get, and what has to happen before anything goes over the network?

<details>
<summary>Answer</summary>

You get a draft: a separate, editable copy of the URL with no captured headers selected. Editing it never changes the recording. Opening a review validates the current draft and shows the effective destination, headers, timeout and size limit. Review does no network work. Only Enter or `s` from a valid review sends the request. Changing the URL clears selected headers and the body, so a choice made for one destination does not travel to another.

**Learn more:** [A destination change also changes a decision](../chapters/16-preparing-a-request.md#a-destination-change-also-changes-a-decision) · [Review the current draft, not an earlier one](../chapters/16-preparing-a-request.md#review-the-current-draft-not-an-earlier-one) · [`ReplayDraft`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/replay.rs#L65)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="borrowing-versus-cloning"></a>

[Borrowing versus cloning](../questions/newcomer.md#borrowing-versus-cloning)

<a id="what-stops-a-capture-from-sending-traffic"></a>

[What stops a capture from sending traffic?](../questions/practitioner.md#what-stops-a-capture-from-sending-traffic)

<a id="what-happens-between-a-draft-and-the-wire"></a>

[What happens between a draft and the wire?](../questions/practitioner.md#what-happens-between-a-draft-and-the-wire)

<a id="what-does-opening-a-saved-draft-check"></a>

[What does opening a saved draft check?](../questions/practitioner.md#what-does-opening-a-saved-draft-check)

</details>
