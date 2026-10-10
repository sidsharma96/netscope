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

The draft, draft-invalid, and review fixtures show the replay editor with Send disabled at 80 × 24 and 100 × 30. The screen_fixture modes with those names regenerate them. Inspector fixtures also disclose the r replay shortcut.
