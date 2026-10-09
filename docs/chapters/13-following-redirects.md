# Did the browser follow the redirect?

You ask for `/old-books`. The server's response says to try `/books` instead. A little later, the capture contains a request for `/books`. It is tempting to draw an arrow between them and move on.

Now imagine that two browser tabs were open. One tab followed the instruction; the other asked for the same page independently. Both requests could have the same address and almost the same start time. The recording gives us useful evidence, but the address alone cannot tell us which tab acted on the response.

A good inspector should help you examine that uncertainty. It should also preserve it when the evidence cannot settle the question.

## Read the instruction before finding a follow-up

A response header is a named piece of information sent with the response. For a redirect, the Location header gives an address to consider next. It can contain a full URL, such as `https://example.test/books`, or a relative reference, such as `/books`. A relative reference needs the original URL to determine the complete address. Resolving `/books` against `https://example.test/old-books` gives `https://example.test/books`. [Location in HTTP](https://www.rfc-editor.org/rfc/rfc9110.html#name-location) defines this relationship.

The method also matters. A 307 or 308 redirect preserves it. A 301 or 302 allows POST to become GET. A 303 permits a retrieval with GET or HEAD. Netscope considers those five statuses when suggesting follow-ups. [HTTP redirection semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-redirection-3xx) describes the status rules.

Those fields tell us what the response instructed the client to do. Finding a later compatible request is a separate step. The later request could be absent from the export, or the client could have chosen not to follow the instruction.

## Two plausible requests are still two requests

Consider this small invented recording. Each number identifies one entry, and the times are measured from the beginning of this example. The arrows below show possible explanations, not recorded links.

<!-- redirect-evidence-demo -->

Both later requests satisfy the same basic observations: they ask for the destination using GET, and they start after the redirect response's reported end. Choosing #1 because it appears first would make the display simpler, but it would add a claim that the recording has not established.

Even if #2 were removed, the remaining match would still be a suggestion. Another action could have produced it. A single plausible explanation is useful evidence; it is not automatically a proven cause.

This distinction changes how we read a screen. “One candidate” invites inspection. “Ambiguous” tells us to keep more than one possibility open. Neither label should become “the browser followed this request.”

## A search needs a stated boundary

Suppose the same address appears again an hour later. Should it count? Without a boundary, a long capture could connect unrelated activity simply because a URL was reused.

Netscope looks for compatible requests starting no more than 30 seconds after the source's reported end. It also requires a strictly later start than the source and excludes requests beginning before that reported end. These are rules for this tool, not promises made by HTTP. They make the search reproducible: the same recording and rules produce the same candidates without consulting the current time.

There is a cost to this choice. A client might act on response headers before the full response finishes, or follow a redirect after a long pause. Those cases can fall outside our window. “No candidate” therefore tells you about the search result, not whether the browser ever followed the instruction. An inaccurate recorded duration can affect the result too.

An absent Location creates a different problem: there is no target for this search. Several Location headers create uncertainty about the instruction itself. Netscope explains those cases instead of choosing one header or filling in a target from exporter metadata.

## Give different outcomes different Rust values

We now have two kinds of absence. Sometimes we cannot search. Sometimes we search and find no candidates. If both become an empty list, the program loses the distinction before it even draws the screen.

Rust lets us describe alternatives with an enum, a type whose value takes one of several named forms. A form can carry its own data. The small teaching example below has an unavailable search and a completed search carrying a vector of request numbers. The production analysis also records the resolved target and explanations, but these two forms are enough to study the distinction.

```rust
enum Search {
    Unavailable,
    Completed(Vec<usize>),
}

fn candidate_count(search: &Search) -> Option<usize> {
    match search {
        Search::Unavailable => None,
        Search::Completed(ids) => Some(ids.len()),
    }
}

let unavailable = Search::Unavailable;
let no_matches = Search::Completed(vec![]);
let ambiguous = Search::Completed(vec![1, 2]);

assert_eq!(candidate_count(&unavailable), None);
assert_eq!(candidate_count(&no_matches), Some(0));
assert_eq!(candidate_count(&ambiguous), Some(2));
```

`Completed(Vec<usize>)` means that this form contains a vector of unsigned whole numbers. Here they stand for request IDs. The parentheses in `Search::Completed(vec![1, 2])` put a particular vector into that form. The double colon selects a name belonging to the enum, just as it did in the earlier missing-value examples.

The function receives `&Search`, a shared reference. It can inspect the search while the caller keeps ownership. `match` examines which form is present and runs the corresponding arm, the expression after `=>`. In the completed arm, `ids` borrows the vector inside the search; Rust does not move that vector out through the shared reference. Calling `.len()` reads its number of elements.

The returned `Option<usize>` carries the distinction further. `None` says there is no count because the search was unavailable. `Some(0)` is a count from a completed search. The `assert_eq!` checks compare each actual result with the expected value; a mismatch fails the example when it runs. Predict those three results before looking back at the function.

This is why the choice of a type matters to the explanation on screen. Keeping the alternatives distinct gives the display enough information to tell the truth. It does not, by itself, prove that our matching rules are good; those need their own review and tests.

## Try changing the evidence

Open the supplied `tests/fixtures/har/redirects.har` capture using the command pattern in the [walkthrough](06-first-milestone.md). Open request #0 and scroll to REDIRECT EVIDENCE. You should find two candidates. Return to the list and inspect #3: this time there is one candidate, on another host. Request #5 has no recorded Location header.

Before editing a copy of the capture, predict what would happen if you removed one of #0's candidates, moved both before its reported end, or removed its Location header. Those changes should produce three different explanations. In none of them does the tool learn what the browser intended.

The [redirect contract](../contracts/m2-redirect-suggestions.md) defines exact comparisons and limits. When you are ready to connect the explanation to the program, read the [analysis](../../crates/netscope-core/src/redirects.rs) and its [fixed-input tests](../../crates/netscope-import/tests/redirects.rs). Start by finding the rule that stops equal timestamps from implying an order. Source position can break a display tie; it cannot supply missing evidence about causation.
