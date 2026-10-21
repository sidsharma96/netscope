# Understanding a request with Rust

A browser asks for a list of books. The response arrives, but it takes longer than expected. You open the recording to see what happened. Which request was slow? Did the browser record its response body? What does the timing actually tell you?

Netscope helps you inspect that recording in a terminal. As we build it, we use those questions to understand Rust: how a program stores information, how it shares access to that information, and how it handles missing or invalid input.

This book assumes no Rust knowledge. You do not need to recognise Rust syntax before an explanation begins. New symbols and technical terms are introduced alongside the problem they help solve.

## One request to follow

Our examples follow a browser requesting `https://example.test/api/books`. The response contains a short [JSON](../GLOSSARY.md#json) list. Both the address and its captured traffic are invented, so you can experiment without needing access to a real service.

The recording is a [HAR](../GLOSSARY.md#har) file, a JSON document exported by a browser or another capture tool. JSON represents data using named fields, lists, text, numbers, and a few other values. A HAR arranges those values into records of [HTTP](../GLOSSARY.md#http) requests and responses. HTTP is the protocol the browser uses here to ask for a resource and receive a response.

A recording can be incomplete. A missing response body means the export did not retain it. We will keep returning to distinctions like this because a useful inspector must preserve what the recording actually says.

## A path through the ideas

Start by reading a capture and deciding what you can conclude from it. Then meet Rust's values and functions through a method name and a [status](../GLOSSARY.md#status-code) number. Ownership explains who keeps the request data alive; collections explain how to store many requests without losing their identity.

Once those ideas are familiar, missing values and errors become easier to describe. We then follow a file into the program and a selected request onto the screen. Grouping and redirect evidence show how to calculate useful summaries while preserving uncertainty. Later chapters explain how we check our work and define the first useful version.

The later [sharing lesson](24-sharing-work.md) asks when ownership, channels, locks, and atomics fit a problem. [Shaping the program](23-shaping-the-program.md) connects those choices to the package boundaries and alternatives. The [decision record](../ARCHITECTURE.md) keeps the evidence and open questions beside the lessons.

You can also use the [glossary](../GLOSSARY.md) or the search box to return to a concept. Source links appear after the explanation they support. Reading the implementation is a useful next step, but it is not a prerequisite for understanding a chapter.

## What you can try today

Open a sample capture, choose a request, and inspect its response. The [walkthrough](06-first-milestone.md) shows the keys and the values to look for. Later, you will make an editable copy of a request and send it to a server on your machine. These two activities give us a reason to study borrowing and copying: inspection reads an existing record, while editing needs independent data.

## Begin with an observation

In the supplied capture, find the request method, URL, response status, and body. A method such as GET names the requested operation; a status such as 200 describes the reported response outcome.

Then find one timing recorded as `-1` or not recorded at all. Describe what you know about it without replacing the missing information with a guess. That is the first habit this book asks you to practise.
