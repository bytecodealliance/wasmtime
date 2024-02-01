Cranelift in Rustc
==================

One goal for Cranelift is to be usable as a backend suitable for
compiling Rust in debug mode. This mode doesn't require a lot of
mid-level optimization, and it does want very fast compile times, and
this matches up fairly well with what we expect Cranelift's initial
strengths and weaknesses will be. Cranelift is being designed to take
aggressive advantage of multiple cores, and to be very efficient with
its use of memory.

Another goal is a "pretty good" backend. The idea here is to do the work
to get MIR-level inlining enabled, do some basic optimizations in
Cranelift to capture the low-hanging fruit, and then use that along with
good low-level optimizations to produce code which has a chance of being
decently fast, with quite fast compile times. It obviously wouldn't
compete with LLVM-based release builds in terms of optimization, but for
some users, completely unoptimized code is too slow to test with, so a
"pretty good" mode might be good enough.

There's plenty of work to do to achieve these goals, and if we achieve
them, we'll have enabled a Rust compiler written entirely in Rust, and
enabled faster Rust compile times for important use cases.

See [issues tagged "rustc"](https://github.com/bytecodealliance/wasmtime/labels/cranelift%3Agoal%3Arustc)
for a list of some of the things that will be needed.

With all that said, there is a potential goal beyond that, which is to
build a full optimizing release-capable backend. We can't predict how
far Cranelift will go yet, but we do have some crazy ideas about what
such a thing might look like, including:

 - Take advantage of Rust language properties in the optimizer. With
   LLVM, Rust is able to use annotations to describe some of its
   aliasing guarantees, however the annotations are awkward and
   limited. An optimizer that can represent the core aliasing
   relationships that Rust provides directly has the potential to be
   very powerful without the need for complex alias analysis logic.
   Unsafe blocks are an interesting challenge, however in many simple
   cases, like `Vec`, it may be possible to recover what the optimizer
   needs to know.
 - Design for superoptimization. Traditionally, compiler development
   teams have spent many years of manual effort to identify patterns of
   code that can be matched and replaced. Superoptimizers have been
   contributing some to this effort, but in the future, we may be able
   to reverse roles. Superoptimizers will do the bulk of the work, and
   humans will contribute specialized optimizations that
   superoptimizers miss. This has the potential to take a new optimizer
   from scratch to diminishing-returns territory with much less manual
   effort.
 - Build an optimizer IR without the constraints of fast-debug-build
   compilation. Cranelift's base IR is focused on Codegen, so a
   full-strength optimizer would either use an IR layer on top of it
   (possibly using cranelift-entity's flexible `SecondaryMap`s), or
   possibly an independent IR that could be translated to/from the base
   IR. Either way, this overall architecture would keep the optimizer
   out of the way of the non-optimizing build path, which keeps that
   path fast and simple, and gives the optimizer more flexibility. If we
   then want to base the IR on a powerful data structure like the
   Value State Dependence Graph (VSDG), we can do so with fewer
   compromises.

And, these ideas build on each other. For example, one of the challenges
for dependence-graph-oriented IRs like the VSDG is getting good enough
memory dependence information. But if we can get high-quality aliasing
information directly from the Rust front-end, we should be in great
shape. As another example, it's often harder for superoptimizers to
reason about control flow than expression graphs. But, graph-oriented
IRs like the VSDG represent control flow as control dependencies. It's
difficult to say how powerful this combination will be until we try it,
but if nothing else, it should be very convenient to express
pattern-matching over a single graph that includes both data and control
dependencies.
