# Netscope field notes

Netscope is a terminal program for looking closely at recorded network traffic. You give it a recording, and it shows you the requests inside, what each one asked for, what came back, and how long each stage took. It reads two kinds of recording: a HAR file, which a browser exports from its developer tools, and a classic PCAP file, which a packet-capture tool such as tcpdump writes. It can also send one request again, but only after you have reviewed exactly what will be sent.

The program is careful about the difference between what a recording says and what you might guess from it. A missing timing stays missing instead of becoming zero. A request that looks like a retry is shown as a possible retry, not as proof. Much of this book explains how the code keeps that difference visible, and why.

This book teaches the program and the Rust it is written in. It assumes no Rust knowledge.

## Who this book is for

- **Newcomers to networking or Rust.** You know what a web page is and you have used a terminal. The book introduces HTTP, HAR, TCP and every Rust feature it uses at the point you first need it.
- **Experienced engineers.** You know HTTP and TCP and want to know how this program handles them: which inputs it accepts, what it refuses, and which trade-offs it makes.
- **Anyone preparing to defend the design.** You need to answer an expert reviewer's questions about each decision, its alternatives, its cost, and its known defects.

## What you need first

You need a terminal and, to run the program, a Rust toolchain; [running a program](chapters/22-running-a-program.md) and [install and run](INSTALLATION.md) cover both. You do not need to read Rust already. When a chapter uses a term you have not met, the [glossary](GLOSSARY.md) gives a short definition and links to the section that teaches it.

## Reading paths

Each path is an ordered list. The sidebar holds the whole book in its full order; these paths pick a route through it.

### New to networking or Rust

1. [Understanding a request](chapters/01-start-here.md)
2. [Running a program](chapters/22-running-a-program.md)
3. [Reading a capture](chapters/02-reading-a-capture.md)
4. [Values and functions](chapters/07-values-and-functions.md)
5. [Ownership and borrowing](chapters/03-storing-requests.md)
6. [Collections and identity](chapters/08-structs-vectors-and-ids.md)
7. [Missing values and errors](chapters/09-options-results-and-matching.md)
8. [Reading a file](chapters/10-importing-a-har.md)
9. [Drawing and navigation](chapters/11-drawing-and-navigation.md)
10. [Try the first version](chapters/06-first-milestone.md)
11. [Grouping requests](chapters/12-grouping-requests.md)
12. [Following redirects](chapters/13-following-redirects.md)
13. [Newcomer questions](questions/newcomer.md)

After this path, continue with the replay, stream and packet parts in sidebar order.

### Experienced engineer

1. [Reading a capture](chapters/02-reading-a-capture.md)
2. [Shaping the program](chapters/23-shaping-the-program.md)
3. [Reading a file](chapters/10-importing-a-har.md)
4. [Following redirects](chapters/13-following-redirects.md) and [recognizing repeated requests](chapters/14-recognizing-repeats.md)
5. [Preparing a request](chapters/16-preparing-a-request.md)
6. [Waiting for a response](chapters/17-waiting-for-a-response.md)
7. [Sharing work](chapters/24-sharing-work.md)
8. [Reading event streams](chapters/25-reading-event-streams.md)
9. [Reading packet bytes](chapters/29-reading-packet-bytes.md)
10. [Putting bytes in order](chapters/30-putting-bytes-in-order.md)
11. [Finding HTTP boundaries](chapters/31-finding-http-boundaries.md)
12. [Behaviour contracts](contracts/overview.md), for the exact rules
13. [Practitioner questions](questions/practitioner.md)

### Preparing to defend the design

1. [Shaping the program](chapters/23-shaping-the-program.md)
2. [Architecture decisions](ARCHITECTURE.md)
3. [Defending the design](chapters/34-defending-the-design.md)
4. [Putting bytes in order](chapters/30-putting-bytes-in-order.md)
5. [Finding HTTP boundaries](chapters/31-finding-http-boundaries.md)
6. [Waiting for a response](chapters/17-waiting-for-a-response.md)
7. [Remembering a request](chapters/20-remembering-a-request.md)
8. [Practitioner questions](questions/practitioner.md)
9. [Expert questions](questions/expert.md)

## Testing your understanding

Most teaching chapters end with a few questions under "Check your understanding". The full [question bank](questions/newcomer.md) has three levels: newcomer questions about what the program does and its basic ideas, practitioner questions about how it works and why, and expert questions about failure modes, traps and costs. Each answer stays hidden until you open it and links back to the section that teaches it.

## Where the rules and records live

The chapters explain ideas with small examples. Exact behaviour lives in the [behaviour contracts](contracts/overview.md). The [architecture decisions](ARCHITECTURE.md) record constraints, alternatives and open questions. The milestone, release and verification records under "Historical records" describe what was checked and when. [Writing this book](chapters/05-decisions.md) sets the rules every chapter follows; read it before editing one.

## Building this book

The book is built with mdBook 0.5.4 from `docs/SUMMARY.md`. From the repository root, run `python3 scripts/check_docs.py` to check links, anchors, code links and questions, then `mdbook build docs` to write the site to `docs/book/`. [Working on Netscope](DEVELOPMENT.md#edit-the-book) has the details.
