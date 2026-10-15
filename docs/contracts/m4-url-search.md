# URL search

Implemented October 12, 2026. Search is a local terminal view over an immutable imported session and starts no network activity.

- `/` from Requests edits a separate copy of the applied query. Enter applies; Esc cancels. While editing, text takes precedence over browsing shortcuts. Ctrl+C still quits. Release events are ignored; a too-small terminal preserves the editor and ignores edits.
- Match an exact, case-sensitive literal substring of the retained full URL. Do not decode escapes, normalize case, interpret regular expressions, or search bodies and headers. Empty query means no URL condition.
- Match both the selected host and URL condition. Keep chronological order and original request IDs. Host summaries and capture totals continue to describe the full session.
- Cache matching IDs on query/host changes. No repeated search on redraw or movement. Never copy or mutate captured requests for filtering.
- On a query change, preserve selection by original ID if present; otherwise select the first result, or none when empty. Host changes while searching use the same rule. Reset the overview scroll and bring selection into view.
- `a` clears both conditions. Host-only browsing retains its existing saved return position; search edits invalidate that saved row. With search involved, retain the selected ID when returning to the full list if possible.
- Display matching/full counts, active query, and host while searching. Empty results have an explicit message and permit editing or clearing; inspection and navigation cannot invent a row.
- Left/Right and deletion use grapheme boundaries; Home/End, Backspace/Delete, and Ctrl+u are supported. Accept printable character events up to 1024 UTF-8 bytes, rejecting an over-limit character without partial insertion. The cursor marker stays visible by showing the nearby portion of a long edit. Escape captured terminal controls before rendering.

Evidence: state and rendering [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66), deterministic tests [`search_uses_literal_full_urls_and_keeps_source_identity`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/tests/views.rs#L2213), [navigation lesson](../chapters/11-drawing-and-navigation.md).

The byte limit bounds query storage, not search time. Applying a query can still scan all imported requests on the terminal thread. Reconsider this approach if measured apply latency requires background work; this implementation does not claim a latency bound.
