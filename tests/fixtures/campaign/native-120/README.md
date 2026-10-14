# Native 120-scenario evidence

Final controlled campaign recorded October 13, 2026 local time / October 14 UTC. Payloads are synthetic; network exchanges, timestamps and native exports are actual observations. Chrome 155.0.8059.40 and Firefox 146.0.1 used disposable profiles. The [campaign report](../../../../docs/COMPATIBILITY_CAMPAIGN.md) gives coverage, failed attempts, fixes, discrepancies, trust exceptions and reproduction.

- cases.json: 120 scenario definitions and deterministic decoded-payload hashes; not recorded timings.
- chrome-native.har: native bundled DevTools HAR writer output, unchanged.
- firefox-native.har: bundled Netmonitor HAR export, JSON pretty-printed without changing values.
- browser observations: actual fetch hashes/statuses, EventSource dispatch and WebSocket events; these do not become HAR metadata.
- server journals: received request methods/paths/header/body observations and configured response status/protocol; no implication that every planned response byte reached a client.
- browser versions/provenance: actual versions, capture interval and scoped TLS exceptions.
- results.json: 240 native-capture checks and 100 production checks, including 94 network operations and six exclusions. Known discrepancies and limits are explicit.
- SHA256SUMS: hashes of every retained evidence file above.

No private TLS key, certificate database, profile or normal-user traffic is included. Native files are never edited to fit the importer. CI rechecks existing evidence without sockets/browsers and does not overwrite these files. New recordings belong in a new directory and require an evidence review before replacing this corpus.
