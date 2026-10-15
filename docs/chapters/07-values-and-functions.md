# Values, names, and functions

Our capture contains the method GET and the status 200. To work with those observations, a program needs to store text and numbers, give them names, and perform small operations on them. This chapter introduces enough Rust to read those operations.

The examples are deliberately smaller than the application. Each one isolates an idea you can predict and check before combining it with other ideas.

## Give a value a name

A variable gives us a name to use for a value. Rust introduces a variable with `let`. Here we name a number and a piece of text:

```rust
let status: u16 = 200;
let method: &str = "GET";
assert_eq!(status, 200);
assert_eq!(method, "GET");
```

Read the first line as "let status be a u16 with the initial value 200." The colon introduces the type, which describes the kind of value allowed here. The equals sign supplies the value. The semicolon ends the statement.

`u16` stores an unsigned whole number using 16 bits. Unsigned means it cannot be negative. A bit can be zero or one; 16 bits can represent 65,536 different values, so this type holds numbers from 0 through 65,535. That is enough room for an HTTP status, although many numbers in that range are not valid HTTP statuses. Choosing a type and checking a captured value are separate tasks.

The second line stores a reference to text. Rust writes this type as `&str`: `str` describes [UTF-8](../GLOSSARY.md#utf-8) text, and `&` means a reference to it. The quoted text is part of the program, so this example can refer to its existing bytes. The next chapter explains references alongside `String`, the type we use when a value needs to own its text storage.

The last two lines check our expectations. `assert_eq!` checks that its arguments are equal and stops the example with a failure if they differ. The exclamation mark identifies a macro, a construct that produces Rust code from its input. For now, use these assertions to state what an example should produce. We will return to macros when we need to understand more about how they work.

## A type can come from context

You can also write `let status = 200;`. Rust can often work out a value's type from the operations around it. This is called type inference. When nothing else constrains a whole-number literal, Rust uses `i32`, a signed 32-bit integer type.

An explicit type can make an example clearer, but it does not convert the value. Writing `let status: u16 = "200";` asks Rust to place text where a number is required. The compiler rejects it. Turning text into a number requires a parsing operation, which can fail if the text is unsuitable.

That distinction also appears when reading a capture. The JSON number 200 and the JSON string "200" have different types even though they look similar on a screen.

## Make changes explicit

Selecting the next request changes a position in the list. Rust asks us to mark a variable as changeable with `mut`:

```rust
let mut selected = 0_usize;
selected += 1;
assert_eq!(selected, 1);
```

The suffix in `0_usize` chooses the integer type `usize`. Rust uses this type for collection sizes and positions. Its size depends on the target platform. The `+=` operation adds one and stores the result back in `selected`.

Without `mut`, Rust rejects that change. This gives the reader of the program a useful distinction: some values describe the capture, while others track choices the user can change.

Permission to change a number does not prove the new number identifies a request. A list may be empty. The collection chapter explains how we check a position before using it.

## Put one decision in a function

HAR uses status zero to mean that no HTTP status was recorded. We can put the corresponding question into a function:

```rust
fn has_http_status(status: u16) -> bool {
    status != 0
}
assert!(has_http_status(200));
assert!(!has_http_status(0));
```

`fn` introduces a function. The parentheses describe its input: one value named `status`, of type `u16`. The arrow introduces the output type. `bool` has two possible values, `true` and `false`.

Inside the braces, `!=` asks whether the status differs from zero. This last expression has no semicolon, so its value becomes the function's result. With a semicolon, that value would be discarded and the function would no longer provide the promised boolean result.

`assert!` checks a boolean condition. In the second assertion, the `!` directly before the function call means "not." Its position matters: after a macro name, `!` marks a macro call; before a boolean expression, it reverses true and false.

The function's question is deliberately narrow. A nonzero number is not automatically a valid HTTP status. We will validate the allowed range when accepting input.

## Names have a scope

The parameter `status` belongs to the function body. A caller passes in a value and receives the result; it cannot reach inside the function merely by using that parameter's name.

This region in which a name is available is its [scope](../GLOSSARY.md#scope). Braces often mark a scope in Rust. The ownership chapter adds the related question of when a value is cleaned up. Keeping names and values distinct will help: a name can stop being usable while its value has moved somewhere else.

## Try a different question

Write a function named `is_http_status` that returns true for numbers from 100 through 599. The comparison operators `>=` and `<=` mean "at least" and "at most"; `&&` joins two conditions that must both be true.

Predict the answers for 0, 99, 100, 599, and 600, then check them with assertions. Explain why the new name fits this question better than `has_http_status`.

## Running a small example

Put the statements you want to try inside `fn main() { ... }` in a scratch file named `example.rs`. `main` is the function a standalone Rust program begins with. Place any helper function beside it. The braces in this instruction stand for your actual statements, not the three dots.

Compile with `rustc example.rs`, then run `./example` on macOS or Linux. `rustc` is the Rust compiler. Successful assertions print nothing; a failed assertion reports which expectation failed. The examples in this chapter are also checked by the project's documentation tests.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Naming a value with a type

**Level:** Newcomer


What does `let status: u16 = 200;` say, and why does `let status: u16 = "200";` fail to compile?

<details>
<summary>Answer</summary>

`let` introduces a variable named `status`. The part after the colon is its type: `u16`, an unsigned 16-bit whole number that holds 0 through 65,535. The value after `=` is 200. The second line puts text where the type requires a number, and Rust does not convert between them silently. Turning text into a number needs a parsing step that can fail, and the compiler rejects the line instead.

**Learn more:** [Give a value a name](../chapters/07-values-and-functions.md#give-a-value-a-name) · [A type can come from context](../chapters/07-values-and-functions.md#a-type-can-come-from-context)

</details>

### A function's last expression

**Level:** Newcomer


In `fn has_http_status(status: u16) -> bool { status != 0 }`, where does the returned value come from, and what changes if you add a semicolon after `status != 0`?

<details>
<summary>Answer</summary>

The arrow says the function returns a `bool`. The last expression in the body, `status != 0`, has no semicolon, so its value becomes the result. Adding a semicolon turns it into a statement whose value is discarded. The body then produces no `bool`, and the compiler reports that the function does not return the promised type.

**Learn more:** [Put one decision in a function](../chapters/07-values-and-functions.md#put-one-decision-in-a-function)

</details>

### Why `mut` is needed

**Level:** Newcomer


Why does `let selected = 0_usize; selected += 1;` fail, and what does adding `mut` tell a reader?

<details>
<summary>Answer</summary>

Rust variables cannot be changed unless they are declared with `mut`. Writing `let mut selected` allows `+=` to store a new value. The keyword also tells the reader that this value is meant to change. In Netscope that separates values that describe the capture, which stay fixed, from view state such as the selected row, which changes as you press keys.

**Learn more:** [Make changes explicit](../chapters/07-values-and-functions.md#make-changes-explicit) · [Separate the capture from your place in it](../chapters/11-drawing-and-navigation.md#separate-the-capture-from-your-place-in-it)

</details>
