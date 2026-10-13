# Keeping your place on screen

You select the second request, open its details, and scroll to the response body. When you press Esc, you expect to return to the same request in the list. If the window becomes smaller, you still expect the program to remember what you were inspecting.

Those expectations tell us what the interface needs to remember. The capture itself stays the same while your place within it changes.

## Separate the capture from your place in it

The session owns the recorded requests. A smaller set of values records the selected row, the active screen, and the scroll position. Together, these values are the view state: the information needed to decide what to show next.

Opening the inspector changes that state to identify one request. Returning to the overview restores its saved selection and scroll position. We do not need another copy of the captured body to remember either screen.

<!-- view-flow -->

When the capture is empty, there is no selected request. Rust's `Option` represents that absence. Pressing Enter can therefore leave the empty view in place instead of attempting to look up a nonexistent row.

## Describe the screen with an enum

When browsing requests, we switch between the overview and an inspector for a particular request. Those alternatives fit an enum. In Netscope, they are named `Overview` and `Inspector(RequestId)`.

The host view introduced in the next chapter adds another screen variant using the same idea.

The parentheses mean that the inspector variant carries a request ID. This keeps the fact that we are inspecting a request together with the identity of that request. A separate true-or-false flag would still need another value to say which request it meant.

The same key can have different meanings in those two states. Down selects the next request in the overview and scrolls one line in the inspector. A `match` on the current screen and action makes that decision explicit. The result is another state we can inspect in a test.

## Borrow data, change selection

Drawing needs read access to the capture. Navigation needs to change the view state. Rust distinguishes those permissions in the references passed to a function.

`&Session` is a shared reference. It lets the drawing code inspect the session while its owner keeps the data alive. `&mut App` is a mutable reference to the value holding view state. It permits updates, such as adjusting a scroll offset after a resize.

Here, `mut` follows the ampersand because it describes the reference's permission. This is related to, but distinct from, writing `let mut selected`: that earlier example made a local variable changeable. A mutable reference lends change access to an existing value.

Rust checks that this mutable access is exclusive while it is in use. The caller cannot also use a conflicting reference to the same state during that borrow. After drawing returns, the event loop can borrow the state again to apply the next action.

The drawing function also receives a mutable frame. A frame represents the terminal cells being prepared for the current screen. Writing those cells changes the display, while the borrowed capture remains intact.

## Move through a list that does not fit

Suppose the capture contains ten thousand requests and the window has room for fifteen rows. Pressing Down thousands of times would keep the rules simple but make the program tedious to use. Page Down moves the selection by the number of rows the current view can display. Home selects the first request; End selects the last. Page Up moves back toward the beginning.

Selection and scrolling still answer different questions. Selection identifies the request you want. The scroll offset identifies the first row visible in the window. After moving the selection, the program adjusts that offset only as far as needed to keep the selected row visible. Near the end, it also avoids leaving an unnecessary blank region below the last request.

A host filter changes the list these movements refer to. If a host has one hundred requests, End selects its hundredth request, even if the complete capture has thousands more. Returning to all requests restores the earlier position in that complete list. The selected row is a position within the current list; its request ID identifies the original captured entry. Keeping those two numbers separate prevents opening the wrong request after filtering.

The inspector uses the same keys to move through wrapped lines rather than requests. Its page size is different because the screen reserves a different amount of space for headings and controls. Resizing changes how many rows fit, so the next page movement uses the new size. An empty list has no selection to move, and a temporarily unusable window preserves the previous position.

There is also an arithmetic boundary to respect. Moving up fifteen rows from row three must stop at the beginning. Rust's unsigned integers represent zero and positive values; they cannot represent a negative row. Saturating subtraction stops at zero instead of producing a negative result. Movement toward the end is bounded by the list's last valid position. These rules make a page key predictable even when less than a full page remains.

## Find a request without changing the capture

Suppose a page loads a hundred requests and you want the ones whose URLs contain `/books`. A search should narrow what you see while leaving every recorded request available. If you open a result, its details must belong to that original request, even though it now occupies a different row.

In the request list, `/` opens a small text editor. Enter applies the text; Esc discards the edit. The search looks for an exact, case-sensitive substring anywhere in the captured URL, including its query string. A substring is a consecutive piece of text: `books` matches `/books?page=2`, but `Books` does not. Punctuation is ordinary text, so a dot means a dot rather than “any character.” Encoded text stays encoded: `%20` and a space are different searches.

A host filter and a URL search both have to match. The following illustration starts with three captured requests. The arrows mean “keep the IDs that pass this condition”; they do not move or remove the recorded requests.

```text
Captured URLs                            Original ID
https://a.test/books                     0
https://b.test/books                     1
https://a.test/cover                     2

All IDs [0, 1, 2]
       │ host is a.test
       ▼
       [0, 2]
       │ URL contains books
       ▼
       [0]
```

The result contains ID 0. Clearing just the search reveals IDs 0 and 2 again because the host condition still applies. Pressing `a` clears both conditions. The host summaries continue to describe the complete capture, so narrowing the request list does not silently change the meaning of those counts.

