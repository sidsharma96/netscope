# What a capture can tell us

Our request for a list of books has a reported total duration of 45 milliseconds. Before drawing a timeline, we need to decide what that number describes and what the other timing fields mean. A diagram can look precise even when the recording is incomplete.

A millisecond is one thousandth of a second. In a HAR, the total duration and the durations of individual stages are values reported by the tool that made the recording. We retain those values, including disagreements between them.

## Missing time is different from zero

Suppose the DNS timing is zero. DNS is the system used to look up an address for a hostname such as example.test. A recorded zero means the exporter reported zero at its measurement precision. It does not establish that no lookup work occurred.

Now suppose the DNS field is absent. The export gives us no timing in that field. HAR also permits the number `-1` in an optional timing. The [HAR 1.2 specification](https://raw.githubusercontent.com/ahmadnassri/har-spec/master/versions/1.2.md) defines it for `blocked`, `dns`, `connect`, and `ssl` as "the timing does not apply to the current request". The usual case is a request sent on a connection the browser had already opened: there was no DNS lookup and no connection setup for that request, so the exporter writes `-1` for both. Absence and `-1` therefore say different things. Absence says nothing at all. `-1` says the stage did not happen for this request. Neither reports a duration of zero.

This matters on screen. A bar with zero length suggests a known duration. A label such as `Not recorded` tells the reader that the duration is unknown, and `Not applicable (-1)` tells the reader that the exporter says the stage did not happen for this request. We need to preserve that distinction before choosing how to draw it.

Later, [missing values and errors](09-options-results-and-matching.md) shows how Rust gives these states separate names. For now, the important question is what each observation allows us to say.

## Some timings contain other timings

The sample reports 30 ms for connecting and 20 ms for TLS within that connection. TLS establishes the protected connection used by HTTPS. HAR includes its duration inside the connection duration.

Think of a 30 ms interval with a 20 ms portion inside it. Adding 30 and 20 would count that portion twice. The inspector therefore shows TLS as a detail of connecting. HAR calls this field `ssl`, so you will also see that name in the file and interface.

The waiting duration needs care too. It includes the time between sending the request and starting to receive its response. That interval can contain network delay and server work. A capture does not separate those causes merely by reporting one waiting value.

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
