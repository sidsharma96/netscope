# How does source code become a command?

You have opened Netscope from its project directory several times. Now you want to type `netscope` from a different folder and inspect a capture there. What needs to exist for that command to work?

There are two separate steps. First, the Rust source must become an [executable](../GLOSSARY.md#executable) that your computer can run. Then the shell—the program reading your terminal commands—must find that executable. Understanding the difference helps explain why a successful build can still be followed by “command not found.”

## Building creates another file

A Rust source file contains instructions written for people and the [compiler](../GLOSSARY.md#compiler). The compiler checks those instructions and translates them into a form the target computer can execute. Building also brings together code from the application's dependencies. The result includes an executable file: a program the operating system can start.

[Cargo](../GLOSSARY.md#cargo) coordinates this work. It reads package information from Cargo.toml, selects dependencies under the lockfile rules, and invokes the compiler. In this project, the package that owns the terminal application is named netscope-tui, while its executable is named netscope. Package names and command names can differ.

Follow the arrows in this diagram as actions that produce or place files. Running the final file is a later action; compiling does not start the interactive application.

```text
Rust source + dependencies
          │
        build
          ↓
Executable in the build directory
          │
        install
          ↓
Executable in an installation directory
          │
          run
          ↓
A running process with its own memory
```

The source and executable are separate files. Changing a source file does not rewrite an already installed executable. You must rebuild and reinstall to make the installed command reflect your edits. Cargo's [introduction to building and running](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html) explains that separation and its normal build locations.

## The shell has to find the executable

Suppose an installation places a program in `/example/tools/bin/netscope`. Typing that full path tells the shell exactly which file to start. Typing only `netscope` leaves the location unspecified.

[PATH](../GLOSSARY.md#path) supplies a list of directories where the shell searches for commands. On macOS and Linux, the entries are separated by colons. PATH is an [environment variable](../GLOSSARY.md#environment-variable): a named value made available to a process and normally inherited by processes it starts. It contains directories, not the Rust source code or a list of individual executable files.

Here is an illustrative search order, not a reading of your machine's settings:

```text
Command: netscope
PATH:    /example/tools/bin:/usr/local/bin:/usr/bin

         first directory   → look for netscope
         next directory    → try here if needed
         next directory    → continue the search
```

If the installation directory is absent from that search, the executable can exist while its short command name still fails. Running it by its full path separates a search problem from a program problem. If an older copy appears earlier in the search order, you may also run a different copy from the one you just built. The installation guide shows how to inspect that choice.

The working directory answers a different question: where relative file paths begin. If you run `netscope capture.har` from a folder containing that capture, the argument refers to the file there. It does not refer to a file beside the executable. Installing the program should not require keeping your recordings in the source checkout.

## Build tools and runtime needs differ

Building Netscope requires Cargo, the Rust toolchain, and the dependencies needed for compilation. Once built, starting the installed executable does not invoke Cargo to compile it again. It still depends on the operating system and compatible runtime facilities; a binary built for one operating system and processor is not automatically usable on another.

The book and sample captures are useful companion files in the repository. They are not files the installed application secretly loads to interpret your own capture. A first-run test from another directory is useful precisely because it exposes accidental reliance on those nearby project files.

Cargo installation normally builds with its release profile and puts commands under the installation root's bin directory. A profile is a set of build settings; release settings generally enable optimization. Installation location can be configured. The [Cargo install reference](https://doc.rust-lang.org/cargo/commands/cargo-install.html) gives the exact options; the [Netscope installation guide](../INSTALLATION.md) provides the commands for this checkout.

## Ask the installed program what it is

Help and version output should work before you supply any capture. `netscope --help` explains accepted arguments and basic controls. `netscope --version` identifies the package version compiled into that executable. It does not inspect the current source directory or prove that two builds with the same package version contain identical code.

A command also returns an exit status, a small number that a shell or another program can inspect. Netscope uses zero for success, one for a file or runtime failure, and two for invalid arguments. This lets a script distinguish an unreadable capture from a misspelled option without interpreting every sentence of an error message.

After installing, open a new terminal or check your command search path, then ask for the version and help. Next, open a supplied capture from a different directory using its full path. Predict which part would fail if the executable were missing, if its directory were missing from PATH, or if the capture path were wrong. Those are three different failures, even though each begins with a command that does not do what you expected.

The [entry-point contract](../contracts/m4-installation.md) records argument and exit-status rules. The command entry point [`main`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/main.rs#L43) chooses a mode, and the isolated installation check [`check_install.py`](https://github.com/sidsharma96/netscope/blob/main/scripts/check_install.py#L1) verifies that the resulting command works away from the checkout.

<!-- check-your-understanding: generated by scripts/sync_questions.py from docs/questions; edit the bank, not this section -->

## Check your understanding

These questions come from the question bank ([newcomer](../questions/newcomer.md), [practitioner](../questions/practitioner.md), [expert](../questions/expert.md)). Answer each one before you open its answer.

### Building, installing and running

**Level:** Newcomer


You built Netscope, but typing `netscope` in another folder prints "command not found". What are the separate steps between source code and a working command, and which one failed?

<details>
<summary>Answer</summary>

Building turns the Rust source into an executable file in the build directory. Installing copies that executable into an installation directory, such as Cargo's `bin` directory. Running it by its short name requires the shell to find it. The shell searches the directories listed in the PATH environment variable, in order. "Command not found" usually means the installation directory is not on PATH. Running the executable by its full path tells you whether the file itself works.

**Learn more:** [Building creates another file](../chapters/22-running-a-program.md#building-creates-another-file) · [The shell has to find the executable](../chapters/22-running-a-program.md#the-shell-has-to-find-the-executable)

</details>

### Where a relative capture path points

**Level:** Newcomer


You run `netscope capture.har`. Netscope is installed in one folder and the shell is open in another. Which folder must contain `capture.har`?

<details>
<summary>Answer</summary>

The folder the shell is in, called the working directory. A relative path such as `capture.har` starts there, not next to the executable and not in the source checkout. The installed program also does not load the book or the sample captures from the repository, so you can keep your recordings anywhere and pass their path.

**Learn more:** [The shell has to find the executable](../chapters/22-running-a-program.md#the-shell-has-to-find-the-executable) · [Build tools and runtime needs differ](../chapters/22-running-a-program.md#build-tools-and-runtime-needs-differ)

</details>

### Exit status and version output

**Level:** Newcomer


What exit statuses does Netscope return, and what does `netscope --version` prove?

<details>
<summary>Answer</summary>

Netscope exits with 0 for success, 1 for a file or runtime failure, and 2 for invalid arguments. A script can therefore tell an unreadable capture from a misspelled option without parsing the error text. `netscope --version` prints the package version compiled into that executable. It does not read the source directory, and two builds with the same version number can still contain different code.

**Learn more:** [Ask the installed program what it is](../chapters/22-running-a-program.md#ask-the-installed-program-what-it-is) · [`main`](https://github.com/sidsharma96/netscope/blob/main/crates/netscope-tui/src/main.rs#L43)

</details>
