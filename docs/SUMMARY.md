# Summary

# Start here

- [How to read this book](README.md)
- [Understanding a request](chapters/01-start-here.md)

# Foundations

- [Running a program](chapters/22-running-a-program.md)
- [Reading a capture](chapters/02-reading-a-capture.md)
- [Values and functions](chapters/07-values-and-functions.md)
- [Ownership and borrowing](chapters/03-storing-requests.md)
- [Collections and identity](chapters/08-structs-vectors-and-ids.md)
- [Missing values and errors](chapters/09-options-results-and-matching.md)

# Inspecting a capture

- [Reading a file](chapters/10-importing-a-har.md)
- [Drawing and navigation](chapters/11-drawing-and-navigation.md)
- [Grouping requests](chapters/12-grouping-requests.md)
- [Following redirects](chapters/13-following-redirects.md)
- [Recognizing repeated requests](chapters/14-recognizing-repeats.md)
- [Reading protocol labels](chapters/15-reading-protocol-labels.md)

# Replaying a request

- [Preparing a request](chapters/16-preparing-a-request.md)
- [Waiting for a response](chapters/17-waiting-for-a-response.md)
- [Sharing work](chapters/24-sharing-work.md)
- [Comparing responses](chapters/18-comparing-responses.md)
- [Submitting text](chapters/19-submitting-text.md)
- [Moving through text](chapters/21-moving-through-text.md)

# Streams and connections

- [Reading event streams](chapters/25-reading-event-streams.md)
- [Switching protocols](chapters/26-switching-protocols.md)
- [Observing live events](chapters/27-observing-live-events.md)
- [Sharing a connection](chapters/28-sharing-a-connection.md)

# Packets and traces

- [Reading packet bytes](chapters/29-reading-packet-bytes.md)
- [Putting bytes in order](chapters/30-putting-bytes-in-order.md)
- [Finding HTTP boundaries](chapters/31-finding-http-boundaries.md)
- [Reading signaling](chapters/32-reading-signaling.md)
- [Interpreting a trace](chapters/33-interpreting-a-trace.md)

# Building with care

- [Remembering a request](chapters/20-remembering-a-request.md)
- [Shaping the program](chapters/23-shaping-the-program.md)
- [Checking our work](chapters/04-quality-and-tests.md)
- [Try the first version](chapters/06-first-milestone.md)

# Defending the design

- [Defending the design](chapters/34-defending-the-design.md)

# Question bank

- [Newcomer questions](questions/newcomer.md)
- [Practitioner questions](questions/practitioner.md)
- [Expert questions](questions/expert.md)

# Reference

- [Glossary](GLOSSARY.md)
- [Architecture decisions](ARCHITECTURE.md)
- [Install and run](INSTALLATION.md)
- [Working on Netscope](DEVELOPMENT.md)
- [Workspace layout](UI_LAYOUT.md)
- [Browser HAR compatibility](HAR_COMPATIBILITY.md)
- [Large captures](LARGE_CAPTURES.md)
- [Native capture campaign](COMPATIBILITY_CAMPAIGN.md)
- [Behaviour contracts](contracts/overview.md)
    - [M1: import and inspect a HAR](contracts/m1-har-inspection.md)
    - [M2: host summaries](contracts/m2-host-summaries.md)
    - [M2: recorded protocols](contracts/m2-recorded-protocols.md)
    - [M2: redirect suggestions](contracts/m2-redirect-suggestions.md)
    - [M2: retry suggestions](contracts/m2-retry-suggestions.md)
    - [M3: single-request replay](contracts/m3-single-request-replay.md)
    - [M3: request bodies](contracts/m3-request-bodies.md)
    - [M3: response comparison](contracts/m3-response-comparison.md)
    - [M3: saved drafts](contracts/m3-saved-drafts.md)
    - [M4: installation](contracts/m4-installation.md)
    - [M4: keyboard editing](contracts/m4-keyboard-editing.md)
    - [M4: URL search](contracts/m4-url-search.md)
    - [M5: captured SSE](contracts/m5-captured-sse.md)
    - [M5: WebSocket upgrade](contracts/m5-websocket-upgrade.md)
    - [M6: live SSE](contracts/m6-live-sse.md)
    - [M7: packets](contracts/m7-packets.md)
    - [M8: streams](contracts/m8-streams.md)
    - [Replay transport](contracts/replay-transport.md)

# Historical records

- [Milestone status](MILESTONES.md)
- [Release readiness: 0.1.0](RELEASE_READINESS.md)
- [Verification ledger](VERIFICATION.md)

# Appendices

- [Writing this book](chapters/05-decisions.md)
