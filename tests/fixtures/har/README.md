# Synthetic HAR fixtures

In the root-level fixtures listed below, all timestamps and request data are invented. No real endpoints, cookies, or credentials are included. These fixtures are consumed by deterministic importer and CLI tests.

| File | Verified importer behaviour |
| --- | --- |
| empty.har | Accept and display an empty capture |
| single-request.har | Accept one request, retain duplicate headers, report 45 ms total with SSL contained in connect |
| missing-body.har | Accept and distinguish an omitted response body from captured empty text |
| invalid-required-timing.har | Reject `log.entries[0].timings.wait` because required wait is negative |
| stable-order.har | Accept source IDs 0, 1, 2 and display IDs 1, 2, 0; the last two instants are equal despite differing offsets |

The core timing tests use direct numeric values. Importer tests deserialize these fixtures and mutate fixed JSON values to exercise body states, consumed-field failures, duplicate keys, limits, and warning rules. CLI tests consume the invalid-timing fixture and check contextual rejection. Interactive UI acceptance remains pending.

The multi-host.har fixture contains six entries across three normalised URL hosts. API spelling, scheme, and port variants share a group; sub.api.example.test remains separate. Status zero, error statuses, equal starts, a timing warning, and overlapping durations exercise the host-summary contract.

`retries.har` contains synthetic GET/HEAD repeats after selected errors, an excluded POST, missing status, and a source with no match. Its Retry-After value deliberately exceeds the matching window; the inspector does not interpret that header.

`protocols.har` retains exact request/response labels for equivalent aliases, disagreement, and unknown spellings. Its invented entries are not packet or frame captures.

The [browser fixtures](browsers/README.md) instead contain native browser exports of controlled loopback requests, with real recorded timings and synthetic content. Their frozen files are also tested offline.
