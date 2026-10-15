# Why was the same request sent again?

A page asks for a list of books. The first response reports an error. A moment later, another request asks for the same list and receives a response. Perhaps the application tried again because the first attempt failed.

There are other explanations. You might have pressed refresh. The page might ask for an update at regular intervals, a behaviour called polling. Another tab might have requested the same list. All of these can produce similar entries in a recording.

Before calling the second request a [retry](../GLOSSARY.md#retry), we need to decide what we can compare and what the comparison cannot tell us. This is also a useful way to understand equality in Rust: a program can compare values exactly while the explanation behind those values remains uncertain.

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

Two requests can ask for the same URL while carrying different information. One might include a [cookie](../GLOSSARY.md#cookie) identifying a signed-in session; another might have no such cookie. A header might ask for a different representation. A body might contain different submitted data. Looking only at the address would hide these differences.

Our first retry analysis therefore uses a narrow comparison. It considers GET and HEAD, requires the exporter to report a request body size of zero with no captured body representation, and compares the recorded request headers, [query](../GLOSSARY.md#query-string) fields, and cookies exactly. It also compares the method and the normalized URL. The [redirect lesson](13-following-redirects.md) introduces why an address may need normalization before comparison.

“Exactly” has a consequence here. Reordering the captured headers excludes a candidate, even if that order would make no practical difference to the server. A changing request identifier could also exclude a real retry. This policy favours a clear, reproducible comparison over an attempt to guess which differences the application considers meaningful.

Missing body information is a separate problem. No captured body text does not prove that the request had no body. Requiring a reported zero size adds evidence, but it still relies on the exporter. Similarly, two empty header lists can compare equal even if both exports omitted information. Equality applies to the values we have.

## Turn a comparison into a Boolean value

Rust represents a yes-or-no answer with `bool`. Its two values are `true` and `false`. The equality operator, `==`, compares two values and produces a [Boolean](../GLOSSARY.md#boolean). It is different from the single `=` used to assign a value to a variable.

The example below compares just methods and ordered header lists. It deliberately leaves out the other application rules so we can follow each result. A header is represented here as a pair of text values, its name and value. Rust calls a fixed group written in parentheses a [tuple](../GLOSSARY.md#tuple). Unlike a vector, a tuple has a fixed number of positions, and those positions can have different types; these two both happen to contain borrowed text.

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

Recognising a possible past retry is also different from deciding whether to send a request again. HTTP's term [idempotent](../GLOSSARY.md#idempotent) concerns whether repeating an identical request has the same intended server effect as sending it once; it does not promise identical responses. The [HTTP method rules](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods) matter when designing a client that actually repeats operations. This inspector only reads a recording. The analysis is also narrower than idempotence. RFC 9110 makes GET, HEAD, OPTIONS, and TRACE safe, and PUT and DELETE idempotent as well. Netscope considers only GET and HEAD for a different reason: they normally carry no body, so a request recorded with body size zero and no captured body gives complete evidence for an exact comparison. A PUT carries a body, and the HAR may not retain it.

## Predict what changes the result

Open `tests/fixtures/har/retries.har` using the [walkthrough's command pattern](06-first-milestone.md). Request #0 has two candidates. Request #3 has one, even though its response header asks for a longer wait than our search window. Request #6 has no recorded status. Scroll to RETRY EVIDENCE to compare the explanations.

In a copy of the file, change one candidate's request header value. Predict whether it still matches. Then restore the value and move its timestamp beyond the window. The visible result may be the same, but the reason is different: one experiment changes the compared request fields, while the other changes when it happened.

Finally, change the Rust example so the later method is HEAD. Which Boolean becomes false? Why do the vectors remain available? Answer those questions before looking at the analysis code [`retries::analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L27). The [contract](../contracts/m2-retry-suggestions.md) states the full policy, and the tests [`time_window_retains_ties_without_inventing_direction`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/retries.rs#L131) check its boundaries.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Possible retries

**Level:** Newcomer


After a 503, the same GET appears again. When does Netscope list it as a possible retry, and what does it avoid claiming?

<details>
<summary>Answer</summary>

Netscope searches only after statuses 408, 429, 500, 502, 503 and 504, and only for a GET or HEAD reported with no body. A later request qualifies if it has the same method and URL, exactly the same recorded headers, query fields and cookies, and starts within 30 seconds after the first one ended. It is a candidate, not a confirmed retry. A refresh or another tab can produce the same entry.

**Learn more:** [An address is only part of a request](../chapters/14-recognizing-repeats.md#an-address-is-only-part-of-a-request) · [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [`RetryEvidence`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L7)

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

### How are retry candidates found?

**Level:** Practitioner


Which source requests are eligible for retry analysis, which statuses start the search, what must match for a later request to be a candidate, what time window applies, and how is the search implemented?

<details>
<summary>Answer</summary>

A source with status 0 gets "Unavailable", because a network failure cannot be assumed. Statuses other than 408, 429, 500, 502, 503 and 504 get "NotApplicable". The source must be a GET or HEAD with a reported body size of 0 and no captured body, its duration must be finite and non-negative, and its URL must parse conservatively as HTTP or HTTPS with the fragment removed.

The index is a `BTreeMap` from (URL, method) to request IDs in chronological order, built once at import. For each source, `partition_point` binary-searches that list for the first request that starts strictly after the source starts and at or after the source ends. The scan then moves forward and stops once a request starts more than 30,000 ms after the source's end. A candidate must have exactly the same recorded headers, cookies and query lists, in the same order and case. Up to 20 candidates are kept, and `more` records that another exists.

**Common wrong answer.** "Status 0 triggers a retry search because it means a network error." Netscope refuses to assume that.

**Learn more:** [Decide when to look for a repeat](../chapters/14-recognizing-repeats.md#decide-when-to-look-for-a-repeat) · [Why is binary search valid in redirect and retry matching?](../chapters/34-defending-the-design.md#why-is-binary-search-valid-in-redirect-and-retry-matching) · [Eligibility and comparison](../contracts/m2-retry-suggestions.md#eligibility-and-comparison) · [`analyze`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/retries.rs#L27-L96)

</details>
