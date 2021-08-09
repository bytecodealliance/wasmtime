# August 5 Wasmtime project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
- Alex: update on Wasm64
- Andrew: update on differential fuzzing against spec interpreter
- Till: meetings are finally public
- Chris: SIMD complete on x64 and aarch64

## Attendees
- Alex
- Andrew
- Benjamin
- Chris
- Dan
- Johnnie
- Nick
- Pat
- Till

## Notes
### Wasmtime project meetings public
Till: We now invite the public to join this meeting.
Alex: Should we record these meetings?
Various: It’s a tradeoff, but recording meetings has the downside of making people less open, so it seems like we’ll continue to not record them for now.

### Wasm64 support
Alex: I’m implementing wasm64 in Cranelift.
Initial focus is on getting it working; optimization will happen later.
PR is almost ready; currently doing fuzzing and fixing bugs
Initial implementation will bounds-check on every access
Andrew: Is there a way to use guard pages?
CF: Could we do pointer masking?
Alex: Wasm semantics need a bounds check.
Alex: Our focus right now is just to make sure all our infrastructure is in place for 64-bit addresses. We’ll look at optimization later, and hopefully the people championing wasm64 will help come up with ideas in this space.
Alex: It will also help once we have benchmarks to help us evaluate performance.
Alex: In some settings we may be able to specialize for the case where the memory is dynamically always less than 4 GIB.
Till: If wasm64 becomes popular, people may start asking about ASLR. There’s a lot to think about here?
Dan: Hardware support could significantly accelerate wasm64 and give us more options for things like ASLR.
Till: There are actual use cases for wasm64 out there.

### Differential fuzzing against spec interpreter
Andrew: Wasm spec interpreter and fuzzing.
Fuzzing against the spec interpreter would be nice because we’d be able to compare wasmtime to the spec interpreter. The spec interpreter is written in OCaml, so I’m using the OCaml API to run the spec interpreter.
It’s only scalar for now, but we’ll add SIMD, which will be one of the big motivations for this work.
Requires having OCaml installed on the system.
Instrumentation indicates that about half of the fuzz-generated modules are actually getting executed, so it seems like it’s working pretty well.
There’s currently an unexplained segfault.
Some work items remaining to enable SIMD:
Enable wasm-smith to emit SIMD code
Either use git tricks to use the simd proposal repo, or wait for simd to merge to the spec repo

### SIMD support complete
Chris: Wasm SIMD support is now complete, big thanks to Andrew and Johnnie, and Anton and
Enable it with a command-line flag. One more PR needed for arm64 support.
Getting ready to announce it publicly; need more testing, and possibly the OCaml fuzzing work before enabling it by default
Alex: One way to test is to compile misc Rust crates with autovectorization.
Chris: The fuzzer would be the big thing to aim for.

### [ad-hoc item] Project positioning
Benjamin: What is the project positioning about?
Chris: We’re starting to put together an RFC.
Till: As the project grows, we need to communicate shared goals.
Johnnie: Have there been any specific problems related to this?
TIll: No, but this is something we see across many projects, where individuals and organizations will go off and add features or make changes that meet their needs, and it can be difficult to integrate them into a coherent whole if we don’t have well-communicated shared goals.

### [ad-hoc item] Status of replacing Lucet with Wasmtime?
Andrew: What is the status of replacing Lucet with Wasmtime?
Pat: We’ve made lots of progress, can serialize/deseriealize, though it’s still months out from production use. Wasmtime has all the features.
Till: And we know know of another Lucet user that has switched to Wasmtime.
