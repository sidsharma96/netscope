# Native browser exports

Captured locally on October 11, 2026 (timestamps cross into October 12 UTC), from an isolated Python loopback HTTP server at port 57618. Request content is synthetic; timestamps and durations are actual exporter output. No normal user browser profile or third-party destination was used. The only cookie is `netscope_fixture=synthetic`; browser user-agent strings and local server metadata are intentionally retained.

| File | Origin and transformation |
| --- | --- |
| chrome-154.har | Chrome 154.0.8037.98; native HAR string saved unchanged |
| firefox-146.har | Firefox 146.0.1, build 20251217121356; native HAR JSON parsed and pretty-printed with Unicode characters preserved |
| firefox-146-in-progress.har | Same Firefox exporter invoked before its request collection had settled; JSON reformatted as above; 3 entries |

SHA256SUMS records the checked-in bytes. The Chrome HAR creator identifies WebInspector 537.36; that metadata is not the installed browser version. The complete files each have nine entries. The early Firefox file is an in-progress snapshot, not a deliberately failed network request.

## Capture method

Chrome ran headlessly with a fresh temporary profile and an ephemeral remote-debugging port. A bundled `devtools_app.html` frontend connected to the page target. Its Network panel and NetworkLog were initialized before navigation. After the controlled page completed, `Writer.harStringForRequests(NetworkLog.instance().requests(), {sanitize: false}, progress)` serialized the requests using Chrome's bundled HAR writer. Cookie inclusion was deliberate for the synthetic fixture.

Firefox ran with a fresh temporary profile and Marionette system access. A Netmonitor toolbox was opened before navigation. After the page completed, the driver waited for the Network Monitor request store to contain all nine entries, obtained its HAR export connector, and called the bundled `HarExporter.fetchHarData` with those items, `includeResponseBodies: true`, and `forceExport: true`. An earlier call, made before the store settled, produced the retained three-entry snapshot. Both browser processes were closed after capture.

These are native exporter outputs, not HAR files synthesized from browser protocol events. Temporary driver profiles are not repository fixtures. The [compatibility reference](https://netscope-docs.sidsharma.dev/HAR_COMPATIBILITY.html) describes the covered scenarios and manual reproduction with the maintained local server helper. The helper has the same request sequence and content; it does not reproduce the original page source formatting or measurement values byte for byte.

## Source cross-checks

Exporter behaviour was also inspected in [Chrome's HAR builder](https://github.com/ChromeDevTools/devtools-frontend/blob/42f8e5fc3590ecbbea0b5efeda97a78e11f48197/front_end/models/har/Log.ts) and [Firefox's HAR builder](https://github.com/mozilla-firefox/firefox/blob/8e9ac3e12bbaf39d4c282dc8849221aa17d4f714/devtools/client/netmonitor/src/har/har-builder.js). These pinned research snapshots are not claimed to be the exact source revisions bundled in the installed browsers. The recordings themselves supply the regression evidence.

Do not normalize these files to fit the importer. Add deliberate mutations inside tests, where the change and expected result remain visible. Tests use fixed bytes and values; recording new traffic is a separate exploratory activity.

## HTTPS/H2 extension

`chrome-154-h2.har` and `firefox-146-h2.har` were recorded later on October 11, 2026, with the same browser versions, new temporary profiles, and `scripts/har_https_fixture_server.mjs` running under Node 24.12.0 on loopback port 58714. Each has six entries. The Chrome native string is unchanged; Firefox JSON was parsed and pretty-printed with Unicode preserved. Both used the bundled exporter methods described above. No cookies or credentials are present in these two files.

The local certificate was self-signed and generated for this experiment. Chrome's temporary process used `--ignore-certificate-errors-spki-list` with that certificate's public-key fingerprint. Firefox's Marionette session used acceptInsecureCerts. The first Firefox attempt incorrectly nested the capability, left it false, and failed certificate validation; it produced no fixture. The successful session reported the capability true. These exceptions were limited to the disposable test browser processes; no system trust changes were made.

`https-server-observations.jsonl` is a separate server-side request log captured before the additional Node client validation. Session 1 handled Chrome and session 3 handled Firefox. Session 2 had no logged request during the failed Firefox attempt. Chrome also requested a DevTools discovery URL that its exported page log does not include. The observation file retains that server request. Session numbers are local evidence, not HAR connection identifiers, and the importer does not consume this log. The source helper generated the exact captured page and payloads.

The [compatibility reference](https://netscope-docs.sidsharma.dev/HAR_COMPATIBILITY.html) records compression sizes, timing differences, certificate limitations, and reproduction commands. SHA256SUMS includes both HAR files and the independent server observation file. Frozen timings are evidence of exporter behaviour, not performance baselines.

## SSE and WebSocket extension

`chrome-155-streams.har` and `firefox-146-streams.har` were recorded on October 13, 2026 from `scripts/stream_fixture_server.py` at loopback port 57244. Chrome was **155.0.8059.40**; Firefox remained **146.0.1**, build **20251217121356**. Each native export has three entries: page, finite SSE response, classic WebSocket upgrade. Fresh temporary profiles and the bundled native exporters described above were used. Chrome's native string is unchanged; Firefox JSON was pretty-printed with Unicode preserved. No cookies, credentials, external destinations or certificate exceptions were used.

The helper supplies a leading UTF-8 BOM and three events: added with multiline café/shelf 3 data and ID book-1, a default message inheriting that ID, and a done event whose data is [DONE]. Retry 1500 is a setting. Both exporters omit the leading BOM from retained content text (123 UTF-8 bytes versus 126 server body bytes); the three complete events remain. The helper splits writes, but HAR does not preserve per-event arrival times.

The server sends one synthetic WebSocket text frame after its valid 101 response. Neither HAR supplies usable frame history. Chrome reports bodySize -202, empty serverIPAddress, and no body text. Firefox reports bodySize 202 and empty body text. These recorded values are not payload/message counts. The importer warns with Chrome's exact negative number while representing size as unavailable, and treats the empty upgrade address as absent. Native files are not repaired. SHA256SUMS includes both files.
