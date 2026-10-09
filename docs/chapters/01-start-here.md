# Understanding a request with Rust

A browser asks for a list of books. The response arrives, but it takes longer than expected. You open the recording to see what happened. Which request was slow? Did the browser record its response body? What does the timing actually tell you?

Netscope helps you inspect that recording in a terminal. As we build it, we use those questions to understand Rust: how a program stores information, how it shares access to that information, and how it handles missing or invalid input.

This book assumes no Rust knowledge. You do not need to recognise Rust syntax before an explanation begins. New symbols and technical terms are introduced alongside the problem they help solve.

## One request to follow

Our examples follow a browser requesting `https://example.test/api/books`. The response contains a short JSON list. Both the address and its captured traffic are invented, so you can experiment without needing access to a real service.

The recording is a HAR file, a JSON document exported by a browser or another capture tool. JSON represents data using named fields, lists, text, numbers, and a few other values. A HAR arranges those values into records of HTTP requests and responses. HTTP is the protocol the browser uses here to ask for a resource and receive a response.

A recording can be incomplete. A missing response body means the export did not retain it. We will keep returning to distinctions like this because a useful inspector must preserve what the recording actually says.

## A path through the ideas

Start by reading a capture and deciding what you can conclude from it. Then meet Rust's values and functions through a method name and a status number. Ownership explains who keeps the request data alive; collections explain how to store many requests without losing their identity.

Once those ideas are familiar, missing values and errors become easier to describe. We then follow a file into the program and a selected request onto the screen. Grouping and redirect evidence show how to calculate useful summaries while preserving uncertainty. Later chapters explain how we check our work and define the first useful version.

You can also use the Rust index to return to a concept. Source links appear after the explanation they support. Reading the implementation is a useful next step, but it is not a prerequisite for understanding a chapter.

## What you can try today

Netscope can open a local HAR, list its requests, and show a selected request's headers, body, and available timings. It also reports invalid input and inconsistent measurements. A host view groups requests by URL host and opens a filtered list without changing their IDs. The inspector also shows possible redirect follow-ups, with missing evidence and ambiguity labelled. It does not send the recorded requests again.

The [first version walkthrough](06-first-milestone.md) uses the supplied synthetic capture. Replay and deeper network analysis belong to later work. The [verification record](../VERIFICATION.md) keeps track of what has been checked.

## Begin with an observation

In the supplied capture, find the request method, URL, response status, and body. A method such as GET names the requested operation; a status such as 200 describes the reported response outcome.

Then find one unavailable timing. Describe what you know about it without replacing the missing information with a guess. That is the first habit this book asks you to practise.
