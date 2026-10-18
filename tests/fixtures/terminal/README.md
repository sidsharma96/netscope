# Reviewed terminal buffers

These text files capture the complete Ratatui TestBackend cells for the synthetic single-request capture. They include trailing spaces deliberately. Overview fixtures cover 100 × 30 and 80 × 24; the inspector fixture covers its first page at 80 × 24. Tests compare the complete buffers and separately check selection styling, scrolling, Unicode, representation labels, and fallback behaviour.

Regenerate deliberately after reviewing an intended layout change, for example:

```sh
cargo run -q -p netscope-tui --example screen_fixture --locked --offline -- 80 > tests/fixtures/terminal/overview-80x24.txt
```

Use 100 for the larger overview and add inspector for the inspector view. These are text cell fixtures, not terminal-emulator screenshots. The separate pseudo-terminal check verifies terminal mode restoration.

Host and filtered-list fixtures use multi-host.har at both supported sizes. Generate them with mode hosts or filtered after the width argument. Existing overview and inspector fixtures also record the host-navigation key legend.

The retries fixtures show the bottom of request #0 from retries.har, including ambiguous candidates and the matching limits. Regenerate them with the screen_fixture example's retries mode at widths 80 and 100. The fixed buffers retain every terminal column, including trailing spaces.

The protocols fixtures show request #0 from protocols.har after scrolling to its protocol notes. The labels HTTP/2 and h2 agree while their original text remains visible. Regenerate with screen_fixture protocols mode at widths 80 and 100.

The draft, draft-invalid, and review fixtures show the current editor and explicitly reviewed send boundary at 80 × 24 and 100 × 30. The screen_fixture modes with those names regenerate them. Wide replay uses a workbench; narrow replay keeps one active pane. Comparison and exact save previews use the full width beneath the operation strip.

The October 13 layout pass adds headers, body, and journey modes at 80, 100, and 120 columns, and default inspector buffers at the wider sizes. Wide overviews preview the selected request beside the list. The original full-report scrolling tests now deliberately choose All (6). The sse mode scrolls All until its derived-event heading reaches the first content row (row 8, counting from zero); SSE fixtures omit the final newline to match their test's buffer representation. Do not count these generated examples as native browser captures.

The October 14 journey buffers use selectable stages beside a focused explanation, with a labelled duration scale and explicit comparison of reported total with known-stage sum. `journey-gaps` opens synthetic request #2 from multi-host.har (70 ms total, 5 ms known sum); `journey-tls` selects SSL in the single-request capture. Both modes are checked at 80, 100 and 120 columns. A third example argument can set the height for manual layout review. Large windows cap the journey workspace instead of stretching every paragraph.

The October 18 guidance pass preserves full tab names in two rows, adds reading prompts and scroll position labels, and adds host outcome bars and packet direction guides. Replay fixtures use a fixed initial activity phase; a unit test advances supplied time and verifies stationary reduced motion and absence of extra sends or saves.

Packet learning buffers add `packet-explained` and `stream-explained` at 80×24, 100×30 and 120×30. Regenerate through `NETSCOPE_UPDATE_PACKET_VIEWS=1 cargo test -p netscope-tui --test packets --locked --offline`. The existing layer/evidence buffers explicitly choose 2 Recorded report. These are synthetic offline fixtures.
