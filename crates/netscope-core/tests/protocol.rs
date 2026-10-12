//! Exact recognition must leave unfamiliar exporter labels unclassified.
use netscope_core::RecordedProtocol;

#[test]
fn recognizes_only_explicit_spellings() {
    for (labels, expected) in [
        (&["HTTP/1.0"][..], RecordedProtocol::Http10),
        (&["HTTP/1.1"][..], RecordedProtocol::Http11),
        (
            &["HTTP/2", "HTTP/2.0", "http/2.0", "h2"][..],
            RecordedProtocol::Http2,
        ),
        (&["HTTP/3", "HTTP/3.0", "h3"][..], RecordedProtocol::Http3),
    ] {
        for label in labels {
            assert_eq!(RecordedProtocol::from_label(label), expected);
        }
    }
    for label in [
        "",
        "http/1.1",
        "H2",
        "http/2",
        "http/2.0 ",
        "http/20.0",
        "HTTP/2 ",
        " HTTP/3",
        "HTTP/20",
        "h3-29",
        "HTTP/4",
        "HTTP/1.1\n",
    ] {
        assert_eq!(
            RecordedProtocol::from_label(label),
            RecordedProtocol::Unrecognized
        );
    }
}
