# What does moving one character mean?

You reopen a saved request and notice a typo in the middle of its body. Moving Left to fix it seems simple: go back one character, remove the mistake, and type the correction. But the word “character” hides several different units. A byte belongs to storage. A Unicode value identifies a piece of text. What looks like one character on screen can contain more than one such value.

An editor needs to decide which unit a key operates on. Otherwise, correcting a title could split a letter from its accent or leave half of a joined emoji behind.

## One visible letter can have two parts

The previous text lesson used a precomposed é, stored as two [UTF-8](../GLOSSARY.md#utf-8) bytes. Another spelling uses an ordinary e followed by a combining acute accent. Those are two [Unicode scalar values](../GLOSSARY.md#unicode-scalar-value): individual values that Rust's char type can represent. Together they form a [grapheme cluster](../GLOSSARY.md#grapheme-cluster), a group of text elements treated as one unit for this editor's movement and deletion.

The drawing compares the spellings. The byte values are illustrative hexadecimal values, not a network measurement. The visual result depends on the font, but the stored sequences are different.

```text
Visible example         é                         é
Unicode values          one                       e + combining accent
UTF-8 bytes             c3 a9                     65 cc 81
Editor movement         one step                  one step
```

A one-byte step can land inside an encoded value. A one-scalar-value step can separate the base letter from its combining mark. Grapheme boundaries give the editor a more useful unit. The segmentation library exposes these groups and their byte positions through [UnicodeSegmentation](https://docs.rs/unicode-segmentation/1.12.0/unicode_segmentation/trait.UnicodeSegmentation.html).

## A byte position needs a rule

A cursor can be stored as a usize holding a byte position in a String. That number alone does not establish that the position is suitable for editing. The program must keep it at an allowed boundary.

Here is a small fragment using borrowed text; its statements can run inside main. It distinguishes a byte count, a scalar-value count, and a grapheme count without creating an editor.

```rust
use unicode_segmentation::UnicodeSegmentation;

let letter = "e\u{301}";

assert_eq!(letter.len(), 3);
assert_eq!(letter.chars().count(), 2);
assert_eq!(letter.graphemes(true).count(), 1);

assert_eq!(letter.get(0..1), Some("e"));
assert_eq!(letter.get(1..2), None);
```

The escape `\u{301}` writes the combining accent by its Unicode value. It becomes part of the string rather than the literal backslash and digits. `chars()` returns an [iterator](../GLOSSARY.md#iterator), a sequence we can visit one item at a time; `count()` consumes that sequence and counts its items. The imported [trait](../GLOSSARY.md#trait) makes `graphemes()` available on the string slice. Its true argument selects extended grapheme rules.

The range `0..1` includes byte zero and stops before byte one. It ends after the e, so `get` can return that valid UTF-8 substring, wrapped in Some. The range `1..2` ends inside the accent's encoding, so `get` returns None. Rust's [checked string access](https://doc.rust-lang.org/std/primitive.str.html#method.get) lets the caller handle such an invalid range without a panic.

Notice that valid UTF-8 is not the editor's complete rule. The first slice succeeds, yet it separates the base letter from its accent. Netscope therefore obtains cursor positions from grapheme boundaries, rather than assuming every valid Rust string boundary is a suitable place for Left or Right to stop.

## Editing can change the boundaries

Suppose two emoji are separate, with the cursor between them. Inserting a joining character can make the pair one grapheme. Leaving the old cursor position untouched would now place it inside that group.

Read this drawing as a change to the text, not a sequence of transmitted bytes. The vertical bar represents the intended insertion position and is not part of the stored body.

```text
Before insertion       woman | laptop        two separate emoji
Insert a joiner                ↓
After insertion        woman-joiner-laptop |  one joined group
```

The editor checks boundaries again after a mutation. If the intended byte position now lies inside a group, it advances to that group's end. Backspace can then remove the whole group. Deleting text can also join its neighbours, so the same check applies after deletion.

String insertion itself uses a byte position. It can move the following bytes to make room, and it requires a valid text boundary. The [String insertion documentation](https://doc.rust-lang.org/std/string/struct.String.html#method.insert) describes that requirement. Tracking a checked position is what lets the editor use that operation predictably.

This does not mean every edit takes constant work. Finding boundaries and shifting the remaining bytes both involve examining or moving text. The field's existing size limit bounds the amount of text involved; this editor does not claim the design of a large document editor.

## Storage positions are not screen columns

Even a correct byte position does not directly locate the terminal cursor. A CJK character may occupy two terminal columns. A combining mark may add no column. Netscope also displays control characters using visible escapes, so one stored newline appears as several printable symbols rather than moving the terminal to another line.

The display therefore takes the text before the insertion point, escapes it, and measures how that representation wraps in the available width. That produces a row and column. Making the terminal narrower changes those coordinates without changing where the cursor belongs in the String.

When the cursor moves out of view, the editor scrolls to keep it visible. At the end of an exactly full row, it reserves a blank display row for the cursor. The blank row is not a newline inserted into the request. This is another distinction between the text being edited and the screen used to show it.

Terminal emulators and fonts can differ in their rendering. The application uses one consistent width model for wrapping and cursor placement; it does not claim that every emoji will look identical everywhere.

## Make a correction without changing the operation

Open a body field and type a short title with an accented letter. Move Left and Right, then remove a group with Backspace or Delete. Use Home to reach the beginning and End to reach the end of the whole field. Predict whether each action changes the body, just the cursor, or neither because the cursor is already at a boundary.

Press Esc and reopen the field. The saved draft should still contain its previous body, because the abandoned edit was a separate value. Repeat the correction and press Enter to save it. You must still review and explicitly Send before any request leaves the program.

The same movement works in URLs, header fields, saved names, and paths. The [keyboard contract](../contracts/m4-keyboard-editing.md) records precise key behaviour and limits. The cursor operations [`Cursor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L5) maintain text boundaries, while the editor [`ReplayEditor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/replay_view.rs#L31) translates them into a visible insertion point.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Counting the bytes in café

**Level:** Newcomer


You POST the plain-text body `café`. What Content-Length does Netscope send, and why?

<details>
<summary>Answer</summary>

Five. Content-Length counts bytes, not letters. Rust stores text as UTF-8, where `c`, `a` and `f` each take one byte and the precomposed `é` takes two. Netscope computes the length from the reviewed body bytes and never reuses a captured Content-Length, since an edited body would make it wrong. A JSON body such as `{"title":"café"}` also counts its braces, quotes and colon.

**Common wrong answer.** "Four, one per character."

**Learn more:** [Text already has a byte representation](../chapters/19-submitting-text.md#text-already-has-a-byte-representation) · [A length describes the bytes we actually send](../chapters/19-submitting-text.md#a-length-describes-the-bytes-we-actually-send)

</details>

### Where does the cursor go after an edit?

**Level:** Practitioner


The replay editor stores its cursor as a byte position in a Rust `String`. Why is a byte position safe there, how do the arrow keys move it, and what happens after an insertion joins two characters into one?

<details>
<summary>Answer</summary>

A Rust `String` is UTF-8, so a character can take one to four bytes, and one visible character (a grapheme) can combine several code points, such as `e` followed by a combining accent, or an emoji sequence joined by zero-width joiners. The cursor stores a byte offset, but it only ever rests on a grapheme boundary. Left moves to the last boundary before the cursor, and right moves to the next boundary after it, using the `unicode-segmentation` crate's extended grapheme rules.

Insertion, backspace and delete can change where the boundaries are. Inserting a joiner between two emoji, for example, can merge them into one grapheme. After every edit, `settle` moves the cursor forward to the next valid boundary, so it never sits inside a character. Backspace removes the whole previous grapheme, not one byte. Insertion is refused if it would take the text past the 16 KiB body limit.

**Learn more:** [One visible letter can have two parts](../chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts) · [Editing can change the boundaries](../chapters/21-moving-through-text.md#editing-can-change-the-boundaries) · [`Cursor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L5-L26) · [`settle`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L27-L34)

</details>

### Moving the cursor by one character

**Level:** Newcomer


The text `é` can be stored as `e` followed by a combining accent. If you press Left once after it, where should the cursor go?

<details>
<summary>Answer</summary>

Before the `e`, past the whole visible letter. The two Unicode values form one grapheme cluster, a group treated as one unit for movement and deletion. A one-byte step could land inside an encoded character, and a one-`char` step would separate the letter from its accent. Netscope stores the cursor as a byte position but moves it only between grapheme boundaries, and checks the boundaries again after each edit.

**Learn more:** [One visible letter can have two parts](../chapters/21-moving-through-text.md#one-visible-letter-can-have-two-parts) · [A byte position needs a rule](../chapters/21-moving-through-text.md#a-byte-position-needs-a-rule) · [`Cursor`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/text_edit.rs#L5)

</details>

### Bytes, characters and terminal columns

**Level:** Newcomer


Why can't Netscope use a string's byte length to decide where to wrap a header on screen?

<details>
<summary>Answer</summary>

Bytes, visible characters and terminal columns are different lengths. `A` takes one byte and one column. `界` takes three bytes and usually two columns. A combining mark takes bytes but no column. Netscope groups text into graphemes and measures their width with a Unicode width library before wrapping. Control characters are first shown as visible escapes, which take several columns.

**Learn more:** [A line of text has more than one length](../chapters/11-drawing-and-navigation.md#a-line-of-text-has-more-than-one-length) · [Storage positions are not screen columns](../chapters/21-moving-through-text.md#storage-positions-are-not-screen-columns)

</details>
