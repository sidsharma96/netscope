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

The optional journey adds explanations without replacing the recorded values. Its arrows illustrate HTTP direction, and its stage descriptions explain what a reported measurement means. They do not recover handshake messages missing from HAR. Keeping that learning view separate leaves routine inspection compact while making an explanation available when a field is unfamiliar.

Suppose a capture reports a 70 ms total, a 5 ms wait, and no duration for finding an address or connecting. The journey keeps those values together. It shows the known-stage sum beside the total, but does not turn their 65 ms difference into a new DNS or connection measurement. Subtraction can expose a discrepancy; it cannot explain its cause. The bars have a labelled duration scale, so a small known interval does not appear to account for the whole reported total.

Selecting Wait changes the explanation beside the stage list. Page Down can move through that explanation while Wait stays selected. Selecting another stage begins its explanation at the top. This is the same distinction between selection and scroll position that we needed for requests: one value identifies what we are reading, while another records our place within its text. Keeping them separate lets a smaller window require more scrolling without changing the selected measurement.

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

The [first version walkthrough](06-first-milestone.md) provides the command to run the sample. The [verification record](../VERIFICATION.md) describes completed checks and their limits. [Ratatui's documentation](https://docs.rs/ratatui/0.29.0/ratatui/) is the implementation reference for frames and drawing.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Why `mut` is needed

**Level:** Newcomer


Why does `let selected = 0_usize; selected += 1;` fail, and what does adding `mut` tell a reader?

<details>
<summary>Answer</summary>

Rust variables cannot be changed unless they are declared with `mut`. Writing `let mut selected` allows `+=` to store a new value. The keyword also tells the reader that this value is meant to change. In Netscope that separates values that describe the capture, which stay fixed, from view state such as the selected row, which changes as you press keys.

**Learn more:** [Make changes explicit](../chapters/07-values-and-functions.md#make-changes-explicit) · [Separate the capture from your place in it](../chapters/11-drawing-and-navigation.md#separate-the-capture-from-your-place-in-it)

</details>

### Hostile text and the terminal

**Level:** Expert


Captured URLs, headers, and filenames are controlled by whoever made the file. How does Netscope keep them from sending escape sequences to the terminal, what does it miss, and how is the terminal restored after an error or a panic?

<details>
<summary>Answer</summary>

`terminal_text` replaces every character for which `char::is_control` is true with its escaped form, so ESC becomes `\u{1b}`. The views and the non-interactive summary pass captured strings and filenames through it. Binary bodies are shown as hex, and previews are cut on character boundaries.

`is_control` covers only Unicode category Cc: U+0000 to U+001F and U+007F to U+009F. That includes ESC, so a captured value cannot start a terminal command. Format characters in category Cf pass through unchanged. The right-to-left override U+202E, the bidi isolates, and the zero-width space can make a URL display differently from its bytes.

The `Restore` guard enables raw mode and marks the alternate screen as entered before writing the escape that enters it, so a partial failure still triggers cleanup. Its `restore` runs on the normal path, and its `Drop` runs on an error return or during a panic unwind. A panic hook on the interface thread leaves the alternate screen and disables raw mode before the panic message is printed, so the message stays visible on the normal screen.

**Common wrong answer.** "All dangerous Unicode is escaped." Only Cc controls are; bidi and zero-width characters are a listed gap.

**Learn more:** [Hostile input files](../chapters/34-defending-the-design.md#hostile-input-files) · [Leave the terminal usable](../chapters/11-drawing-and-navigation.md#leave-the-terminal-usable) · [`terminal_text`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L285-L296) · [`Restore`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/terminal.rs#L42-L82)

</details>

### Why the view stores request IDs

**Level:** Newcomer


The screen must remember which request you selected across many redraws. Why does Netscope store a request ID for this instead of a reference to the request?

<details>
<summary>Answer</summary>

A reference is suited to short access, such as reading during one draw. Remembering a selection lasts across many key presses and redraws. An ID is a plain number the view state can own, and the program looks the request up in the session whenever it needs it. The lookup returns an `Option`, so an ID with no matching request becomes a visible absence instead of a crash.

**Learn more:** [Why store IDs?](../chapters/03-storing-requests.md#why-store-ids) · [`Session::request`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/session.rs#L264)

</details>

### Searching for API

**Level:** Expert


A user presses `/` and searches for `API` to find `https://example.com/api/books`. Will it match? What does the search look at, when does it run, and how does it combine with the host filter?

<details>
<summary>Answer</summary>

It will not match. The search is a case-sensitive substring test, `str::contains`, on the raw recorded URL string. It does not look at headers or bodies, and it does no normalisation or pattern matching.

The query is applied when Enter is pressed, not on each keystroke. The query is limited to 1024 bytes. It filters the current host filter's chronological list, or every request when no host filter is set. The matches are rebuilt as a list of request IDs, and the previously selected request stays selected if it still matches. Otherwise the first row is selected. Pressing `a` clears both the query and the host filter.

Each Enter costs O(n × m) for n requests and a query of length m. The captured requests are never moved or copied; only the list of matching IDs changes.

**Common wrong answer.** "The search is case-insensitive and also looks at headers." It is neither.

**Learn more:** [Find a request without changing the capture](../chapters/11-drawing-and-navigation.md#find-a-request-without-changing-the-capture) · [URL search](../contracts/m4-url-search.md) · [`rebuild_search`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L164-L177)

</details>

### Selection, scroll and focus

**Level:** Newcomer


In a wide window the request list and its details are both visible. Pressing Down could select the next request or scroll the body. What decides which one happens?

<details>
<summary>Answer</summary>

Focus decides. Focus is the pane that receives movement keys, and selection is the request whose details are shown. They are separate values in the view state. Tab moves focus between the list and the details, and Enter focuses the selected request's details. The bright border marks focus and the filled row marks selection. Scroll position is a third value that records your place within the text.

**Learn more:** [Which pane should a key change?](../chapters/11-drawing-and-navigation.md#which-pane-should-a-key-change) · [`App`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/view.rs#L66)

</details>
