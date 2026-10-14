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
| 4 | Timing | Reported stages, including missing/unavailable/non-duration distinctions |
| 5 | Evidence | Import warnings, protocol labels, WebSocket upgrade metadata, redirect/retry candidates |
| 6 | All | Complete original report, including sizes and exporter connection/IP fields |

Short tab names fit the split pane; the expanded names appear in the panel title and help. Changing section resets its scroll position. A wide list shows paths, with the selected request's host and URL available in its details. Long URLs remain fully accessible in All; the two-line heading is a clipped summary. Original previews and their existing byte limits remain unchanged; this layout adds neither decompression nor body normalization. Replay header masking follows its existing rules; captured headers retain their previous display policy.

Press g from the list or inspector for a request journey. It presents HTTP-direction arrows explicitly labelled as an illustration, reported stage values, and short explanations of those stages. It does not reconstruct DNS/TCP/TLS messages, certificate checks, server processing time, or event arrival times. Missing stages remain missing; SSL is part of connect. Press g or Esc to return to the chosen section at its beginning. Number keys also return directly to a section.

## Edit and review

Replay keeps Edit → Review → Send → Result visible. At 100 columns or wider, editing uses the left pane with context and any labelled older result on the right. Review, sending, and result inspection use the right pane with request context on the left. Text editing remains in one active pane: Tab inserts a tab in a body field rather than changing focus. Narrow terminals show the active pane alone, with the same operation strip.

The strip describes the current state, not four independently clickable controls. Drawing, resizing, changing inspection section, and entering Review queue no send. Enter or s in a valid review remains the explicit send boundary. Live SSE has its separate review; cancellation, immutable sent values, unique operation IDs, and retained partial results follow the existing contracts. Saved-file previews and response comparison use the full available width. A retained result is labelled with its own destination and is not silently updated to match a later draft.

## Verification

Fixed cell buffers cover both original sizes and new 120-column section/journey views. State tests exercise focus, identity, resize, text-key routing, missing evidence, and absence of send/save effects from rendering. Process checks exercise terminal restoration, real local sends, comparison, cancellation, saved drafts, and SSE. These checks establish specified behaviour; they do not substitute for user review of the appearance or guarantee identical Unicode rendering in every terminal.

The [architecture record](ARCHITECTURE.md) explains alternatives and costs. [Development](DEVELOPMENT.md) contains the exact commands; the [book](chapters/11-drawing-and-navigation.md) explains how focus differs from selection.
