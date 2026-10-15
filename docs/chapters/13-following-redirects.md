# Did the browser follow the redirect?

You ask for `/old-books`. The server's response says to try `/books` instead. A little later, the capture contains a request for `/books`. It is tempting to draw an arrow between them and move on.

Now imagine that two browser tabs were open. One tab followed the instruction; the other asked for the same page independently. Both requests could have the same address and almost the same start time. The recording gives us useful evidence, but the address alone cannot tell us which tab acted on the response.

A good inspector should help you examine that uncertainty. It should also preserve it when the evidence cannot settle the question.

## Read the instruction before finding a follow-up

A response header is a named piece of information sent with the response. For a [redirect](../GLOSSARY.md#redirect), the [Location](../GLOSSARY.md#location) header gives an address to consider next. It can contain a full URL, such as `https://example.test/books`, or a relative reference, such as `/books`. A relative reference needs the original URL to determine the complete address. Resolving `/books` against `https://example.test/old-books` gives `https://example.test/books`. [Location in HTTP](https://www.rfc-editor.org/rfc/rfc9110.html#name-location) defines this relationship.

The method also matters. A 307 or 308 redirect preserves it. A 301 or 302 allows POST to become GET. A 303 permits a retrieval with GET or HEAD. Netscope considers those five statuses when suggesting follow-ups. [HTTP redirection semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-redirection-3xx) describes the status rules.

Those fields tell us what the response instructed the client to do. Finding a later compatible request is a separate step. The later request could be absent from the export, or the client could have chosen not to follow the instruction.

## Two plausible requests are still two requests

Consider this small invented recording. Each number identifies one entry, and the times are measured from the beginning of this example. The arrows below show possible explanations, not recorded links.

*Figure 10.1. One instruction, two possible follow-ups (illustrative).*

```text
#0  GET /old-books    start 0 ms · end 10 ms
    302 · Location: /books
             │
             ├ · · possible · · → #1 GET /books · start 20 ms
             │
             └ · · possible · · → #2 GET /books · start 30 ms

Recorded: status, Location, URLs, methods, times
Suggested: either later request could be a follow-up
```

Both later requests satisfy the same basic observations: they ask for the destination using GET, and they start after the redirect response's reported end. Choosing #1 because it appears first would make the display simpler, but it would add a claim that the recording has not established.

Even if #2 were removed, the remaining match would still be a suggestion. Another action could have produced it. A single plausible explanation is useful evidence; it is not automatically a proven cause.

This distinction changes how we read a screen. “One [candidate](../GLOSSARY.md#candidate)” invites inspection. “Ambiguous” tells us to keep more than one possibility open. Neither label should become “the browser followed this request.”

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

Open the supplied `tests/fixtures/har/redirects.har` capture using the command pattern in the [walkthrough](06-first-milestone.md). Open request #0, choose Evidence with 5, and scroll to REDIRECT EVIDENCE. You should find two candidates. Return to the list and inspect #3: this time there is one candidate, on another host. Request #5 has no recorded Location header.

Before editing a copy of the capture, predict what would happen if you removed one of #0's candidates, moved both before its reported end, or removed its Location header. Those changes should produce three different explanations. In none of them does the tool learn what the browser intended.

The [redirect contract](../contracts/m2-redirect-suggestions.md) defines exact comparisons and limits. When you are ready to connect the explanation to the program, read the analysis [`redirects::analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L34) and its fixed-input tests [`window_excludes_overlap_and_includes_both_end_boundaries`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/redirects.rs#L63). Start by finding the rule that stops equal timestamps from implying an order. Source position can break a display tie; it cannot supply missing evidence about causation.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### What `match` guarantees

**Level:** Newcomer


Why does Netscope use `match` to choose a label for a timing, and what happens if someone adds a new `Timing` variant later?

<details>
<summary>Answer</summary>

`match` compares a value against patterns and runs the arm that fits. Rust requires the arms to cover every variant. If a new variant is added, every `match` that lists the existing variants stops compiling until someone decides how to show the new case. The display cannot quietly fall back to a misleading label. The same idea keeps "search unavailable" and "search found nothing" apart in the redirect analysis.

**Learn more:** [Handle the alternatives with match](../chapters/09-options-results-and-matching.md#handle-the-alternatives-with-match) · [Give different outcomes different Rust values](../chapters/13-following-redirects.md#give-different-outcomes-different-rust-values)

</details>

### Does Netscope say which request followed a redirect?

**Level:** Practitioner


Netscope shows redirect and retry relationships between requests in a HAR. Does it claim that request B is the redirect follow-up or retry of request A? What does it report instead, and why?

<details>
<summary>Answer</summary>

It never claims a cause. `RedirectEvidence` and `RetryEvidence` each have three variants. `NotApplicable` means the source status is outside the analysis. `Unavailable(reason)` means the search could not run, for example because there is no usable `Location` header. `Candidates` lists up to 20 request IDs, with a `more` flag when further matches exist.

A HAR does not record which request caused another. Two identical GETs after a 503 could be a user reload, another tab or a client retry. Netscope therefore reports requests that are consistent with the relationship and leaves the choice to you. Keeping three variants also separates "the search could not run" from "the search ran and found nothing", which an empty list alone could not express.

**Learn more:** [Two plausible requests are still two requests](../chapters/13-following-redirects.md#two-plausible-requests-are-still-two-requests) · [Give different outcomes different Rust values](../chapters/13-following-redirects.md#give-different-outcomes-different-rust-values) · [`RedirectEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L7-L21) · [`RetryEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L7-L19)

</details>

### The worst case for redirect and retry matching

**Level:** Expert


What is the worst-case time complexity of retry and redirect analysis for a capture with n entries? Which input triggers it, and what bounds it in practice?

<details>
<summary>Answer</summary>

Building each index is O(n log n). For each source, `partition_point` finds the first possible candidate in O(log k), and a forward scan follows. The scan stops at the 30-second window or after 20 accepted candidates.

For retries, a candidate with the same URL and method but a different header, cookie, or query list is skipped with `continue` and does not count toward the cap of 20. Take n GET requests to one URL, all inside 30 seconds, all with a retryable status such as 503, each with a changing request-ID header. Every source scans every later request and accepts none, so the work is O(n²) list comparisons. Redirect matching has the same shape, but only a method mismatch is skipped without counting, so it reaches O(n²) only when many same-URL requests use a method the status does not allow.

Both analyses run once, eagerly, inside `Session::new`. The import limit of 10,000 entries bounds the adversarial case at roughly 5 × 10^7 comparisons.

**Learn more:** [Why is binary search valid in redirect and retry matching?](../chapters/34-defending-the-design.md#why-is-binary-search-valid-in-redirect-and-retry-matching) · [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [`partition_point`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L68) · [`headers`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L80-L85) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L235-L236)

</details>

### Redirect candidates

**Level:** Newcomer


Request #0 got a 302 to `/books`, and requests #1 and #2 both ask for `/books` shortly afterwards. Does Netscope say which one the browser followed?

<details>
<summary>Answer</summary>

No. A HAR does not record which request caused another. Netscope lists both as possible follow-ups because both match the stated rules: the resolved Location target, a compatible method, and a start after #0's reported end and within 30 seconds of it. Even a single candidate is a suggestion, not proof. "No candidates" means only that nothing met the rules.

**Common wrong answer.** "The first matching request is the follow-up." Choosing it adds a claim the recording does not support.

**Learn more:** [Two plausible requests are still two requests](../chapters/13-following-redirects.md#two-plausible-requests-are-still-two-requests) · [A search needs a stated boundary](../chapters/13-following-redirects.md#a-search-needs-a-stated-boundary) · [`RedirectEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L7)

</details>

### Which redirect candidates are accepted?

**Level:** Practitioner


How is the redirect target derived, and which candidate methods are accepted for 301 and 302, for 303, and for 307 and 308? What makes the analysis return "Unavailable" instead of a list of candidates?

<details>
<summary>Answer</summary>

Only statuses 301, 302, 303, 307 and 308 are considered. The response must have exactly one `Location` header. With none, the result is "No Location header recorded; redirectURL metadata is not used", because HAR's `redirectURL` field is deliberately ignored. With more than one, no target is chosen. The value is trimmed of spaces and tabs and rejected if empty or if it contains whitespace, controls or backslashes. It is resolved against the source URL with `Url::join` and must give an HTTP or HTTPS URL; the fragment is removed.

For 303, a candidate must be GET or HEAD. For 301 and 302, it must use the same method, or GET when the source was POST, matching historical browser behaviour. For 307 and 308 the method must be the same. Candidates must have the same normalised URL, start after the source ends and within 30 seconds, and at most 20 are kept. Headers are not compared. A candidate URL with odd characters is simply left out of the index; only a bad source URL or target makes the result "Unavailable".

**Learn more:** [Read the instruction before finding a follow-up](../chapters/13-following-redirects.md#read-the-instruction-before-finding-a-follow-up) · [Evidence and matching](../contracts/m2-redirect-suggestions.md#evidence-and-matching) · [`method_matches`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L23-L32) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/redirects.rs#L34-L118)

</details>
