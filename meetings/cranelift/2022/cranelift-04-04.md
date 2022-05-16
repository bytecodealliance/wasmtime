# April 4 project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. _Submit a PR to add your announcement here_
1. Other agenda items
    1. Cache for incremental compilation of individual wasm/cranelift functions, brainstorming/design

## Notes

### Attendees

* Benjamin Bouvier (bnjbvr)
* Nick Fitzgerald (fitzgen)
* Sam Parker (sam)
* Anton Kirilov (anton)
* Chris Fallin (cfallin)
* Alex VanHattum (alexa)
* Andrew Brown (andrew)
* bjorn3
* Johnnie Birch

### Notes

* cranelift incremental compilation cache
  * bnjbvr: once we have already compiled something to machine code, cache the results on disk, if you see the same input again, then reuse the earlier results
  * bnjbvr: can save compile times for hot reload situations or shared libraries
  * fitzgen: do you have measurements? I [did some](https://github.com/fitzgen/measure-wasm-dedupe-wins) for different applications with same libraries inside, and they can't share anything if you just look at bytes of function bodies then nothing ends up using the same wasm function indices for calls, etc
  * bnjbvr: hadn't thought of that, but maybe situation is different for hot reload case
  * fitzgen: makes sense
  * cfallin: fitzgen and I had previously talked about canonicalization [missed some bits here...]
  * cfallin: framework for CLIF
  * fitzgen: maybe makes sense at wasmtime layer, rather than cranelift, since wasmtime already does caching and rustc already has its own caching, and the runtime/driver will certainly want to control knobs here and limit disk space used for the cache and different embedders might want different knobs
  * cfallin: think we should build the framework in cranelift but have hooks for embedder to customize as they want
* updates
  * fitzgen: reviewed regalloc2 checker, looking forward to reviewing regalloc2 in cranelift PR
  * alexa: [there was an update on the ISLE verifier, but I missed it]
  * sam: working on flexible vectors / dynamic types support for cranelift
  * anton: updated rfc for pointer auth and BTI, will make a motion to finalize soon
  * andrew: been porting lowerings to ISLE, working on loads now, moving amode stuff into ISLE
  * cfallin: regalloc2 in cranelift PR is up, does affect backends a little bit, things are faster in compile time (~20-30% faster) and if the benchmark has register pressure also improves runtime (~20% faster)
