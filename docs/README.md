# Netscope field notes

Read the chapters below on GitHub, or open `index.html` from a cloned checkout in a browser for the offline reading interface. The interactive diagrams belong to that offline reader. All fonts, styles, and behaviour work without external dependencies. External reference links require internet access.

Edit the Markdown files in `chapters/`, then run:

```sh
python3 scripts/build_docs.py
python3 scripts/check_docs.py
```

Run these commands from the repository root. `chapters.json` defines reading order, navigation labels, and aliases for previous section links. Its sequence follows the concepts a beginner needs, while chapter filenames and URLs remain stable. The generated reader is retained alongside its sources so it can be opened directly. The renderer supports paragraphs, headings, links, inline code, fenced code blocks, and explicit markers for the ownership, import, navigation, overlapping-duration, and redirect-evidence diagrams. Extend the renderer deliberately when adding other Markdown constructs.

The book separates proposed architecture, illustrative teaching examples, and verified implementation. The application supports HAR inspection, host filtering and URL search, reviewed GET/POST replay, response comparison, and saved drafts. Lint and deterministic test rules are enforced by scripts/check.py. The [release notes](../CHANGELOG.md) describe the versioned scope; the [release-readiness record](RELEASE_READINESS.md) records checks and limits.

Read [the writing principles](chapters/05-decisions.md) before editing prose. The reader's questions determine the explanation; code provides supporting examples after the idea is clear. Root AGENTS.md records this requirement for future chapters.

## Chapters

1. [Start here](chapters/01-start-here.md)
2. [Running a program](chapters/22-running-a-program.md)
3. [Reading a capture](chapters/02-reading-a-capture.md)
4. [Values and functions](chapters/07-values-and-functions.md)
5. [Ownership and borrowing](chapters/03-storing-requests.md)
6. [Collections and identity](chapters/08-structs-vectors-and-ids.md)
7. [Missing values and errors](chapters/09-options-results-and-matching.md)
8. [Reading a file](chapters/10-importing-a-har.md)
9. [Drawing and navigation](chapters/11-drawing-and-navigation.md)
10. [Grouping requests](chapters/12-grouping-requests.md)
11. [Following redirects](chapters/13-following-redirects.md)
12. [Recognizing repeated requests](chapters/14-recognizing-repeats.md)
13. [Reading protocol labels](chapters/15-reading-protocol-labels.md)
14. [Preparing a request](chapters/16-preparing-a-request.md)
15. [Waiting for a response](chapters/17-waiting-for-a-response.md)
16. [Sharing work](chapters/24-sharing-work.md)
17. [Comparing responses](chapters/18-comparing-responses.md)
18. [Submitting text](chapters/19-submitting-text.md)
19. [Moving through text](chapters/21-moving-through-text.md)
20. [Remembering a request](chapters/20-remembering-a-request.md)
21. [Shaping the program](chapters/23-shaping-the-program.md)
22. [Checking our work](chapters/04-quality-and-tests.md)
23. [Try the first version](chapters/06-first-milestone.md)
24. [Writing this book](chapters/05-decisions.md)

Chapter links are relative to their Markdown source files. The generator translates chapter links into reader routes and adjusts other local paths for docs/index.html. The binary does not require a documentation website.

The [architecture audit and decision record](ARCHITECTURE.md) distinguishes previously recorded choices from current assessments, links evidence, and records alternatives and open questions. The teaching chapters explain the reasoning in context.
