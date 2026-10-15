# 120-scenario native capture campaign

Executed October 13, 2026 local time (October 14 UTC), after the request for at least 100 varied runs. The retained campaign has **120 distinct scenarios, each executed in Chrome and Firefox: 240 browser runs**. It also ran **94 Netscope network operations** and checked six explicitly unsupported replay cases. Auxiliary page loads, CORS preflights, TLS trust priming and redirect destinations are not counted as additional scenarios.

These are actual loopback network exchanges and native browser HAR exports. Payloads, application responses and credentials are synthetic. They are not public-provider trials, throughput benchmarks, a complete GraphQL engine, or whole-browser/protocol conformance tests. Packet reassembly remains covered by its separate fixed PCAP suite; these HAR exports do not supply original packets or WebSocket frame history.

| Family | Distinct scenarios | Coverage |
| --- | ---: | --- |
| REST | 50 | GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS; success/error statuses; queries, JSON/form/text, binary/Unicode, gzip, chunking, cookie/header fields and redirect |
| GraphQL-shaped HTTP | 25 | Queries, variables, mutation/alias/fragment/directive payloads, null/partial/error results, operation selection, batching/persisted-query shapes, MIME and compression |
| SSE | 25 | Multiline/empty/Unicode data, types, ID inheritance/reset/NUL, retry settings, comments/unknown fields, LF/CRLF/CR/BOM, pending tail, exact/exceeded event/text limits, gzip |
| WebSocket | 20 | Text/binary/Unicode, fragments/ping, multiple messages, JSON/GraphQL transport messages, subprotocols, close/status/digest/header failures |

34 scenarios use TLS: 28 HTTPS scenarios and six classic WSS scenarios. The controlled server supports H2 and HTTP/1.1; server observations retain the negotiated protocol. This is connection/protocol evidence, not an inferred frame schedule. Chrome used an exception restricted to the generated test certificate's public-key fingerprint; Firefox used an explicitly verified acceptInsecureCerts capability in a disposable session. Keys/profiles were removed and normal profiles/system trust were unchanged. Netscope's production TLS client did not inherit those browser exceptions.

## What was checked

Each configured scenario must have a native HAR entry for its expected method. Import must preserve recorded statuses/versions/representations and expose missing evidence. Decoded response content is compared with controlled payloads, and browser byte hashes independently check the received decoded bytes. Known exporter-text differences are explicit expectations, not HAR repairs. Requests to GraphQL-shaped endpoints are real HTTP exchanges carrying the listed payloads; the server returns controlled results rather than executing a general query language.

WebSocket checks compare actual browser payload observations with text/binary/frame-fragment expectations and inspect recorded upgrade shape. Failed Chrome handshakes sometimes export status zero rather than the server's response status. That remains unknown in the imported capture; the server log is separate evidence. Digest checks and actual socket frames do not become Netscope capabilities merely because the harness observed them.

SSE checks compare retained-text interpretation with browser-dispatched data/type/ID, while separately recording known discrepancies and inspection budgets. Production live SSE reads real raw bytes and compares derived events with the Chrome dispatch reference, including Unicode. Observation counts/byte caps and monotonic ordering are checked without timing thresholds.

The 100 production checks comprise 64 complete operations, 26 expected certificate rejections, three capture-limit stops, one expected compressed-live-SSE rejection, and six pre-send exclusions. Thus 94 start network operations; only 68 proceed to HTTP, because certificate rejection happens before sending. GET/POST response statuses and retained wire bytes are checked, including gzip bytes that Netscope deliberately does not decompress. PUT/PATCH/DELETE/HEAD/OPTIONS and the captured form representation remain outside reviewed replay support. Browser inspection still covers them.

## Findings and fixes

1. **Negative blocked timing rejected a whole native Chrome HAR.** Some exports contain blocked values between -1 and zero. The importer now retains that narrow case as ExporterNegative, with its exact number and a warning, without treating it as duration or the -1 sentinel. DNS/connect/SSL and required stages remain strict; blocked values below -1 remain rejected. Rendering explicitly says “not a duration.” Original exports are unchanged.
2. **Failed WebSocket exports carried an empty server address.** Chrome status-zero ws/wss entries now treat that empty address as absent, as already done for 101 upgrades. Ordinary HTTP and other malformed addresses remain rejected. No response status or address is reconstructed from the server log.
3. **Text can differ from received bytes.** R25's unlabelled text/plain UTF-8 bytes yield different exported text in Chrome and Firefox. S03 also showed Chrome-exported mojibake despite the event-stream UTF-8 header, while browser dispatch and Netscope's live client saw the original Unicode. These are observed exporter differences in these versions, not reasons to guess a repair during import.
4. **Firefox dispatch differed on metadata-only ID and NUL-ID cases.** S15 dispatched an empty ID where retained-text interpretation carries a; S21 dispatched no event while the retained text produces one after ignoring the invalid ID field. These exact observations are retained, not a universal Firefox or whole-EventSource verdict. Netscope follows its documented retained-text rules rather than rewriting them to match every dispatch difference.
5. **An open notification does not settle negotiation validity.** W19 produced a Firefox open notification followed by close 1006 and no message for an unoffered subprotocol. Netscope flags the inconsistent selected protocol. Chrome records a failed attempt with status zero. The report does not claim a successful application conversation.
6. **Expected evidence limits remained visible.** Redirect bodies can be absent, browser and replay compression representations differ, and inspected SSE event/text limits intentionally differ from complete browser dispatch. Those are labelled outcomes rather than silently counted as identical results.

Early attempts exposed harness setup mistakes (Firefox's raw Marionette TLS capability shape and an example-checker compile error). They were corrected before the retained successful run. Initial exact-byte/dispatch comparisons also exposed the representation distinctions above; expectations were revised explicitly without editing native HAR bytes. Only the final selected campaign counts toward the stated 120/240 totals; retries are not extra distinct scenarios.

## Retained evidence and reproduction

[Case manifest](../tests/fixtures/campaign/native-120/cases.json), unchanged [Chrome HAR](../tests/fixtures/campaign/native-120/chrome-native.har), reformatted native [Firefox HAR](../tests/fixtures/campaign/native-120/firefox-native.har), browser observations, server request logs, versions, provenance, [results](../tests/fixtures/campaign/native-120/results.json), and SHA256SUMS are retained in the [campaign directory](../tests/fixtures/campaign/native-120/README.md). Firefox formatting preserves the native values; Chrome's native string is unchanged.

The normal check script rechecks the frozen corpus offline. No browser/service is started in CI. To record new traffic on macOS with installed Chrome/Firefox, Node 24, Python and OpenSSL:

```sh
python3 scripts/compatibility/run.py --output /tmp/netscope-new-campaign
```

The output directory must be empty. The runner starts only controlled loopback services, uses disposable profiles/keys, obtains HAR through bundled native exporters, runs production probes while services exist, then disposes them. Browser versions and dynamic timings/ports are new observations. Review new captures before replacing fixtures; do not normalize them to force a pass. The checker [`main`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/examples/check_campaign.rs#L24) also runs offline against the retained evidence; its live flag is intended for the managed recording run.
