<a id="netscope-field-notes"></a>

# Learn Rust through Netscope

A browser asks for a list of books. The reply is slow, and you want to know why. Netscope opens a recording of that exchange so you can inspect the request, its response, and the information the recording leaves out.

This book uses that small investigation to teach Rust and networking. You will follow data from a file into memory, onto a screen, and into a new request you choose to send. Along the way, you will learn how ownership keeps data alive, how errors become useful answers, and how a program can wait for a server while still responding to your keys.

<a id="new-to-networking-or-rust"></a>

## Who this book is for

You should be comfortable opening a terminal. You do not need to know Rust or read its syntax already. New language features are explained when an example first needs them. The [glossary](GLOSSARY.md) is available when you want to look a term up again.

## What you need first

Start with [understanding a request](chapters/01-start-here.md). To try the examples, follow [installation](INSTALLATION.md), then [inspect your first capture](chapters/06-first-milestone.md). The sample contains invented traffic, so you do not need a recording from a signed-in browser session.

## Reading paths

Read the learning chapters in sidebar order. The foundations introduce values, ownership, collections, and errors. Later parts use those ideas to build an inspector, edit requests, wait for responses, and interpret streams of bytes.

<a id="experienced-engineer"></a>

If you already know Rust, you can begin with [reading a capture](chapters/02-reading-a-capture.md) and follow the feature that interests you. Each lesson ends with links into the implementation where those help you examine the complete version.

<a id="preparing-to-defend-the-design"></a>

The final [capstone](chapters/34-defending-the-design.md) asks you to explain the design under questioning. It draws on earlier lessons to examine alternatives, failure cases, and changes in requirements. Read it after the lessons it brings together.

## Testing your understanding

Chapter exercises ask you to predict a result or explain a choice using ideas you have already met. Open an answer after trying the question yourself. The separate [practitioner](questions/practitioner.md) and [expert](questions/expert.md) questions offer deeper review once you are ready.

## Where the rules and records live

Use [the usage guide](UI_LAYOUT.md) to look up controls. Contributors can consult [architecture notes](ARCHITECTURE.md) and [exact behavior rules](contracts/overview.md). Test reports and release history are grouped under contributor records, away from the learning sequence.

## Building this book

To edit or preview the book, follow [the contributor instructions](DEVELOPMENT.md#edit-the-book) and [writing guide](chapters/05-decisions.md).
