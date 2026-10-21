<a id="inspection-and-replay-workspace"></a>

# Using Netscope

Open a recording, select an entry, and inspect its details. The bright border marks the pane receiving your keys. In a wide terminal the list stays beside the details; in a narrower terminal you move between them. Netscope needs at least 80 columns and 24 rows.

## Browse and inspect

Use Up/Down to select a request, Enter to inspect it, and Esc to return. Tab switches between list and details when both are visible. Page Up/Down moves by a screen; Home/End goes to either end. Press `?` for help and `q` to quit.

Choose an inspector section with its number:

| Key | Section | Use it to find |
|---|---|---|
| 1 | Summary | The destination, response status, and available content |
| 2 | Headers | Request and response fields, cookies, and query values |
| 3 | Body | Captured request and response content |
| 4 | Timing | Recorded durations and missing measurements |
| 5 | Evidence | Warnings and possible relationships between requests |
| 6 | All | The complete report |

Captured content can include private information. Inspecting a file does not remove that information or make the file safe to share.

### Search and hosts

Press `/`, type part of a URL, and press Enter to apply the search. Matching is case-sensitive and treats punctuation literally: `books` matches `/books`, while `Books` does not. Esc cancels an unfinished search; applying empty text clears it.

Press `h` to see host groups, then Enter to filter the request list by a host. A host filter and URL search both have to match. Press `a` to clear both. A host's summed durations can include overlapping requests, so the sum does not measure total page-load time.

### Request journey

Press `g` to see timing stages beside explanations. Up/Down selects a stage; Page Up/Down scrolls its explanation. Press `g` or Esc to return. The bars compare reported durations. The arrows illustrate request direction; they do not reconstruct missing messages or explain an unrecorded delay.

## Edit and review

Press `r` from a supported captured request to create a draft. Up/Down selects Method, URL, Headers, or Body; Enter changes or opens the selected field. In Headers, Enter selects or deselects a captured field, or removes one you added. The Add a header row accepts a new name and value.

Changing the URL clears the selected headers and body. They were chosen for the old destination. Header values stay hidden until you reveal them with `v`; editing a value shows what you type.

Choose Review request after editing. Left/Right or Tab switches between Summary, Headers, and Body. Check the destination and outgoing information, then press Enter or `s` to send. Esc returns to editing. Opening Review does not send anything.

During a send, Esc cancels local waiting. The server may already have received and processed the request. The result keeps available response data and explains why the operation ended. Press `c` on a result to compare it with its captured source. From the draft, `o` reopens a retained result, which continues to describe its original send even if you have since edited the draft.

### Text editing and saving

Left/Right moves through displayed character groups. Backspace removes the previous group and Delete removes the next. Home/End goes to the beginning/end of the field; Ctrl+u clears it. Enter saves an edit and Esc abandons it. In a body field, Ctrl+n inserts a newline and Tab inserts a tab. While editing, ordinary letters become text rather than browsing shortcuts.

Press `w` from a valid draft to choose a name and file path, then inspect the save preview before confirming. Reopen with `netscope --draft request.json`. Known credential headers are omitted, but the URL, body, and other headers can still contain secrets.

### Event streams

A captured server-sent event response, or SSE response, shows its retained events under Body. Those saved events have no recovered arrival times. To make a new live observation, press `l` from a GET review, inspect the separate live review, then explicitly send. The result reports when Netscope observed complete events. See the [live-events lesson](chapters/27-observing-live-events.md) for what those times mean.

## Packet and stream learning panes

Open a classic packet recording with `netscope --pcap capture.pcap`. Use `p` for packets and `s` for streams, then Enter to inspect a selected entry.

In a packet, Up/Down selects a field such as flags or acknowledgement. In a stream, it selects a byte range. The explanation follows the selection. The stream map shows observed, missing, and conflicting positions; its scale is bytes, not time. HTTP reading stops at a gap or conflict, while later retained bytes remain inspectable.

Press `h` for a longer explanation and Page Up/Down to scroll it. In a stream, `b` switches text and hexadecimal byte previews; `[` and `]` page through bytes. Press `2` for the recorded report and `1` to return to inspection. Esc returns to the list. If the keyboard-help overlay is open, Esc closes that first.

## Beginner guidance and activity

Explanations describe the selected information. A moving indicator appears only during active network work; it is not a completion percentage. For a stationary indicator on macOS or Linux, start with:

```sh
NETSCOPE_REDUCE_MOTION=1 netscope capture.har
```

Any presence of this environment variable enables reduced motion. Remove it entirely to restore animation.

## Verification

Contributors can find the checks in [development](DEVELOPMENT.md#terminal-verification). The [navigation lesson](chapters/11-drawing-and-navigation.md) explains why selection, focus, and scrolling have separate state.
