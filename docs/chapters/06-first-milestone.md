# Inspect your first capture

Opening a recording gives you a few questions to investigate. What requests does it contain? What did one request send and receive? Which measurements are present, and which are missing?

Use the supplied synthetic capture for this walkthrough. It contains invented traffic to example.test, including repeated headers and a short response body. You do not need a recording from a logged-in browser session.

## Open the sample

After [installing Netscope](../INSTALLATION.md), [download the sample capture](https://raw.githubusercontent.com/sidsharma96/netscope/main/tests/fixtures/har/single-request.har). Save it as `single-request.har`, open a terminal in that folder, and run:

```sh
netscope single-request.har
```

You should see one request for `https://example.test/api/books`, status 200, and a reported total of 45 milliseconds. Status 200 reports a successful response. A marker and filled row identify the selected request. In a wide window, its summary appears beside the list; in a narrower window, Enter opens its details.

If you already have the repository, the same sample is at `tests/fixtures/har/single-request.har`. A file path starts from your terminal's current folder. Redirecting output to a file produces a text summary instead of the interactive view.

## Read one request

Press Enter to focus the selected request's inspector. Summary answers the first questions: what was requested, which status was recorded, and what kind of response representation remains? The selected request stays visible in the list when the window has room for both panes.

Press 2 for Headers and find the two `X-Example` fields. Both remain present and in their original order. This is a useful check that the display has not quietly replaced repeated fields with one value.

Press 3 for Body. Find `[]`, the JSON representation of an empty list. This body contains text; it is different from a captured empty string and from a body that was not recorded. Down or j scrolls the focused details; Up or k moves back.

Press 4 for Timing. Connecting takes 30 ms and SSL takes 20 ms within that interval. The [capture chapter](02-reading-a-capture.md) explains why those numbers are not added as separate stages. Press g for a journey that keeps the reported stages beside an explanation of the selected one. Up and Down choose a stage; Page Up and Page Down scroll its explanation when needed. Its arrows illustrate HTTP direction; they are not recovered handshake messages. Press g or Esc to return to the section.

Evidence, under 5, gathers warnings and qualified interpretations. All, under 6, keeps the complete report available when you need fields outside the shorter sections. Choosing a section changes your place in the interface, not the captured request.

## Return to where you were

Press Esc to return to list navigation. The same row remains selected. Tab also switches between the list and details; the bright border identifies which pane receives movement keys. Press ? to see the keys, and press Esc again to close help. Press q from either screen to quit and restore the terminal.

To try selection between several requests, repeat the command with `tests/fixtures/har/stable-order.har`. The displayed source IDs are 1, 2, and 0 because the list is chronological. IDs 1 and 2 name the same start instant and keep their source order. Move with the arrow keys, open a request, then return.

The viewer needs at least 80 columns and 24 rows. A smaller window shows a size message and still accepts q. Enlarging the window resumes the view you were using.

## Notice the limits of an observation

Open `tests/fixtures/har/missing-body.har` and inspect its response body. The label says `Response body not recorded`. Explain why it would be misleading to replace that label with `Captured body is empty`.

Inspecting a capture reads local data. It does not initiate replay or recover network messages omitted from the file. Sending is a separate, explicit action from a reviewed draft. Keeping those limits visible makes the first version useful without implying that the recording contains more than it does.

## Explain what the behaviour requires

After trying the screens, describe what the application must remember when you press Enter and then Esc. Which information belongs to the capture, and which describes your place in the interface?

Then explain how missing timing information can survive the journey from file to screen. You can answer in ordinary language before looking at a Rust type or function name. The ownership, collection, and missing-value chapters give you the tools to connect that explanation to code.

## Exact rules and completed checks

The [usage guide](../UI_LAYOUT.md) collects the controls. The [input reference](../contracts/m1-har-inspection.md) lists accepted fields and limits. Continue with [understanding a request](01-start-here.md) to begin the learning sequence, or [preparing a request](16-preparing-a-request.md) to study how an editable draft stays separate from the capture.
