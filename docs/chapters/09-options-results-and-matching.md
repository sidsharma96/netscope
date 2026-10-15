# Missing values and errors

The DNS timing in our capture might be absent, unavailable, zero, or a positive duration. It might also contain an invalid value. Before using the number, we need to answer two questions: was a value supplied, and can we accept it?

Rust gives these questions different types. Keeping them separate helps us avoid turning an invalid or missing measurement into a convincing number on screen.

## Represent the possibility of absence

`Option` describes a value that may be absent. `Option<f64>` can be `Some` containing a number or `None` containing no value. The angle brackets identify the contained type. `f64` is Rust's 64-bit floating-point type, which can represent fractional durations such as 30.125 milliseconds.

An option is an enum. An enum defines named alternatives, called variants, and a variant can carry data. `Some(30.125)` carries a number; `None` does not carry a hidden zero.

This is also the type used by a checked vector lookup. A successful lookup returns `Some` containing a reference to the element. An out-of-range lookup returns `None`. The same idea describes absence in different situations without pretending that a value exists.

## A present number still needs a meaning

`Some(-1.0)` tells us that a number was supplied. It does not explain the special meaning HAR assigns to that number. For an optional timing, HAR 1.2 says `-1` means the stage does not apply to the current request, for example DNS on a reused connection.

Three variants of our timing type describe standard HAR values. `Missing` means the field was omitted. `Unavailable` means the exporter supplied `-1`. The variant name is older than this explanation: HAR's meaning for a timing is "does not apply", and the interface shows it as `Not applicable (-1)`. For sizes, HAR uses the same `-1` to mean "not available", and there the name fits. `Milliseconds` contains an accepted duration, including zero.

These names make later decisions clearer. Drawing a duration and displaying a not-applicable label are different operations, and the stored value tells us which one applies. The value `-2` fits neither case, so validation rejects it.

A recorded number can also contradict its supposed meaning. In a native browser export, a blocked timing just below zero is not elapsed time travelling backward. Netscope retains that narrow exporter case separately, with a warning, instead of clamping it to zero or pretending it was the standard -1 sentinel. Its fourth variant, `ExporterNegative`, preserves the number without making it a measured duration. The standard conversion below remains strict; the [compatibility reference](../HAR_COMPATIBILITY.md) records the importer exception and its evidence.

## Give failure a value too

`Result` describes an operation that can succeed or fail. `Ok` carries the successful value; `Err` carries information about the failure. Both alternatives are ordinary values the caller can inspect.

The timing conversion returns `Result<Timing, InvalidTiming>`. The first type inside the brackets describes success, and the second describes failure. Here are checks against the implemented conversion:

```rust
use netscope_core::{InvalidTiming, Timing};
assert_eq!(Timing::from_optional_har(None), Ok(Timing::Missing));
assert_eq!(Timing::from_optional_har(Some(-1.0)), Ok(Timing::Unavailable));
assert_eq!(Timing::from_optional_har(Some(0.0)), Ok(Timing::Milliseconds(0.0)));
assert_eq!(Timing::from_optional_har(Some(-2.0)), Err(InvalidTiming));
```

The `use` line brings two names from the core library into this example. The braces group the names we want. In `Timing::from_optional_har`, the double colon selects a function associated with `Timing`. The same notation selects its named variants, such as `Timing::Missing`.

Read the first assertion in two steps. We supply no numeric value, represented by `None`. Conversion succeeds, represented by `Ok`, and the successful result records that the timing is missing. Missing data can therefore be a valid outcome. An invalid negative duration is a different outcome, represented by `Err`.

## Handle the alternatives with match

Once a value has named alternatives, we need a way to choose what to do for each one. A `match` expression compares a value against patterns:

```rust
use netscope_core::Timing;
let timing = Timing::Missing;
let label = match timing {
    Timing::Missing => "Not recorded",
    Timing::Unavailable => "Not applicable",
    Timing::Milliseconds(_) => "Duration recorded",
    Timing::ExporterNegative(_) => "Nonstandard exporter observation",
};
assert_eq!(label, "Not recorded");
```

Each line inside the braces is a match arm. The part before `=>` is its pattern; the part after it is the value to produce when that pattern matches. The underscore ignores the number inside `Milliseconds` because this example needs only a label.

Rust requires the match to cover every possible variant. That is useful when different stored states require different displays. If we later add another variant, a match listing each existing variant will need an explicit decision about the new case.

The timing conversion uses the same mechanism to inspect its input. It also checks that a supplied duration is finite and non-negative. Floating-point types have special values for infinity and "not a number," often called NaN, so accepting any `f64` would be too broad.

## Stop when the next step cannot proceed

Reading a request requires several successful checks. If a required field is invalid, there is no complete request to return. Writing out a full match for every check would repeat the same error-handling pattern.

The question-mark operator, `?`, expresses that pattern:

```rust
use netscope_core::{InvalidTiming, Timing};
fn optional_dns(value: Option<f64>) -> Result<Timing, InvalidTiming> {
    let timing = Timing::from_optional_har(value)?;
    Ok(timing)
}
assert_eq!(optional_dns(Some(-2.0)), Err(InvalidTiming));
```

When conversion returns `Ok`, `?` extracts the timing and execution continues. When conversion returns `Err`, `?` returns that error from `optional_dns` immediately. The surrounding function must have a compatible error return type.

This small helper exists to make that behaviour visible. The importer uses it in a longer sequence of field checks. Returning an error lets the caller explain the failed field to the user. Calling `unwrap` would instead panic when the result is `Err`, which is unsuitable for an expected input failure.

## Know where validation applies

The conversion above validates optional numeric timings. The importer must also check JSON types and enforce stricter rules for required durations. For example, a required send duration cannot be absent or `-1`.

Rust's types enforce the distinctions we have represented, but they do not invent rules we have left out. The public timing variants allow other Rust code to construct values directly. The current guarantee for capture data comes from passing it through the importer before use.

## Try the cases

Predict the conversion result for `None`, `Some(-1.0)`, `Some(-0.001)`, `Some(0.0)`, and `Some(30.125)`. Explain which cases are valid missing information and which are invalid input.

Then explain why accepting an optional DNS timing does not prove that the same value is valid for a required send duration.

## Where this appears in Netscope

Read the timing conversion [`Timing::from_optional_har`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L52) and its numeric tests [`absence_unavailability_and_zero_remain_distinct`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/tests/timing.rs#L5) after working through the cases. The importer tests [`optional_stages_preserve_each_state_and_cookie_types_are_checked`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/tests/har.rs#L323) cover the additional JSON rules. [Recoverable errors with Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html) provides more language examples.
