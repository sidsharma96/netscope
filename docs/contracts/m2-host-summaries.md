# M2, first step: host summaries and request filtering

This step adds a host view to offline inspection. Redirect suggestions are defined separately in the [redirect contract](m2-redirect-suggestions.md). Retry suggestions have their own [evidence contract](m2-retry-suggestions.md). Recorded protocol explanations are defined by the [protocol contract](m2-recorded-protocols.md).

## Meaning of a group

Group requests by the normalised URL host already stored by the importer. Host IDs retain first-source-occurrence identity; display hosts lexicographically. Schemes and ports do not create separate host groups. This is a URL-host grouping, not a list of servers, origins, connections, or organisations. Do not infer a host's purpose, connection reuse, or a shared destination from this grouping.

Each summary contains the count of source entries, counts of reported status classes 1xx through 5xx, the count with no HTTP status recorded, the number of entries with import warnings, and the sum of reported entry durations. Each entry contributes exactly once. Status zero remains separate from HTTP status classes and is not diagnosed as a failure.

Sum durations in source order. Overlapping requests still contribute their individual reported durations, so label the result as a sum and explain that it is not elapsed time. If addition exceeds the finite numeric range, retain the requests and display Overflow instead of infinity or a partial sum. A recorded sum of zero remains zero. Do not sum SSL separately or substitute timing-stage sums for entry.time.

A host's request IDs follow the same chronological order and source-order tie rule as the complete capture. Filtering never renumbers requests or clones their bodies. Request timing bars retain the complete capture's time scale when a filter is applied.

## Navigation and display

Press h from the request list or inspector to open Hosts. Up/Down and j/k select a host. Enter opens that host's chronological request list. Enter again opens a request inspector. Esc from the inspector returns to its filtered list; Esc from a filtered list opens Hosts. Esc or h from Hosts returns to the view from which Hosts was opened. Choosing a different host resets its request selection to the first row; choosing the current host retains that list's position.

Press a from a request list or Hosts to return to all requests. Restore the selection and scroll position saved before the first host filter. Opening help and returning, or shrinking below the minimum terminal size and enlarging it, preserves current state. q works from every screen, including help and the small-terminal fallback.

At both 100 × 30 and 80 × 24, the Hosts view has a filename, capture count, labelled host table, selected-row marker and contrast, and a key legend. Rows show the host, entry count, 4xx count, 5xx count, count with no status, and summed reported duration. A detail area previews the selected host name with wrapping, all status-class counts, and count of requests with import warnings. A label states that requests can overlap. Empty captures have no selectable host.

A filtered list clearly names its host and displays the shown count and capture count. A long name may be abbreviated there; the individual inspector retains access to the complete URL. Extremely long host names are labelled as shortened in the host detail area. Captured strings receive the same control escaping and Unicode-aware layout as other views. No network traffic is sent.

## Acceptance

Use a synthetic multi-host capture with mixed URL case, schemes and ports, equal instants, status zero, error statuses, warning-bearing entries, and overlapping durations. Check exact counts, first-occurrence host IDs, alphabetical display, chronological filtered IDs, zero durations, and sum overflow. Verify that normalisation merges the intended host spellings without merging distinct subdomains.

Test the navigation sequence through Hosts, a filtered list, an inspector, and back to all requests, including empty input, scroll bounds, help, resize, and q. Compare full fixed buffers for Hosts and a filtered list at the two supported sizes, and verify selection styling. Extend the separate pseudo-terminal check to exercise host navigation before quitting.

The learning chapter explains grouping, repeated operations, and the meaning of aggregates through a small capture. It distinguishes a sum from elapsed time before introducing the implementation. Verification results belong in the ledger.
