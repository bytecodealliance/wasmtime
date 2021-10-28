# October 28 Wasmtime project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. _Sumbit a PR to add your announcement here_
1. Other agenda items
    1. Discuss [Wasm exception handling](https://github.com/WebAssembly/exception-handling)
       implementation strategy (see
       [#3427](https://github.com/bytecodealliance/wasmtime/issues/3427)).
    1. New release process and making an 0.31 release (@alexcrichton).
    1. _Sumbit a PR to add your item here_

## Notes

### Attendees

* Fitzgen
* Cfallin
* Bjorn3
* Catherine z
* Dgohman
* Alex crichton
* Bnjbvr
* Anton kirlov

### Notes

* Wasm EH strategy
   * Root of decision tree: How do we build support for unwinding? Options:
      * Generate EH tables in DWARF and SEH
         * Pro: ABI compat with other compilers
         * Con: More effort
      * Generate a custom format
         * Pro: we’d control it; no dependency on libunwund
         * Nick: in GC and reference-types, we’re using libunwind already, though historically we’ve wanted to change this, as libunwind is slow and unreliable.
         * Chris: It seems like if we do a custom format for EH, it makes sense to use it for GC and reference-types too.
         * Catherine z: Regardless of the strategy, a lot of the infrastructure can be shared.
         * Catherine z: Other consumers, such as debuggers and other tools, only support DWARF
         * Chris: Another option would be to support all of the above, DWARF, SEH, and our own format.
         * Alex: We can keep clif IR simple, because we don’t need the full DWARF expressivity.
         * Alex: We might start with our own subset of DWARF and use that.
         * Bjorn3: I built a prototype of  https://hackmd.io/@bjorn3/r1kCYBuIt a https://github.com/bytecodealliance/wasmtime/compare/main...bjorn3:eh_cleanup which may be a starting point.
         * Alex: DWARF also gives us dwarfdump and objdump compatibility
         * Nick: That’s been really helpful.
         * Chris: Starting with DWARF sounds good, though we should look closely at SEH to make sure we don’t become accidentally incompatible with it.
         * Anton: If we use DWARF, we get compatibility with the system unwinder.
         * Alex: I expect we’re not going to build our own full DWARF and SEH unwinder; starting with our DWARF subset, we can just implement our own simple unwinder that just does the parts we want, and we can use it on Windows as well.
         * Chris: I envision two RFCs coming out of this: one for the CLIF IR, and one for the output format.
         * Alex: We might be ok with just one RFC.
         * Alex: I consider zero-cost a requirement.
   * Release process and the 0.31 release.
      * Alex: Lots of automation, for releases and publishing packages.
      * Nick: Are we going to have a canary release?
      * Alex: Yes, and we should have some extra documentation around a 1.0 release.
      * I’d like to trial the new release process with a 0.31 release.
      * Chris: do we have docs for the release process, and for the manual version of the release process when the automatic process fails?
      * Alex: Yes and yes. Some parts may still be in my head, so I’m happy to document more things as we find things missing.
   * Johnny: We were experimenting with wasm64, and we aren’t seeing any performance difference. Does anyone have any advice on what tools to use?
   * Alex: clang/llvm have basic wasm64 support, though Rust and wasi-sdk don’t fully support it yet.
   * Johnnie: no initial perf diff with the things we have tested, haven’t looked at perf counters in too much detail, just timings
   * Cfallin: make sure you are comparing against wasm32 without bounds checks (also with bounds checks, I guess)
   * Alex: worth double checking if you are actually using wasm64. Dan, does wasi-sdk have plans for wasm64?
   * Dan: [missed the answer]
   * Anton: could we make wasm64’s bounds checks branchless with cmov where we make pointers null that are out of bounds and then keep using signal handlers?
   * Alex: we would have to be careful to deal with offsets correctly
   * Anton: did some small benchmarks with V8 vs Wasmtime on aarch64, Wasmtime was actually faster on my workloads, it seems like they aren’t using virtual memory tricks for bounds checking, at least on aarch64. On aarch64 explicit bounds checks have ~10% overhead.
   * Alex: we haven’t put a ton of work into optimizing bounds checks or doing redundant bounds checks elimination in cranelift
   * Anton: could possibly use memory tagging for bounds checking in wasm64 on aarch64
   * Cfallin: would be interesting to see some experiments/benchmarks, are cores with this available?
   * Anton: should be available on the market in the next half year or so probably (just a guess)
   * Dan: using mpk is hard because we need to interleave stack storage and the wasm memory region
   * Dan: could have a separate `mmap` of the memory [missed the details for the rest]
   * Anton: that sounds similar to what address sanitizer does
* Anton: RFC for CFI enhancements is posted; feedback welcome.   
