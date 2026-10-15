# Large captures: navigation and local measurements

Verified October 11, 2026, on an Apple M1 Mac with the pinned Rust toolchain and a release build. The importer still accepts at most 10,000 entries and 32 MiB of input. Those limits are independent: a capture with fewer entries can exceed the byte limit.

## Moving around

Page Up and Page Down move by the current view's visible row count. Home and End select the first and last rows in Requests and Hosts, or the first and last visible page positions in the inspector. A host filter confines request navigation to that host's chronological IDs. Returning to all requests restores its saved position. Help and undersized windows ignore movement. Empty lists remain unselected.

At 80 × 24, a request page has 15 rows and a host page has 7; at 100 × 30, they have 21 and 13. Inspector movement uses its wrapped lines and available height. Resizing changes the subsequent page step. These shortcuts apply to capture browsing; editable replay fields retain their existing Home/End cursor behaviour.

## What was measured

Run `python3 scripts/probe_large_capture.py` from the repository root. It generates temporary HAR files from fixed content and timestamps, builds the release `capture_probe` example, and starts it three times per case. Each process reads the file before timing import, then warms up one 100 × 30 in-memory frame and measures 200 Down-and-draw operations. No external destinations are contacted. Temporary captures are removed afterward.

| Workload, 10,000 entries | Input bytes | Median import before | Median import after |
| --- | --- | --- | --- |
| Distinct URLs across 100 hosts | 6,468,911 | 142.855 ms | 129.546 ms |
| Overlapping requests with 503 responses | 6,400,021 | 419.094 ms | 122.225 ms |
| Overlapping redirects to the same URL | 6,800,021 | 408.158 ms | 134.839 ms |

The [raw samples](measurements/large-capture-2026-10-11.csv) include 1,000-entry runs and every measured frame average. The baseline used application revision ae119c8 with the same newly added measurement harness. The after samples include the matching optimization and new navigation actions. Runs were sequential, not a controlled benchmark campaign; machine load and allocation effects can influence them. The distinct-URL difference should not be credited to an optimization that targets overlapping candidates.

For the overlapping workloads, every request lasts ten seconds and starts one millisecond after its predecessor. Their timing stages remain illustrative fixture data, and inconsistency warnings are expected. These cases deliberately stress rejecting later starts that still precede a source's reported completion. No follow-up qualifies. Binary search now skips that entire prefix of the sorted candidate list rather than visiting each ineligible entry. The strict later-start rule, inclusive reported-end boundary, 30-second window, stable ties, and 20-candidate cap remain unchanged.

Across the 10,000-entry after runs, average Down-and-draw work was about 0.21–0.22 ms per frame in Ratatui's TestBackend. This excludes terminal output, event delivery, file reading, and operating-system scheduling delays. It measures the first 200 movements, not every list position or inspector body. It is not an end-to-end frame-rate guarantee. The overview drawing path was not optimized in this slice.

## Deterministic checks and limits of the result

The regular test suite imports 10,000 entries, preserves source IDs for timestamp ties, and rejects 10,001 entries. A separate 10,000-request, 100-host view test checks paging, first/last movement, filtering, returning to the previous all-requests position, inspector bounds, help, resize, and empty input at both supported test dimensions. Relationship tests compare the indexed result against a full scan across reversed source order, equal starts, zero/positive/long durations, and window boundaries. No speed threshold appears in these tests.

The matching optimization removes one source of repeated work; worst-case comparisons can still be quadratic when many candidates lie inside the window but fail method or metadata checks. Long headers, large bodies, many distinct hosts, allocation pressure, and peak memory were not comprehensively profiled. Import is still synchronous. These observations do not justify raising the input limits or claiming that every allowed file loads within the measured times.

## Packet captures are not measured here

The measurements above cover HAR import only. PCAP import has separate limits: 32 MiB of file, 20,000 records, and 256 KiB per record. A file with more than 20,000 records keeps the first 20,000, and a truncated final record (the usual result of killing `tcpdump`) is dropped while the complete records before it are kept. Either case adds a file-level note to the packet view. Reassembly then allows at most 8 Mi sequence positions in total, at two bytes each. The resulting peak of roughly 80 MiB for a maximum import is derived from those constants, not measured. [Defending the design](chapters/34-defending-the-design.md#how-are-memory-and-time-bounded-in-reassembly) shows the calculation.
