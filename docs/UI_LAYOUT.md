# Inspection and replay workspace

This layout follows the October 13, 2026 design discussion. The split inspector is the default browsing view on terminals at least 100 columns wide. The journey is optional; the replay workbench is used only after opening a draft. These are complementary views, not competing defaults.

## Browse and inspect

At 100 columns or wider, the request list occupies 40% of the width. The remaining space previews the selected request. Enter or Tab focuses the inspector; Tab or Esc returns to list navigation. The bright border identifies the pane receiving movement keys, while a filled row identifies the selected request. Up/Down and page/home/end keys select requests when the list has focus and scroll details when the inspector has focus.

At 80–99 columns, the list and inspector occupy separate screens. The same selection, filters, section choice, and source IDs remain in use. Below 80 × 24 the existing resize message preserves state. This avoids squeezed panes and a second set of navigation rules.

Inspector sections use number keys:

| Key | Section | Contents |
|---|---|---|
| 1 | Summary | Status, reported total, retained representation, capture boundaries |
| 2 | Headers | Request/response fields, query fields, cookies; duplicates retained |
| 3 | Body | Request and response previews; retained SSE analysis when eligible |
| 4 | Timing | Reported stages, including missing, not-applicable (-1), and non-duration distinctions |
| 5 | Evidence | Import warnings, protocol labels, WebSocket upgrade metadata, redirect/retry candidates |
| 6 | All | Complete original report, including sizes and exporter connection/IP fields |

Full tab names occupy two rows, including in the split pane. The selected tab has a filled background. A bottom-border position label indicates additional content and whether Tab is needed before scrolling. Changing section resets its scroll position. A wide list shows paths, with the selected request's host and URL available in its details. Long URLs remain fully accessible in All; the two-line heading is a clipped summary. Original previews and their existing byte limits remain unchanged; this layout adds neither decompression nor body normalization. Replay header masking follows its existing rules; captured headers retain their previous display policy.

Press g from the list or inspector for a request journey. It presents HTTP-direction arrows explicitly labelled as an illustration, reported stage values, and short explanations of those stages. It does not reconstruct DNS/TCP/TLS messages, certificate checks, server processing time, or event arrival times. Missing stages remain missing; SSL is part of connect. Press g or Esc to return to the chosen section at its beginning. Number keys also return directly to a section.

### Request journey

Following the October 14 screenshot review, the journey uses a compact exchange diagram above two panes: all seven timing fields on the left and the selected field's explanation on the right. It starts on Wait. Up/Down selects a stage; Home/End selects the first/last. Page Up/Down scrolls only the explanation when it does not fit. Changing stage returns that explanation to its beginning. The stage list remains visible at the supported 80 × 24 minimum, and the layout stops growing at 140 columns and 30 body rows in a larger terminal.

The total and sum of known main-stage durations are shown together. The sum excludes SSL because it is included in Connect, and excludes missing, unavailable, and exporter-negative non-duration values. A difference above 0.01 ms is highlighted without assigning it to an unknown stage or naming a cause. Fractional sums are rounded for display with an approximation sign; original values remain available. Numeric sum overflow has its own label.

Bars compare durations on an explicitly labelled common scale: the larger of the reported total and largest reported stage. They do not place stages on a reconstructed timeline. Zero has no filled bar; missing/unavailable values show a dash; non-duration exporter values have an asterisk and no duration bar. The selected explanation distinguishes these cases. Selecting TLS on a plain HTTP URL does not call a missing SSL value a failed handshake. Reported SSL/Connect inconsistencies remain visible as conflicting or insufficient evidence.

## Edit and review

Replay keeps Edit → Review → Send → Result visible. At 100 columns or wider, editing uses the left pane with context and any labelled older result on the right. Review, sending, and result inspection use the right pane with request context on the left. Text editing remains in one active pane: Tab inserts a tab in a body field rather than changing focus. Narrow terminals show the active pane alone, with the same operation strip.

