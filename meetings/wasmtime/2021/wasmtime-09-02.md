# September 02 Wasmtime project call

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
    1. Improvements to precompiled module load times (Alex, 5-10 min)
    1. _Sumbit a PR to add your item here_

## Notes

### Attendees

- Nick Fitzgerald (NF)
- Alex Crichton (AC)
- Benjamin Bouvier (BB)
- Till Schneidereit (TS)
- Pat Hickey (PH)
- Chris Fallin (CF)
- Andrew Brown (AB)
- Johnnie L Birch (JB)
- Dan Gohman (DG)
- Will Woods (WW)

### Notes

- NF: heads-up: intern from KTH to work on wasm-mutate
  - take a wasm binary, parse it, make some transformation (optionally
    semantics-preserving). Will help fuzzing.
- AB: will be on sabbatical for two months (starting in 10 days)
  - all: have fun!
- AC: improvements to module load time
  - https://github.com/bytecodealliance/wasmtime/issues/3230#issuecomment-910528582
  - Making loading precompiled modules fast. Now basically `mmap` and run.
    About 100x faster.
  - Precompiled modules are now valid ELF modules. Allows `objdump -d` to look
    at compiled output code.
  - TS: new API to make this work?
  - AC: currently "deserialize a module from these bytes"; now also have
    "deserialize a module from this file" to allow mmap. Might add other
    flavors as well for other use-cases.
  - JB: Ready to use now? (AC: yes!) Better alternative to `wasm2obj`?
    - AC: wasm2obj is still there; but we can now do `wasmtime compile` and
      disassemble the resulting ELF. This means we see the exact code whereas
      `wasm2obj` sometimes diverged in settings.
  - TS: this was one of the big remaining things where we knew there's work to
    do; now it's done!
  - TS: no full numbers yet to compare against Lucet but if the measurements
    scale up this should be faster than Lucet. wasmtime was originally 20x
    slower. Lucet does dlopen; dlopen needs to do more relocations.
  - AC: one day would be nice to be able to statically link a compiled Wasm
    module into your application. RLBox-style. Not quite there yet.
    - TS: talked to RLBox a while ago, seemed clear wasmtime wasn't right for
      RLBox at the time because of compilation strategy and also call overhead.
      Taking a look at call overhead now (wasm->host, host->wasm) especially
      for non-Rust embeddings.
  - TS: using direct calls -- makes things faster?
    - AC: probably!
    - CF: spectre mitigations disable indirect predictor entirely or partly (?)
      so indirect calls are somewhat expensive; this should be better
  - TS: think that maybe we did direct calls before but maybe not?
    - CF: we just did an Abs8 constant loaded into a register, to avoid
      problems with range
    - CF: relevant on aarch64, part of Alex's patch was careful work to make
      long-range (> 64MiB on aarch64) calls work. Still an issue with
      intra-function branches but that issue was there before; other limits
      come into play as well.
      - AC: yes, e.g. regalloc limits.
      - CF: scaling up to large code and hitting implementation limits is a
        bigger issue that we should definitely address

- TS: APIs; stable C API, unstable C++ API on top of it, call performance
  - supporting calls into native C-ABI functions; more difficult than in Rust
    where we can monomorphize around the function we're given
  - AC: not sure if this will pan out. Would involve JIT compilation between
    wasmtime-defined layers and outside world.
  - DS: impossible to expose wasm functions as C-ABI function pointer?
    - AC: lots of stuff to set up, like setjmp, etc
  - AC: call overhead Rust to wasm is ~20ns, C overhead is ~30ns, both with no
    args, higher with args
    - TS: all runtimes are slow enough that this doesn't stick out; in other
      runtimes calls have to go through JS
  - AC: want to get at least wasm to host overhead as low as possible
