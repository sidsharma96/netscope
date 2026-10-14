# Inspect your first capture

The first useful version of Netscope answers a small set of questions about a recording. What requests does it contain? What did one request send and receive? Which measurements are present, and which are missing?

Use the supplied synthetic capture for this walkthrough. It contains invented traffic to example.test, including repeated headers and a short response body. You do not need a recording from a logged-in browser session.

## Open the sample

From the project directory, run:

```sh
cargo run -p netscope-tui --locked --offline -- tests/fixtures/har/single-request.har
```

Cargo is Rust's build tool. This command builds and starts the terminal application, then passes it the sample file. The [development guide](../DEVELOPMENT.md) covers toolchain and dependency setup if this is your first run on another machine.

In an interactive terminal, you should see one request for `https://example.test/api/books`, status 200, and a reported total of 45 ms. A marker and filled row identify the selection. On a window at least 100 columns wide, its summary appears beside the list. At narrower supported widths, Enter opens the details on their own screen. The narrow list's bar shows recorded start and total; it does not identify a particular network connection.

If you redirect the output to a file or pipe, the program prints a text summary instead of opening the interactive screen.

## Read one request

Press Enter to focus the selected request's inspector. Summary answers the first questions: what was requested, which status was recorded, and what kind of response representation remains? The selected request stays visible in the list when the window has room for both panes.

Press 2 for Headers and find the two `X-Example` fields. Both remain present and in their original order. This is a useful check that the display has not quietly replaced repeated fields with one value.

Press 3 for Body. Find `[]`, the JSON representation of an empty list. This body contains text; it is different from a captured empty string and from a body that was not recorded. Down or j scrolls the focused details; Up or k moves back.

Press 4 for Timing. Connecting takes 30 ms and SSL takes 20 ms within that interval. The [capture chapter](02-reading-a-capture.md) explains why those numbers are not added as separate stages. Press g for a journey that explains each reported stage. Its arrows illustrate HTTP direction; they are not recovered handshake messages. Press g or Esc to return to the section.

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

The [M1 contract](../contracts/m1-har-inspection.md) records accepted input, size limits, screen behaviour, and acceptance cases. M1 means the first milestone. The [verification record](../VERIFICATION.md) distinguishes checks that have run from work still awaiting review.

The older HTML mockups remain design history. The written contract describes the current application. The [replay chapters](16-preparing-a-request.md) explain the separate decision to send a new request.