The strip describes the current state, not four independently clickable controls. Drawing, resizing, changing inspection section, and entering Review queue no send. Enter or s in a valid review remains the explicit send boundary. Live SSE has its separate review; cancellation, immutable sent values, unique operation IDs, and retained partial results follow the existing contracts. Saved-file previews and response comparison use the full available width. A retained result is labelled with its own destination and is not silently updated to match a later draft.

## Verification

Fixed cell buffers cover both original sizes and new 120-column section/journey views. State tests exercise focus, identity, resize, text-key routing, missing evidence, and absence of send/save effects from rendering. Process checks exercise terminal restoration, real local sends, comparison, cancellation, saved drafts, and SSE. These checks establish specified behaviour; they do not substitute for user review of the appearance or guarantee identical Unicode rendering in every terminal.

The [architecture record](ARCHITECTURE.md) explains alternatives and costs. [Development](DEVELOPMENT.md) contains the exact commands; the [book](chapters/11-drawing-and-navigation.md) explains how focus differs from selection.

## Beginner guidance and activity

The October 18, 2026 pass gives each inspection section a short reading prompt. Summary explains the recorded status class before listing measurements. Headers, Body, Timing and Evidence explain what to look for; All retains the full original report. Help begins with a route through inspection before listing navigation and replay controls. Empty search results offer a way to change or clear the filters.

Hosts retain the alphabetical table and add outcome bars for the selected host. Each bar uses that host's request count as its denominator; the displayed counts remain exact even when the bar rounds down to a terminal cell. Other status classes and missing statuses remain labelled. Summed request durations do not measure elapsed page-load time because requests can overlap.

Packet and stream views share the charcoal background, mint focus border and selected-row fill. A guide above the records explains a packet or one-way TCP stream and shows the selected endpoints when available. These arrows represent recorded direction, not packet animation or a verified receiver transcript. The record pane uses its actual remaining height for paging. The comparison view keeps aligned rows and marks differing statuses and header values with both words and amber colour.

Replay has numbered Edit → Review → Send → Result steps and a short next-action hint. The active network operation has a rotating indicator, advancing every 180 ms. This indicates activity only: it is not a percentage, a handshake stage, or proof of response progress. Observed SSE data continues to use the existing retained-byte and event observations. The terminal supplies elapsed time; the renderer reads no clock. Fixed-buffer fixtures therefore keep a stable initial phase.

Set `NETSCOPE_REDUCE_MOTION=1` before starting Netscope for a stationary activity indicator. Any presence of this variable enables reduced motion, including a value of `0`. Captured data, editing and completed results do not animate. Colour supplements labels and selection markers; it is never the only indication of state.

## Packet and stream learning panes

Packet and stream inspectors now open in **1 Explain**. The recorded-value summary stays above a topic list and explanation pane. Up/Down selects a topic, Home/End selects the first/last topic, and Page Up/Down scrolls its explanation. Selecting another topic starts at the top. **2 Recorded report** retains the prior complete scrollable report; **1** returns to the selected explanation. Esc returns directly to the original list selection. Help describes the two movement modes. At large sizes the learning workspace stops growing at 140 columns and 30 rows; it remains available at 80×24.

TCP topics cover flags, sequence positions, acknowledgements, and stored layers/bytes. ACK-off fields are labelled inactive. SYN adjusts the first data position, FIN consumes a position after data, and sequence arithmetic wraps at 32 bits. Packet byte offsets remain distinct from stream positions. UDP and unsupported headers receive separate explanations instead of fabricated TCP values.

Stream topics cover a position map, exact ranges, beginning/end evidence, and the contiguous prefix available to HTTP inspection. The 40-cell map groups adjacent retained positions with conflict-over-missing-over-observed priority. Exact counts accompany the map; a conflict cell may also contain missing positions. Empty spans are explicitly labelled. Exact runs are bounded to 64 with a truncation label. No map uses time, animation, inferred delivery, or bytes outside the retained span.
