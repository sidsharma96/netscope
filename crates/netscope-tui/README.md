# Netscope

A terminal inspector for HAR and classic PCAP captures, with reviewed HTTP replay and beginner-friendly explanations.

```sh
cargo install netscope --locked
netscope capture.har
netscope --pcap capture.pcap
```

Requires Rust 1.92 or newer to install and a terminal at least 80×24. macOS and Linux are tested; Windows interactive behavior is not verified. Opening a capture never sends traffic. Replay requires a separate review and explicit send.

Browse request headers, bodies and timings; inspect captured SSE and WebSocket upgrades; explore TCP gaps and conflicting bytes. Replay supports bounded GET/POST over HTTP/1.1 and separately reviewed live SSE. PCAPNG, encrypted application decoding and proxy interception are outside scope.

The [repository](https://github.com/sidsharma96/netscope) contains the source and sample captures. Registry archives include the sources needed to build the crates, but not the repository's complete fixture suite.

Licensed under MIT OR Apache-2.0, at your option.
