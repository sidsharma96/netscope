# Why was the same request sent again?

A page asks for a list of books. The first response reports an error. A moment later, another request asks for the same list and receives a response. Perhaps the application tried again because the first attempt failed.

There are other explanations. You might have pressed refresh. The page might ask for an update at regular intervals, a behaviour called polling. Another tab might have requested the same list. All of these can produce similar entries in a recording.

Before calling the second request a retry, we need to decide what we can compare and what the comparison cannot tell us. This is also a useful way to understand equality in Rust: a program can compare values exactly while the explanation behind those values remains uncertain.

## Start with what repeated

Here is an invented recording. The first request reports status 503. Two later requests have the same method, URL, and captured request fields. The dotted arrows are possible relationships. They were not recorded by the browser.

```text
Illustrative recording · times from the start of this example

#0  GET /books   start 0 ms · end 10 ms · status 503
      │
      ├ · · possible repeat · · → #1 GET /books · start 20 ms
      │
      └ · · possible repeat · · → #2 GET /books · start 30 ms

Known: the recorded requests share the compared fields.
Unknown: whether either request was caused by the earlier error.
```

It would be misleading to call #1 “attempt two” and #2 “attempt three.” That numbering assumes they belong to one sequence of attempts. A capture may contain both a retry and an independent request, and these fields do not identify which is which.

The same caution applies to a later response. A candidate that receives status 200 has a recorded response outcome, but that alone does not establish that it repaired the earlier failure or completed the user's task. Netscope shows possible repeats without giving them a success label.

## An address is only part of a request

Two requests can ask for the same URL while carrying different information. One might include a cookie identifying a signed-in session; another might have no such cookie. A header might ask for a different representation. A body might contain different submitted data. Looking only at the address would hide these differences.

Our first retry analysis therefore uses a narrow comparison. It considers GET and HEAD, requires the exporter to report a request body size of zero with no captured body representation, and compares the recorded request headers, query fields, and cookies exactly. It also compares the method and the normalized URL. The [redirect lesson](13-following-redirects.md) introduces why an address may need normalization before comparison.

“Exactly” has a consequence here. Reordering the captured headers excludes a candidate, even if that order would make no practical difference to the server. A changing request identifier could also exclude a real retry. This policy favours a clear, reproducible comparison over an attempt to guess which differences the application considers meaningful.

Missing body information is a separate problem. No captured body text does not prove that the request had no body. Requiring a reported zero size adds evidence, but it still relies on the exporter. Similarly, two empty header lists can compare equal even if both exports omitted information. Equality applies to the values we have.

## Turn a comparison into a Boolean value

Rust represents a yes-or-no answer with `bool`. Its two values are `true` and `false`. The equality operator, `==`, compares two values and produces a Boolean. It is different from the single `=` used to assign a value to a variable.

The example below compares just methods and ordered header lists. It deliberately leaves out the other application rules so we can follow each result. A header is represented here as a pair of text values, its name and value. Rust calls a fixed group written in parentheses a tuple. Unlike a vector, a tuple has a fixed number of positions, and those positions can have different types; these two both happen to contain borrowed text.

```rust
let earlier_method = "GET";
let later_method = "GET";
let earlier_headers = vec![("Accept", "application/json"), ("X-View", "books")];
let later_headers = vec![("Accept", "application/json"), ("X-View", "books")];

let same_method = earlier_method == later_method;
let same_headers = earlier_headers == later_headers;
let same_recorded_fields = same_method && same_headers;
assert_eq!(same_recorded_fields, true);

let reordered_headers = vec![("X-View", "books"), ("Accept", "application/json")];
assert_eq!(earlier_headers == reordered_headers, false);
assert_eq!(earlier_method == "HEAD", false);

// Comparison leaves both vectors available to their owners.
assert_eq!(earlier_headers.len(), 2);
assert_eq!(later_headers.len(), 2);
```

Vector equality first requires equal lengths and then equal elements in corresponding positions. Each tuple compares its corresponding fields. The first pair of vectors has matching values at every position, so the result is `true`. In the reordered vector, the first element is different. Having the same two headers somewhere in the list is not enough for this comparison.

The `&&` operator means “and”: both conditions must be true. Rust evaluates the expression on its left first. If that expression is false, the whole result must be false, so Rust skips evaluating the expression on the right. This behaviour is called short-circuit evaluation. In our example, the two comparisons have already been stored in variables; combining them merely reads those Boolean values.

The comparisons do not transfer ownership of the vectors. Rust's equality operation borrows the values for comparison, which is why both vectors remain usable afterward. The final checks read their lengths. A comparison can inspect owned data without consuming it.

Notice what the name `same_recorded_fields` says. It describes the result of these comparisons. Naming it `was_a_retry` would make a stronger claim without adding any evidence. Good names help the next reader understand the limit of a value.

## Decide when to look for a repeat

A repeated successful request is common during ordinary browsing. Netscope begins this search only after selected recorded error statuses: 408, 429, 500, 502, 503, and 504. This is the tool's current scope, not a universal rule about which operations should be retried. Status zero means that no status was recorded; it does not establish a network failure.

Candidates must begin after the source's start and within the interval from its reported end through 30 seconds later. This bound can exclude real repeats. It makes the search finite and predictable, but does not reveal the application's waiting policy. As with redirects, no matches means only that nothing met the stated rules.

A response may provide Retry-After to indicate a waiting period; 429 is one example where it can appear. Netscope retains that header for inspection but does not interpret it in this analysis. A candidate inside our window is therefore not a claim that the client followed the server's waiting instruction. [The definition of 429](https://www.rfc-editor.org/rfc/rfc6585.html#section-4) explains the header's role in that response.

Recognising a possible past retry is also different from deciding whether to send a request again. HTTP's term idempotent concerns whether repeating an identical request has the same intended server effect as sending it once; it does not promise identical responses. The [HTTP method rules](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods) matter when designing a client that actually repeats operations. This inspector only reads a recording. The analysis is also narrower than idempotence. RFC 9110 makes GET, HEAD, OPTIONS, and TRACE safe, and PUT and DELETE idempotent as well. Netscope considers only GET and HEAD for a different reason: they normally carry no body, so a request recorded with body size zero and no captured body gives complete evidence for an exact comparison. A PUT carries a body, and the HAR may not retain it.

## Predict what changes the result

Open `tests/fixtures/har/retries.har` using the [walkthrough's command pattern](06-first-milestone.md). Request #0 has two candidates. Request #3 has one, even though its response header asks for a longer wait than our search window. Request #6 has no recorded status. Scroll to RETRY EVIDENCE to compare the explanations.

In a copy of the file, change one candidate's request header value. Predict whether it still matches. Then restore the value and move its timestamp beyond the window. The visible result may be the same, but the reason is different: one experiment changes the compared request fields, while the other changes when it happened.

Finally, change the Rust example so the later method is HEAD. Which Boolean becomes false? Why do the vectors remain available? Answer those questions before looking at the analysis code [`retries::analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L27). The [contract](../contracts/m2-retry-suggestions.md) states the full policy, and the tests [`time_window_retains_ties_without_inventing_direction`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/retries.rs#L131) check its boundaries.
