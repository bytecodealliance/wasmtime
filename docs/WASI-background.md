One of the biggest challenges in WebAssembly is figuring out what it's
supposed to be.

## A brief tangent on some related history

The LLVM WebAssembly backend has gone down countless paths that it has
ended up abandoning. One of the early questions was whether we should use
an existing object file format, such as ELF, or design a new format.

Using an existing format is very appealing. We'd be able to use existing
tools, and be familiar to developers. It would even make porting some
kinds of applications easier. And existing formats carry with them
decades of "lessons learned" from many people in many settings, building,
running, and porting real-world applications.

The actual WebAssembly format that gets handed to platforms to run is
its own format, but there'd be ways to make things work. To reuse existing
linkers, we could have a post-processing tool which translates from the
linker's existing output format into a runnable WebAssembly module. We
actually made a fair amount of progress toward building this.

But then, using ELF for example, we'd need to create a custom segment
type (in the `PT_LOPROC`-`PT_HIPROC` range) instead of the standard
`PT_LOAD` for loading code, because WebAssembly functions aren't actually
loaded into the program address space. And same for the `PT_LOAD` for the
data too, because especially once WebAssembly supports threads, memory
initialization will need to
[work differently](https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md#design).
And we could omit the `PT_GNU_STACK`, because WebAssembly's stack can't
be executable. And maybe we could omit `PT_PHDR` because unless
we replicate the segment headers in data, they won't actually be
accessible in memory. And so on.

And while in theory everything can be done within the nominal ELF
standard, in practice we'd have to make major changes to existing ELF
tools to support this way of using ELF, which would defeat many of the
advantages we were hoping to get. And we'd still be stuck with a custom
post-processing step. And it'd be harder to optimize the system to
take advantage of the unique features of WebAssembly, because everything
would have to work within this external set of constraints.

So while the LLVM WebAssembly backend started out trying to use ELF, we
eventually decided to back out of that and design a
[new format](https://github.com/WebAssembly/tool-conventions/blob/master/Linking.md).

## Now let's talk APIs

It's apparent to anyone who's looked under the covers at Emscripten's interface
between WebAssembly and the outside world that the current system is particular
to the way Emscripten currently works, and not well suited for broader adoption.
This is especially true as interest grows in running WebAssembly outside
of browsers and outside of JS VMs.

It's been obvious since WebAssembly was just getting started that it'd eventually
want some kind of "system call"-like API, which could be standardized, and
implemented in any general-purpose WebAssembly VM.

And while there are many existing systems we could model this after, [POSIX]
stands out, as being a vendor-neutral standard with considerable momentum. Many
people, including us, have been assuming that WebAssembly would eventually
have some kind of POSIX API. Some people have even started experimenting with
what
[this](https://github.com/WAVM/Wavix/)
[might](https://github.com/jfbastien/musl)
[look](https://github.com/golang/go/blob/e5489cfc12a99f25331831055a79750bfa227943/misc/wasm/wasm_exec.js)
[like](https://github.com/emscripten-core/emscripten/blob/incoming/src/library_syscall.js).

But while a lot of things map fairly well, some things are less clear. One of
the big questions is how to deal with the concept of a "process". POSIX's IPC
mechanisms are designed around process, and in fact, the term "IPC" itself
has "process" baked into it. The way we even think about what "IPC" means
bakes in in understandings about what processes are and what communication
between them looks like.

Pipes, Unix-domain sockets, POSIX shared memory, signals, files with `fcntl`
`F_SETLK`/`F_GETLK`-style locking (which is process-associated), are tied
to processes. But what *is* a process, when we're talking about WebAssembly?

## Stick a fork in it

Suppose we say that a WebAssembly instance is a "process", for the purposes
of the POSIX API. This initially seems to work out well, but it leaves us
with several holes to fill. Foremost is `fork`. `fork` is one of the pillars
of Unix, but it's difficult to implement outside of a full Unix-style OS. We
probably *can* make it work in all the places we want to run WebAssembly, but
do we want to? It'd add a bunch of complexity, inefficiency, subtle behavioral
differences, or realistically, a combination of all three.

Ok, so maybe we can encourage applications to use `posix_spawn` instead. And
some already do, but in doing so we do lose some of the value of POSIX's
momentum. And even with `posix_spawn`, many applications will explicitly do
things like `waitpid` on the resulting PID. We can make this work too, but
we should also take a moment and step back to think about IPC in general.

In WebAssembly, instances can synchronously call each other, and it can be
very efficient. This is not something that typical processes can do. Arguably,
a lot of what we now think of as "IPC" is just working around the inability
of processes to have calls between each other. And, WebAssembly instances will
be able to import each others' memories and tables, and eventually even pass
around slices to their memories. In WebAssembly circles we don't even tend to
think of these as IPC mechanisms, because the process metaphor just doesn't
fit very well here. We're going to want applications to use these mechanisms,
because they're efficient and take advantage of the platform, rather than
using traditional Unix-style IPC which will often entail emulation and
inefficiencies.

Of course, there will always be a role for aiding porting of existing
applications. Libraries that emulate various details of Unix semantics are
valuable. But we can consider them tools for solving certain practical
problems, rather than the primary interfaces of the system, because they
miss out on some of the platform's fundamental features.

## Mm-Mm Mmap

Some of the fundamental assumptions of `mmap` are that there exists a
relatively large virtual address space, and that unmapped pages don't
occupy actual memory. The former doesn't tend to hold in WebAssembly,
where linear address spaces tend to be only as big as necessary.

For the latter, would it be possible to make a WebAssembly engine capable
of unmapping pages in the middle of a linear memory region, and releasing
the resources? Sure. Is this a programming technique we want WebAssembly
programs doing in general, requiring all VMs to implement this?
Probably not.

What's emerging is a sense that what we want is a core set of
APIs that can be implemented very broadly, and then optional API
modules that VMs can opt into supporting if it makes sense for them.
And with this mindset, `mmap` feels like it belongs in one of these
optional sets, rather than in the core.

(although note that even for the use case of reading files quickly,
`mmap`
[isn't always better than just reading into a buffer](https://blog.burntsushi.net/ripgrep/).

## A WebAssembly port of Debian?

This is a thought-experiment. Debian is ported to numerous hardware
architectures. WebAssembly in some settings is presented as a hardware
architecture. Would it make sense to port the Debian userspace to
WebAssembly? What would this look like? What would it be useful for?

It would be kind of cool to have a WebAssembly-powered Unix shell
environment or even a graphical desktop environment running inside a
browser. But would it be *really* cool? Significantly more cool than,
say, an SSH or VNC session to an instance in the cloud? Because to do
much with it, you'll want a filesystem, a network stack, and so on,
and there's only so much that browsers will let you do.

To be sure, it certainly would be cool. But there's a tendency in
some circles to think of something like Debian as the natural end goal
in a system API and toolchain for WebAssembly. We feel this tendency
too ourselves. But it's never really been clear how it's supposed to
work.

The insight here is that we can split the design space, rather than
trying to solve everything at once. We can have a core set of APIs
that will be enough for most applications, but that doesn't try to
support all of Debian userland. This will make implementations more
portable, flexible, testable, and robust than if we tried to make
every implementation support everything, or come up with custom
subsets.

As mentioned above, there is room for additional optional APIs to be
added beyond the core WASI set. And there's absolutely a place for
tools and libraries that features that aren't in the standard
platform. So people interested in working on a Debian port can still
have a path forward, but we don't need to let this become a focus for
the core WASI design.

## A picture emerges

While much of what's written here seems relatively obvious in
retrospect, this clarity is relatively new. We're now seeing many of the
ideas which have been swirling around, some as old as WebAssembly
itself, come together into a cohesive overall plan, which makes this
an exciting time.

[POSIX]: http://pubs.opengroup.org/onlinepubs/9699919799/
