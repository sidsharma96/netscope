# What does moving one character mean?

You reopen a saved request and notice a typo in the middle of its body. Moving Left to fix it seems simple: go back one character, remove the mistake, and type the correction. But the word “character” hides several different units. A byte belongs to storage. A Unicode value identifies a piece of text. What looks like one character on screen can contain more than one such value.

An editor needs to decide which unit a key operates on. Otherwise, correcting a title could split a letter from its accent or leave half of a joined emoji behind.

## One visible letter can have two parts

The previous text lesson used a precomposed é, stored as two UTF-8 bytes. Another spelling uses an ordinary e followed by a combining acute accent. Those are two Unicode scalar values: individual values that Rust's char type can represent. Together they form a grapheme cluster, a group of text elements treated as one unit for this editor's movement and deletion.

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

The escape `\u{301}` writes the combining accent by its Unicode value. It becomes part of the string rather than the literal backslash and digits. `chars()` returns an iterator, a sequence we can visit one item at a time; `count()` consumes that sequence and counts its items. The imported trait makes `graphemes()` available on the string slice. Its true argument selects extended grapheme rules.

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

The same movement works in URLs, header fields, saved names, and paths. The [keyboard contract](../contracts/m4-keyboard-editing.md) records precise key behaviour and limits. The [cursor operations](../../crates/netscope-tui/src/text_edit.rs) maintain text boundaries, while the [editor](../../crates/netscope-tui/src/replay_view.rs) translates them into a visible insertion point.
