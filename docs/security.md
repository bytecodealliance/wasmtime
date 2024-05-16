# Security

One of WebAssembly (and Wasmtime's) main goals is to execute untrusted code in
a safe manner inside of a sandbox. WebAssembly is inherently sandboxed by design
(must import all functionality, etc). This document is intended to cover the
various sandboxing implementation strategies that Wasmtime has as they are
developed. This has also been documented in a [historical blog post] too.

[historical blog post]: https://bytecodealliance.org/articles/security-and-correctness-in-wasmtime

At this time Wasmtime implements what's necessary for the WebAssembly
specification, for example memory isolation between instances. Additionally the
safe Rust API is intended to mitigate accidental bugs in hosts.

Different sandboxing implementation techniques will also come with different
tradeoffs in terms of performance and feature limitations, and Wasmtime plans to
offer users choices of which tradeoffs they want to make.

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

## Defense-in-depth

While WebAssembly is designed to be sandboxed bugs or issues inevitably arise so
Wasmtime also implements a number of mitigations which are not required for
correct execution of WebAssembly but can help mitigate issues if bugs are found:

* Linear memories by default are preceded with a 2GB guard region. WebAssembly
  has no means of ever accessing this memory but this can protect against
  accidental sign-extension bugs in Cranelift where if an offset is accidentally
  interpreted as a signed 32-bit offset instead of an unsigned offset it could
  access memory before the addressable memory for WebAssembly.

* Wasmtime uses explicit checks to determine if a WebAssembly function should be
  considered to stack overflow, but it still uses guard pages on all native
  thread stacks. These guard pages are never intended to be hit and will abort
  the program if they're hit. Hitting a guard page within WebAssembly indicates
  a bug in host configuration or a bug in Cranelift itself.

* Where it can Wasmtime will zero memory used by a WebAssembly instance after
  it's finished. This is not necessary unless the memory is actually reused for
  instantiation elsewhere but this is done to prevent accidental leakage of
  information between instances in the face of other bugs. This applies to
  linear memories, tables, and the memory used to store instance information
  itself.

* The choice of implementation language, Rust, for Wasmtime is also a
  defense in protecting the authors for Wasmtime from themselves in addition to
  protecting embedders from themselves. Rust helps catch mistakes when writing
  Wasmtime itself at compile time. Rust additionally enables Wasmtime developers
  to create an API that means that embedders can't get it wrong. For example
  it's guaranteed that Wasmtime won't segfault when using its public API,
  empowering embedders with confidence that even if the embedding has bugs all
  of the security guarantees of WebAssembly are still upheld.

* Wasmtime is in the [process of implementing control-flow-integrity
  mechanisms][cfi-rfc] to leverage hardware state for further guaranteeing that
  WebAssembly stays within its sandbox. In the event of a bug in Cranelift this
  can help mitigate the impact of where control flow can go to.

[cfi-rfc]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/cfi-improvements-with-pauth-and-bti.md

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

Wasmtime implements a few forms of basic spectre mitigations at this time:

* Bounds checks when accessing entries in a function table (e.g. the
  `call_indirect` instruction) are mitigated.

* The `br_table` instruction is mitigated to ensure that speculation goes to a
  deterministic location.

* Wasmtime's default configuration for linear memory means that bounds checks
  will not be present for memory accesses due to the reliance on page faults to
  instead detect out-of-bounds accesses. When Wasmtime is configured with
  "dynamic" memories, however, Cranelift will insert spectre mitigation for the
  bounds checks performed for all memory accesses.

Mitigating Spectre continues to be a subject of ongoing research, and Wasmtime
will likely grow more mitigations in the future as well.