Remembering selection requires one more distinction. A row number describes a place in the current list; an ID describes a captured request. Before applying a query, the program remembers the selected ID. It then finds that ID's position in the new results. If it is absent, the first result becomes selected. If there are no results, selection becomes `None`, the absence introduced earlier, and Enter opens nothing. Clearing search recovers a usable list without inventing a request.

The application stores a vector of matching IDs, rather than copying request bodies or headers. It builds that vector when the applied query or host condition changes. Drawing can then read the same result list repeatedly. Searching every time the terminal redraws would save this extra vector, but would repeat a scan even when neither the capture nor the query had changed. The stored vector costs space for one ID per result and requires every filter change to rebuild it. That tradeoff fits a capture which stays fixed while you browse.

Edits remain separate from the applied query until Enter. You can type `q`, `h`, or `/` into the search without triggering their browsing shortcuts. Left and Right move between complete displayed characters; Home and End move to either end; Ctrl+u clears the edit. The editor accepts up to 1024 UTF-8 bytes. This is a bound on stored text, so a character that uses several bytes consumes several bytes of that allowance.

Try selecting a request whose URL contains `books`, then apply that search. Predict its new row and original ID before opening it. Next apply a word absent from the capture. Explain why the empty result can be drawn safely and why clearing it does not require importing the file again.

## A line of text has more than one length

The letter A uses one UTF-8 byte and usually one terminal column. The character 界 uses three UTF-8 bytes and commonly two columns. An accented letter can contain a base character and a combining mark, while a joined emoji may contain several Unicode characters.

These examples explain why counting bytes is not enough to wrap a header. The view needs to keep visible units together and estimate how many columns they occupy. A grapheme is a unit used for that grouping. Netscope walks through graphemes and uses a Unicode width library to count columns before wrapping the line.

Wrapping happens before choosing which lines to display. Otherwise, the scroll position could refer to the original lines while the screen showed a different number of wrapped lines. A long header remains available by scrolling rather than disappearing after its first screen-width portion.

Some captured characters are terminal commands rather than ordinary visible text. ESC can begin a sequence that changes colour or moves the cursor. We turn these controls into visible escaped text before drawing, so inspecting a header cannot make the terminal execute its contents.

Terminal fonts and Unicode versions can still differ. The tests establish how our chosen libraries lay out representative strings; they do not establish identical appearance in every terminal emulator.

## Repeat a small loop

The interactive program draws a frame, waits for input, changes the view state, and draws again. Several physical keys can become the same action: j and Down both mean move down. Keeping that conversion separate lets the navigation rules work with actions rather than every keyboard detail.

Ratatui, the terminal drawing library, accepts a closure when it is time to draw. A closure is a function-like value written where it is used. It can borrow values from the surrounding code. Ours uses the session and the current view state to prepare one frame.

We can call the same drawing function with a fixed screen held in memory. A test can then check the visible text, the selected row's styling, and what appears at different sizes. No person has to press a key at exactly the right moment to test those rules.

## Leave the terminal usable

An interactive viewer changes how a terminal receives input so keys can be handled without waiting for Enter. It also uses an alternate screen, which gives the viewer a display separate from the shell's usual contents. On exit, those settings need to be restored.

A guard is a value that takes responsibility for cleanup. Our terminal guard records the changes made during setup. Normal exit explicitly restores them. If the function returns early after an error, the guard's `Drop` implementation attempts cleanup when the guard leaves scope.

`Drop` is Rust's custom cleanup mechanism. It connects this terminal problem to the ownership rule we first met with `String`: a value can carry responsibility for releasing a resource. A terminal setting is a different resource from a text buffer, but both need a clear owner for cleanup.

Cleanup can also fail, so the explicit restoration path reports errors. A separate pseudo-terminal check exercises actual terminal settings after both quitting and an injected drawing failure. A pseudo-terminal behaves like a terminal connection under program control; it lets the test inspect mode changes without depending on a person's terminal window.

## Try returning to the same request

Use the supplied capture with several entries, select source ID 2, open it, and scroll. Before pressing Esc, predict which row should remain selected. Then shrink and enlarge the terminal. Explain which values need to change and which recorded data can stay as it is.

Finally, explain why drawing takes a shared reference to the session but a mutable reference to view state. You should be able to describe the permissions without listing the fields of either struct.

## Follow the implementation when ready

The [view code](../../crates/netscope-tui/src/view.rs) contains state changes and drawing. Its [tests](../../crates/netscope-tui/tests/views.rs) exercise navigation and fixed screen buffers. The [terminal code](../../crates/netscope-tui/src/terminal.rs) manages input and cleanup, with a separate [restoration check](../../scripts/check_terminal.py).

The [first version walkthrough](06-first-milestone.md) provides the command to run the sample. The [verification record](../VERIFICATION.md) describes completed checks and their limits. [Ratatui's documentation](https://docs.rs/ratatui/0.29.0/ratatui/) is the implementation reference for frames and drawing.
