# Seeing a group without losing its requests

The request list is useful when you know which entry to inspect. With hundreds of entries, you may first want a smaller question: how many requests went to the host that serves our book list?

A host summary answers that question by collecting entries with the same URL host. It gives you a way into the capture while keeping the individual records available. The important choices are what makes two entries belong together and what a number about the group actually means.

## Choose what makes a group

In `https://api.example.test/books`, the host is `api.example.test`. The path identifies `/books` within that address. Several paths can therefore belong to the same host group.

Netscope uses the normalised host produced while parsing the URL. This lets different letter casing in an ordinary hostname belong to the same group. A different subdomain, such as `sub.api.example.test`, remains a separate host. Schemes and ports do not divide this particular grouping: an HTTP URL and an HTTPS URL with the same host can appear together.

That rule defines a useful collection of requests. It does not establish that they reached one machine or used one connection. A summary's title needs to name the rule it uses, which is why the screen says it groups by URL host.

## Count each entry once

Imagine these three requests in source order: one to `api.example.test`, one to `assets.example.test`, and another to `api.example.test`. To count the API requests, start at zero, read each host, and add one when it matches.

A loop expresses this repeated operation in Rust:

```rust
let hosts = ["api.example.test", "assets.example.test", "api.example.test"];
let mut api_requests = 0;
for host in hosts {
    if host == "api.example.test" {
        api_requests += 1;
    }
}
assert_eq!(api_requests, 2);
```

The square brackets create an array, a collection whose length is fixed. All of its elements have the same type; here they are references to quoted text. This short example uses an array because we already know all three names. The imported capture uses a vector so it can hold however many entries the file contains within its limit.

`for host in hosts` gives the name `host` to each element in turn. The code inside the braces runs once for each element. We do not need to maintain a separate numeric position to advance through the array.

The `if` statement runs its body only when the condition is true. `==` compares the names, while the single `=` earlier assigned an initial value. The counter is mutable because each matching entry changes it. After the loop, the assertion checks the result.

To produce a summary for every host, the program needs to find the right counter for each name. A map provides that lookup: a key identifies a stored value. Here, the host name identifies its summary. The map helps find a group; the original vector still retains every request, including repeated ones.

## Keep missing statuses visible

We can repeat the same counting idea for response statuses. A reported 200 contributes to the 2xx count; a reported 503 contributes to the 5xx count. These are counts of values in the recording.

Status zero needs its own count because HAR uses it when no HTTP status was recorded. Putting those entries into a success count or diagnosing them as a particular network failure would add a conclusion the value does not support.

The selected host also shows how many requests have import warnings. One request with several warnings still contributes one to that count. Defining the unit before counting prevents a small ambiguity from becoming a misleading label.

## A sum answers a different question from elapsed time

Suppose two requests to the same host start together. One takes 40 ms and the other takes 60 ms. Adding their reported durations gives 100 ms, but the interval from their common start to the later finish is 60 ms in this invented example.

The horizontal lines below share one time scale. Their overlap is the reason the sum and elapsed interval differ.

<!-- grouped-duration-demo -->

The arithmetic itself is straightforward:

```rust
let reported_durations = [40.0, 60.0];
let mut sum = 0.0;
for duration in reported_durations {
    sum += duration;
}
assert_eq!(sum, 100.0);
```

This loop correctly adds the durations. The meaning comes from the question we asked it to answer. Calling the result elapsed time would be an error in interpretation, even though the addition is correct.

The host view therefore labels the value as a sum of reported durations and notes that requests can overlap. It adds each entry's reported total once. It does not also add SSL timing, which may already be contained in the total.

Numbers also have a finite representable range. If a sum grows beyond that range, the summary displays `Overflow` while retaining the requests. It does not display infinity as a measurement or substitute zero. This is another use for the distinction between an available value and an unavailable result.

## Filter the view while keeping identity

Selecting a host narrows the request list to that group's IDs. Those IDs still identify their original source entries, and their order still follows the recorded start times. Filtering out other rows does not renumber the remaining requests.

Keeping the request records in one place lets the inspector borrow the same data whether you opened it from the complete list or a host's list. Returning to all requests can also restore the position you had before filtering. The filter changes what you are looking at, not the captured values.

Timing bars retain the complete capture's scale when a host filter is applied. A request therefore does not appear longer merely because you hid requests to other hosts.

## Why calculate a group before drawing it?

The captured requests stay unchanged while you browse. We can therefore calculate host counts and ID lists once and reuse them on later screens. This spends memory on the summaries but avoids repeating the same count for every keypress. The lists contain request IDs, so grouping does not make another copy of every response body.

Calculating only when Hosts is first opened would defer that work for readers who never need it. Recounting each time would avoid keeping a summary but repeat the work. If requests arrived continuously, a once-built summary would become stale; we would need to update it or rebuild it. The present choice depends on an offline, unchanging session, not on a rule that caching is always better. A cache is retained information that avoids calculating or fetching the same answer again.

## Try the host view

From the project directory, run:

```sh
cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/multi-host.har
```

Press h to open Hosts. The invented capture contains three host groups. Select `api.example.test` and press Enter to see its three requests. The summary includes one recorded 2xx status, one 5xx status, and one entry with no status recorded.

Press Enter again to inspect a request. Esc returns to its filtered list, and another Esc opens Hosts. Press a from either list to return to all requests at your saved position. Help lists the available keys.

Before running the sample, predict the API group's duration sum from its three recorded totals: 50 ms, 70 ms, and zero. Then explain why the resulting 120 ms does not by itself describe how long the group took to finish.

## Follow the implementation when ready

The [group summaries](../../crates/netscope-core/src/hosts.rs) hold the derived counts, and the [session](../../crates/netscope-core/src/session.rs) builds them while preserving request IDs. The [host screen](../../crates/netscope-tui/src/host_view.rs) presents the values, with navigation in the [view state](../../crates/netscope-tui/src/view.rs).

The [host tests](../../crates/netscope-import/tests/hosts.rs) check grouping, order, status counts, zero, and overflow. The [host-view contract](../contracts/m2-host-summaries.md) defines the exact policy. The [next chapter](13-following-redirects.md) asks a different question: when can two requests plausibly be related? Counting a group is exact under our grouping rule; explaining why one request followed another needs more care.
