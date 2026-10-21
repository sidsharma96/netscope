# Keeping your place on screen

You select the second request, open its details, and choose Body. When you press Esc, you expect to return to the same request in the list. If the window becomes smaller, you still expect the program to remember what you were inspecting.

Those expectations tell us what the interface needs to remember. The capture itself stays the same while your place within it changes.

## Separate the capture from your place in it

The session owns the recorded requests. A smaller set of values records the selected row, the active screen, and the scroll position. Together, these values are the view state: the information needed to decide what to show next.

Opening the inspector changes that state to identify one request. Returning to the overview restores its saved selection and scroll position. We do not need another copy of the captured body to remember either screen.

*Figure 8.1. Keep the selection while the view changes.*

```text
Request list                         Request details
Request 1                            Read request 2
> Request 2 ─────── Enter ──────────→ Scroll through its fields
Request 0
         ←────────── Esc ────────────
Same selection and list position

The view state changes. Both screens read the same stored capture.
```

When the capture is empty, there is no selected request. Rust's `Option` represents that absence. Pressing Enter can therefore leave the empty view in place instead of attempting to look up a nonexistent row.

## Describe the screen with an enum

When browsing requests, we switch between the overview and an inspector for a particular request. Those alternatives fit an [enum](../GLOSSARY.md#enum). In Netscope, they are named `Overview` and `Inspector(RequestId)`.

The host view introduced in the next chapter adds another screen variant using the same idea.

The parentheses mean that the inspector variant carries a request ID. This keeps the fact that we are inspecting a request together with the identity of that request. A separate true-or-false flag would still need another value to say which request it meant.

The same key can have different meanings in those two states. Down selects the next request in the overview and scrolls one line in the inspector. A `match` on the current screen and action makes that decision explicit. The result is another state we can inspect in a test.

## Which pane should a key change?

Imagine keeping the request list visible beside its details. You can now see both at once, but pressing Down still needs one meaning. Should it select the next request or scroll the current body? Visibility alone cannot answer that question.

Focus identifies the part of the interface receiving navigation keys. Selection identifies the request whose details are shown. They are different: switching focus to the inspector should keep the same request selected. In a wide Netscope window, the bright pane border marks focus and the filled request row marks selection. Tab switches focus; Enter focuses the selected request's details.

Read the following drawing as changes to navigation responsibility. Both sides refer to the same captured request. The arrows do not represent a network exchange.

```text
Illustrative focus change; captured request #2 stays unchanged

List has focus                     Details have focus
Down selects another request  ─Tab→ Down scrolls request #2
                              ←Tab─
```

The overview and inspector alternatives therefore still have a purpose even when both panes are drawn. They identify what a movement key should change. When the window is narrower, the same alternatives show one pane at a time. That lets the program adapt its presentation without inventing a second navigation model.

Choosing Headers or Body answers another question: which information about this request do you want to examine? That section choice belongs to view state too. An enum can name the available sections, while the captured header and body values stay in the session. Changing sections begins at the top of the newly chosen content, so a scroll position from a long body cannot hide a shorter summary.

The journey view adds an explanation of a selected timing stage. Its selected stage and its explanation's scroll position are separate values too: scrolling a long explanation should not select a different measurement. The same state model supports a smaller window without changing the captured values.

## What should a moving indicator mean?

A moving indicator can show that a send is still active. It cannot tell us how much remains: a server might reply now or keep a response open. Netscope animates active work and leaves recorded requests still. Drawing receives elapsed time from the terminal loop, which lets a test choose a particular moment and reproduce the same display. The [usage guide](../UI_LAYOUT.md) explains the reduced-motion option.

## Borrow data, change selection

Drawing needs read access to the capture. Navigation needs to change the view state. Rust distinguishes those permissions in the references passed to a function.

`&Session` is a shared reference. It lets the drawing code inspect the session while its owner keeps the data alive. `&mut App` is a mutable reference to the value holding view state. It permits updates, such as adjusting a scroll offset after a resize.

Here, `mut` follows the ampersand because it describes the reference's permission. This is related to, but distinct from, writing `let mut selected`: that earlier example made a local variable changeable. A mutable reference lends change access to an existing value.

Rust checks that this mutable access is exclusive while it is in use. The caller cannot also use a conflicting reference to the same state during that borrow. After drawing returns, the event loop can borrow the state again to apply the next action.

The drawing function also receives a mutable frame. A frame represents the terminal cells being prepared for the current screen. Writing those cells changes the display, while the borrowed capture remains intact.

## Move through a list that does not fit

Suppose a capture contains a hundred requests and the window shows fifteen. Selection identifies the request you want; the scroll offset identifies the first visible row. After a selection moves, the program changes the offset only as far as needed to keep that row visible.

Page Down advances by the number of rows that fit. Near the end of the list, it stops at the last request. Moving upward must also stop at the beginning: subtracting fifteen from row three must produce row zero. Rust's saturating subtraction expresses that rule by stopping at zero when an unsigned result would otherwise go below it.

Filtering changes the rows available for navigation, but their request IDs still refer to the original capture. Resizing changes how many rows fit without changing those IDs. Keeping identity, selection, and scroll position separate gives each action a clear responsibility.

## Find a request without changing the capture

Searching for `/books` narrows the list to matching URLs. The program keeps a list of matching request IDs, so opening a result still reads the original request. Clearing the search makes the other requests visible again; importing the file again is unnecessary.

A row's position can change after filtering. To retain the same selection, remember its request ID and find that ID in the new results. If it is absent, select the first result. If the results are empty, there is no selection. `Option` lets us represent that last case without pretending that row zero exists.

The capture stays unchanged while you browse, so the application rebuilds this result list when a filter changes and reuses it when drawing. That costs storage for IDs but avoids repeating the search on every redraw. The [usage guide](../UI_LAYOUT.md) gives the search controls and matching rules.

## A line of text has more than one length

The letter A uses one UTF-8 byte and usually one terminal column. The character 界 uses three UTF-8 bytes and commonly two columns. An accented letter can contain a base character and a combining mark, while a joined emoji may contain several Unicode characters.

These examples explain why counting bytes is not enough to wrap a header. The view needs to keep visible units together and estimate how many columns they occupy. A grapheme is a unit used for that grouping. Netscope walks through graphemes and uses a Unicode width library to count columns before wrapping the line.

Wrapping happens before choosing which lines to display. Otherwise, the scroll position could refer to the original lines while the screen showed a different number of wrapped lines. A long header remains available by scrolling rather than disappearing after its first screen-width portion.

Some captured characters are terminal commands rather than ordinary visible text. ESC can begin a sequence that changes colour or moves the cursor. We turn these controls into visible escaped text before drawing, so inspecting a header cannot make the terminal execute its contents.

Terminal fonts and Unicode versions can still differ. The tests establish how our chosen libraries lay out representative strings; they do not establish identical appearance in every terminal emulator.

## Repeat a small loop

The interactive program draws a frame, waits for input, changes the view state, and draws again. Several physical keys can become the same action: j and Down both mean move down. Keeping that conversion separate lets the navigation rules work with actions rather than every keyboard detail.

[Ratatui](../GLOSSARY.md#ratatui), the terminal drawing library, accepts a [closure](../GLOSSARY.md#closure) when it is time to draw. A closure is a function-like value written where it is used. It can borrow values from the surrounding code. Ours uses the session and the current view state to prepare one frame.

We can call the same drawing function with a fixed screen held in memory. A test can then check the visible text, the selected row's styling, and what appears at different sizes. No person has to press a key at exactly the right moment to test those rules.

## Leave the terminal usable

An interactive viewer changes how a terminal receives input so keys can be handled without waiting for Enter. It also uses an [alternate screen](../GLOSSARY.md#alternate-screen), which gives the viewer a display separate from the shell's usual contents. On exit, those settings need to be restored.

A [guard](../GLOSSARY.md#guard) is a value that takes responsibility for cleanup. Our terminal guard records the changes made during setup. Normal exit explicitly restores them. If the function returns early after an error, the guard's `Drop` implementation attempts cleanup when the guard leaves scope.

`Drop` is Rust's custom cleanup mechanism. It connects this terminal problem to the ownership rule we first met with `String`: a value can carry responsibility for releasing a resource. A terminal setting is a different resource from a text buffer, but both need a clear owner for cleanup.

Cleanup can also fail, so the explicit restoration path reports errors. A separate [pseudo-terminal](../GLOSSARY.md#pseudo-terminal) check exercises actual terminal settings after both quitting and an injected drawing failure. A pseudo-terminal behaves like a terminal connection under program control; it lets the test inspect mode changes without depending on a person's terminal window.

## Try returning to the same request

Use the supplied capture with several entries, select source ID 2, open it, choose All with 6, and scroll. Before pressing Esc, predict which row should remain selected. Then shrink and enlarge the terminal. Explain which values need to change and which recorded data can stay as it is.

Finally, explain why drawing takes a shared reference to the session but a mutable reference to view state. You should be able to describe the permissions without listing the fields of either struct.

## Follow the implementation when ready

The view code [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66) contains state changes, and the workspace renderer [`sections`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/workspace_view.rs#L133) composes the list and sections. Their tests [`navigation_retains_overview_selection_and_scroll`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/tests/views.rs#L333) exercise navigation and fixed screen buffers. The terminal code [`run_app`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L158) manages input and cleanup, with a separate restoration check [`check`](https://github.com/sidsharma96/netscope/blob/main/scripts/check_terminal.py#L20).

The [first version walkthrough](06-first-milestone.md) provides the command to run the sample. [Ratatui's documentation](https://docs.rs/ratatui/0.29.0/ratatui/) is the implementation reference for frames and drawing.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

Try these questions using the explanation above, then open the answers.

### Selection, scroll and focus

In a wide window the request list and its details are both visible. Pressing Down could select the next request or scroll the body. What decides which one happens?

<details>
<summary>Answer</summary>

Focus decides. Focus is the pane that receives movement keys, and selection is the request whose details are shown. They are separate values in the view state. Tab moves focus between the list and the details, and Enter focuses the selected request's details. The bright border marks focus and the filled row marks selection. Scroll position is a third value that records your place within the text.

**Learn more:** [Which pane should a key change?](../chapters/11-drawing-and-navigation.md#which-pane-should-a-key-change) · [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66)

</details>

### Leaving the terminal usable

Netscope switches the terminal into raw mode and an alternate screen. How does it restore the terminal if drawing fails partway through?

<details>
<summary>Answer</summary>

A guard value owns the cleanup. When setup changes the terminal, the guard records those changes. A normal exit restores them explicitly and reports any error. If the function returns early after an error, the guard's `Drop` implementation runs when it leaves scope and attempts the same restoration. A separate pseudo-terminal check runs the real program and inspects the terminal modes after it exits.

**Learn more:** [Leave the terminal usable](../chapters/11-drawing-and-navigation.md#leave-the-terminal-usable) · [Check the rule and the visible result](../chapters/04-quality-and-tests.md#check-the-rule-and-the-visible-result) · [`Restore`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L42)

</details>

<details>
<summary>Further questions for later study</summary>

<a id="why-mut-is-needed"></a>

[Why `mut` is needed](../questions/newcomer.md#why-mut-is-needed)

<a id="why-the-view-stores-request-ids"></a>

[Why the view stores request IDs](../questions/newcomer.md#why-the-view-stores-request-ids)

<a id="searching-for-a-url"></a>

[Searching for a URL](../questions/newcomer.md#searching-for-a-url)

<a id="bytes-characters-and-terminal-columns"></a>

[Bytes, characters and terminal columns](../questions/newcomer.md#bytes-characters-and-terminal-columns)

<a id="hostile-text-and-the-terminal"></a>

[Hostile text and the terminal](../questions/expert.md#hostile-text-and-the-terminal)

<a id="searching-for-api"></a>

[Searching for API](../questions/expert.md#searching-for-api)

</details>
