## Why not a more traditional set of POSIX-like syscalls?

In related work, the LLVM wasm backend started out trying to use ELF object
files for wasm, to be as conventional as possible. But wasm doesn't fit into
ELF in some very fundamental ways. Code isn't in the address space, callers
have to know their callee's exact signatures, imports and exports don't have
ELF semantics, function pointers require tables to be populated, index 0 is
valid in some contexts where it isn't in ELF, and so on. It ultimately got
to the point where the work we were considering doing to *emulate* ELF
interfaces to make existing tools happy looked like more than the work that
would be required to just build new tools.

The analogy isn't perfect, but there are some parallels to what we're now
figuring out about system calls. Many people, including us, had initially
assumed that at least some parts of the wasm ecosystem would eventually
standardize on a basic map of POSIX-like or Linux-like system calls into wasm
imports. However, this turns out to be more complex than it initially seems.

One of WebAssembly's unique attributes is the ability to run sandboxed
without relying on OS process boundaries. Requiring a 1-to-1 correspondence
between wasm instances and heavyweight OS processes would take away this key
advantage for many use cases. Fork/exec are the obvious example of an API
that's difficult to implement well if you don't have POSIX-style processes,
but a lot of other things in POSIX are tied to processes too. So it isn't
a simple matter to take POSIX, or even a simple subset of it, to WebAssembly.

We should note that Spectre concerns are relevant here, though for now we'll
just observe that actual security depends on the details of implementations
and use cases, and it's not necessarily a show-stopper.

Another area where WebAssembly differs from traditional POSIX-like platforms
is in its Capability-oriented approach to security. WebAssembly core has no
ability to address the outside world, except through interacting with
imports/exports. And when reference types are added, they'll be able to
represent very fine-grained and dynamic capabilities.

A capability-oriented system interface fits naturally into WebAssembly's
existing sandbox model, by extending the simple story that a wasm module
can't do anything until given capabilities. There are ways to sandbox
traditional OS filesystem APIs too, but in a multiple-implementation
ecosystem where the methods for setting up path filtering will likely
differ between implementations, designing the platform around capabilities
will make it easier for people to consistently configure the capabilities
available to wasm modules.

This is where we see WASI heading.

## Why not non-blocking?

This is an open question. We're using blocking APIs for now because that's
*by far* the simpler way to get the overall system to a usable state, on
both the wasm runtime side and the toolchain side. But one can make an
argument that non-blocking APIs would have various advantages, so we
look forward to discussing this topic with the WebAssembly CG subgroup
once it's set up.

## Why not async?

We have some ideas about how the current API could be extended to be async.
In particular, we can imagine making a distinction between WebAssembly
programs which are *Commands* and those which we'll call *Reactors*.
Commands have a `main` function which is called once, and when `main`
exits, the program is complete. Reactors have a setup function, but
once that completes, the instance remains live and is called from callbacks.
In a Reactor, there's an event loop which lives outside of the nominal
program.

With this distinction, we may be able to say things like:
 - In a Reactor, WASI APIs are available, but all functions have an
   additional argument, which specifies a function to call as a continuation
   once the I/O completes. This way, we can use the same conceptual APIs,
   but adapt them to run in an callback-based async environment.
 - In a Command, WASI APIs don't have callback parameters. Whether or not
   they're non-blocking is an open question (see the previous question).

Reactors might then be able to run in browsers on the main thread,
while Commands in browsers might be limited to running in Workers.

## Why no mmap and friends?

True mmap support is something that could be added in the future,
though it is expected to require integration with the core language.
See "Finer-grained control over memory" in WebAssembly's
[Future Features] document for an overview.

Ignoring the many non-standard mmap extensions out there,
the core mmap behavior is not portable in several respects, even
across POSIX-style systems. See
[LevelDB's decision to stop using mmap], for one example in
practice, and search for the word "unspecified" in the
[POSIX mmap spec] for some others.

And, some features of mmap can lead to userspace triggering
signals. Accessing memory beyond the end of the file, including in
the case where someone else changes the size of the file, leads to a
`SIGBUS` on POSIX-style systems. Protection modes other than
`PROT_READ|PROT_WRITE` can produce `SIGSEGV`. While some VMs are
prepared to catch such signals transparently, this is a burdensome
requirement for others.

Another issue is that while WASI is a synchronous I/O API today,
this design may change in the future. `mmap` can create situations
where doing a load can entail blocking I/O, which can make it
harder to characterize all the places where blocking I/O may occur.

And lastly, WebAssembly linear memory doesn't support the semantics
of mapping and unmapping pages. Most WebAssembly VMs would not
easily be able to support freeing the memory of a page in the middle
of a linear memory region, for example.

To make things easier for people porting programs that just use
mmap to read and write files in a simple way, WASI libc includes a
minimal userspace emulation of `mmap` and `munmap`.

[POSIX mmap spec]: http://pubs.opengroup.org/onlinepubs/7908799/xsh/mmap.html
[LevelDB's decision to stop using mmap]: https://groups.google.com/forum/#!topic/leveldb/C5Hh__JfdrQ
[Future Features]: https://webassembly.org/docs/future-features/.

## Why no UNIX-domain sockets?

UNIX-domain sockets can communicate three things:
 - bytes
 - file descriptors
 - user credentials

The concept of "users" doesn't fit within WASI, because many implementations
won't be multi-user in that way.

It can be useful to pass file descriptor between wasm instances, however in
wasm this can be done by passing them as arguments in plain function calls,
which is much simpler and quicker. And, in WASI implementations where file
descriptors don't correspond to an underlying Unix file descriptor concept,
it's not feasible to do this if the other side of the socket isn't a
cooperating WebAssembly engine.

We may eventually want to introduce a concept of a WASI-domain socket, for
bidirectional byte-oriented local communication.

## Why no dup?

The main use cases for `dup` are setting up the classic Unix dance of setting
up file descriptors in advance of performing a `fork`. Since WASI has no `fork`,
these don't apply.

And avoiding `dup` for now avoids committing to the POSIX concepts of
descriptors being distinct from file descriptions in subtle ways.

## Why are `path_remove_directory` and `path_unlink_file` separate syscalls?

In POSIX, there's a single `unlinkat` function, which has a flag word,
and with the `AT_REMOVEDIR` flag one can specify whether one wishes to
remove a file or a directory. However, there really are two distinct
functions being performed here, and having one system call that can
select between two different behaviors doesn't simplify the actual API
compared to just having two system calls.

More importantly, in WASI, system call imports represent a static list
of the capabilities requested by a wasm module. Therefore, WASI prefers
each system call to do just one thing, so that it's clear what a wasm
module that imports it might be able to do with it.
