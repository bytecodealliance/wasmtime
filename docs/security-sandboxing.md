# Sandboxing

One of WebAssembly (and Wasmtime's) main goals is to execute untrusted code in
a safe manner inside of a sandbox. WebAssembly is inherently sandboxed by design
(must import all functionality, etc). This document is intended to cover the
various sandboxing implementation strategies that Wasmtime has as they are
developed.

At this time Wasmtime implements what's necessary for the WebAssembly
specification, for example memory isolation between instances. Additionally the
safe Rust API is intended to mitigate accidental bugs in hosts.

Different sandboxing implementation techniques will also come with different
tradeoffs in terms of performance and feature limitations, and Wasmtime plans to
offer users choices of which tradeoffs they want to make.

More will be added here over time!

## WebAssembly Core

The core WebAssembly spec has several features which create a unique sandboxed
environment:

 - The callstack is inaccessible. Unlike most native execution environments,
   return addresses from calls and spilled registers are not stored in memory
   accessible to applications. They are stored in memory that only the
   implementation has access to, which makes traditional stack-smashing attacks
   targeting return addresses impossible.

 - Pointers, in source languages which have them, are compiled to offsets
   into linear memory, so implementations details such as virtual addresses
   are hidden from applications. And all accesses within linear memory are
   checked to ensure they stay in bounds.

 - All control transfers—direct and indirect branches, as well as direct and
   indirect calls—are to known and type-checked destinations, so it's not
   possible to accidentally call into the middle of a function or branch
   outside of a function.

 - All interaction with the outside world is done through imports and exports.
   There is no raw access to system calls or other forms of I/O; the only
   thing a WebAssembly instance can do is what is available through interfaces
   it has been explicitly linked with.

 - There is no undefined behavior. Even where the WebAssembly spec permits
   multiple possible behaviors, it doesn't permit arbitrary behavior.

## Filesystem Access

Wasmtime implements the WASI APIs for filesystem access, which follow a
capability-based security model, which ensures that applications can only
access files and directories they've been given access to. WASI's security
model keeps users safe today, and also helps us prepare for shared-nothing
linking and nanoprocesses in the future.

Wasmtime developers are intimately engaged with the WASI standards process,
libraries, and tooling development, all along the way too.

## Terminal Output

If untrusted code is allowed to print text which is displayed to a terminal, it may
emit ANSI-style escape sequences and other control sequences which, depending on
the terminal the user is using and how it is configured, can have side effects
including writing to files, executing commands, injecting text into the stream
as if the user had typed it, or reading the output of previous commands. ANSI-style
escape sequences can also confuse or mislead users, making other vulnerabilities
easier to exploit.

Our first priority is to protect users, so Wasmtime now filters writes to output
streams when they are connected to a terminal to translate escape sequences into
inert replacement sequences.

Some applications need ANSI-style escape sequences, such as terminal-based
editors and programs that use colors, so we are also developing a proposal for
the WASI Subgroup for safe and portable ANSI-style escape sequence support, which
we hope to post more about soon.

## Spectre

Wasmtime does not yet implement Spectre mitigations, however this is a subject
of ongoing research.
