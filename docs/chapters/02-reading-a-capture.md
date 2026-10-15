# What a capture can tell us

Our request for a list of books has a reported total duration of 45 milliseconds. Before drawing a timeline, we need to decide what that number describes and what the other timing fields mean. A diagram can look precise even when the recording is incomplete.

A millisecond is one thousandth of a second. In a HAR, the total duration and the durations of individual stages are values reported by the tool that made the recording. We retain those values, including disagreements between them.

## Missing time is different from zero

Suppose the [DNS](../GLOSSARY.md#dns) timing is zero. DNS is the system used to look up an address for a hostname such as example.test. A recorded zero means the [exporter](../GLOSSARY.md#exporter) reported zero at its measurement precision. It does not establish that no lookup work occurred.

Now suppose the DNS field is absent. The export gives us no timing in that field. HAR also permits the number `-1` in an optional timing. The [HAR 1.2 specification](https://raw.githubusercontent.com/ahmadnassri/har-spec/master/versions/1.2.md) defines it for `blocked`, `dns`, `connect`, and `ssl` as "the timing does not apply to the current request". The usual case is a request sent on a connection the browser had already opened: there was no DNS lookup and no connection setup for that request, so the exporter writes `-1` for both. Absence and `-1` therefore say different things. Absence says nothing at all. `-1` says the stage did not happen for this request. Neither reports a duration of zero.

This matters on screen. A bar with zero length suggests a known duration. A label such as `Not recorded` tells the reader that the duration is unknown, and `Not applicable (-1)` tells the reader that the exporter says the stage did not happen for this request. We need to preserve that distinction before choosing how to draw it.

Later, [missing values and errors](09-options-results-and-matching.md) shows how Rust gives these states separate names. For now, the important question is what each observation allows us to say.

## Some timings contain other timings

The sample reports 30 ms for connecting and 20 ms for [TLS](../GLOSSARY.md#tls) within that connection. TLS establishes the protected connection used by HTTPS. HAR includes its duration inside the connection duration.

Think of a 30 ms interval with a 20 ms portion inside it. Adding 30 and 20 would count that portion twice. The inspector therefore shows TLS as a detail of connecting. HAR calls this field `ssl`, so you will also see that name in the file and interface.

The waiting duration needs care too. It includes the time between sending the request and starting to receive its response. That interval can contain network delay and server work. A [capture](../GLOSSARY.md#capture) does not separate those causes merely by reporting one waiting value.

## An observation and an explanation need different labels

A recorded status of 200 is a fact present in the file. A calculated total is a derived value: we obtained it by applying a stated rule to recorded numbers. Both are different from an inference, which proposes an explanation that the recording does not establish on its own.

For example, two similar requests might suggest a retry. They could also be independent actions. Calling the second request a retry needs more evidence or a visible explanation of the assumption.

A teaching diagram has another purpose. It may illustrate the messages exchanged while opening a connection, even though this HAR contains no record of those individual messages. We label that diagram as an illustration. Information the recording does not provide stays unavailable.

These labels make the reasoning readable. The reader can tell which claims came from the file, which came from a calculation, and which need further evidence.

## An empty body is still information

A response body is the content returned with a response, such as our JSON list of books. A captured empty string means the export retained an empty text representation. An absent body field means it retained no body text.

Those cases may look identical if we immediately turn both into an empty string. Keeping them separate lets the inspector say either `Captured body is empty` or `Response body not recorded`. The distinction helps you investigate the recording without silently changing its meaning.

## Try the distinction

A capture reports 30 ms for connecting and 20 ms for TLS. Explain why those values do not describe 50 ms of connection setup. Then describe what you would display if the TLS timing were `-1`.

Finally, imagine that a response body is absent. State one fact you can report and one conclusion the absence does not support.

## Further reading

The [HAR 1.2 specification](https://raw.githubusercontent.com/ahmadnassri/har-spec/master/versions/1.2.md) describes the timing fields and their relationships. Netscope's [input contract](../contracts/m1-har-inspection.md) records the exact subset we accept. The contract is a reference for details; the distinctions above are the reason for those rules.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### What Netscope is for

**Level:** Newcomer


In one or two sentences, what does Netscope do, and what does it never do without an explicit action from you?

<details>
<summary>Answer</summary>

Netscope is a terminal program for inspecting recordings of network traffic. It reads HAR files exported by a browser and classic PCAP packet files, and shows what each recording contains, including what it leaves out. It can also send one new GET or POST request based on a recorded one, but only after you review a separate draft and press Send. Opening a capture or a saved draft never sends anything.

**Learn more:** [An observation and an explanation need different labels](../chapters/02-reading-a-capture.md#an-observation-and-an-explanation-need-different-labels) · [Make dependencies point toward the data rules](../chapters/23-shaping-the-program.md#make-dependencies-point-toward-the-data-rules)

</details>

### Why is a timing an enum?

**Level:** Practitioner


HAR timings are numbers. Why does Netscope model an optional timing as an enum instead of `Option<f64>`, or instead of storing 0 when the value is absent? Name the variants and say what each one means.

<details>
<summary>Answer</summary>

`Timing` has four variants. `Missing` means the exporter left the field out. `Unavailable` means the exporter wrote `-1`, which HAR 1.2 defines as "does not apply to this request", for example DNS on a reused connection. `Milliseconds(f64)` holds a finite, non-negative duration. `ExporterNegative(f64)` keeps a nonstandard negative value that some exporters write for `blocked`, without treating it as a duration.

Storing 0 for an absent value would invent a measurement: a reader could not tell "took no time" from "was not recorded". `Option<f64>` is better but still merges two different facts, an absent field and a field that does not apply. With four variants, every later calculation has to say what it does with each case, and the compiler checks that it does. The required `send`, `wait` and `receive` fields are plain `f64` values, because the importer rejects an entry that lacks them.

**Common wrong answer.** "-1 means the value is unknown." HAR uses -1 for a phase that did not happen for this request, which is a different claim from "unknown".

**Learn more:** [Missing time is different from zero](../chapters/02-reading-a-capture.md#missing-time-is-different-from-zero) · [Why keep -1, missing, zero and nonstandard values apart?](../chapters/34-defending-the-design.md#why-keep--1-missing-zero-and-nonstandard-values-apart) · [`Timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L11-L22) · [`from_optional_har`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L52-L61)

</details>

### What a HAR file is

**Level:** Newcomer


What is a HAR file, and who decides what goes into it?

<details>
<summary>Answer</summary>

A HAR (HTTP Archive) file is a JSON document that a browser or another capture tool exports. It lists HTTP requests and their responses, with headers, bodies when the tool kept them, and reported timings. The exporter decides what to record, so a HAR can omit a body or a timing. Netscope reads the file and keeps those gaps visible. It does not fill them in.

**Learn more:** [Reading structure comes before understanding meaning](../chapters/10-importing-a-har.md#reading-structure-comes-before-understanding-meaning) · [What a capture can tell us](../chapters/02-reading-a-capture.md#what-a-capture-can-tell-us)

</details>

### When do timing stages disagree with the total?

**Level:** Practitioner


How does the importer decide to warn "Timing stages differ from reported total"? Which fields are added, which are left out and why, what tolerance applies, and how is a `blocked` value between -1 and 0 treated?

<details>
<summary>Answer</summary>

The importer adds `send`, `wait` and `receive`, then adds `blocked`, `dns` and `connect` only when each one is a measured `Milliseconds` value. Missing, not-applicable and nonstandard values contribute nothing, so no duration is invented. `ssl` is left out because HAR defines the TLS time as part of `connect`, and adding it again would count it twice. If the sum is not finite, or differs from the entry's `time` by more than 0.01 ms, the request gets a warning, not an error.

A second check warns when `ssl` is a duration but `connect` is not a duration at least as large. A `blocked` value strictly between -1 and 0 becomes `ExporterNegative` with its own warning, because some exporters write small negative values there. Any other negative value apart from -1 is an import error, and only `blocked` gets this tolerance.

**Learn more:** [Some timings contain other timings](../chapters/02-reading-a-capture.md#some-timings-contain-other-timings) · [Timing rules](../contracts/m1-har-inspection.md#timing-rules) · [`optional_timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L352-L372) · [`sum`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-import/src/lib.rs#L507-L524)

</details>

### Missing, minus one, and zero

**Level:** Newcomer


A HAR's DNS timing can be absent, `-1`, or `0`. What does each one mean, and how does Netscope keep them apart?

<details>
<summary>Answer</summary>

An absent field means the exporter recorded nothing. HAR 1.2 defines `-1` for an optional timing as "does not apply to the current request", for example DNS on a connection the browser had already opened. A `0` is a reported duration of zero at the exporter's precision. Netscope stores these as different variants of its `Timing` enum: `Missing`, `Unavailable` (shown as `Not applicable (-1)`), and `Milliseconds(0.0)`. None of them is turned into another.

**Common wrong answer.** "`-1` means the value is unknown, so treat it like zero." It means the stage did not happen for this request, and adding it as zero would invent a measurement.

**Learn more:** [Missing time is different from zero](../chapters/02-reading-a-capture.md#missing-time-is-different-from-zero) · [A present number still needs a meaning](../chapters/09-options-results-and-matching.md#a-present-number-still-needs-a-meaning) · [`Timing`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-core/src/lib.rs#L11)

</details>
