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

## Spectre

Wasmtime does not yet implement Spectre mitigations, however this is a subject
of ongoing research.
