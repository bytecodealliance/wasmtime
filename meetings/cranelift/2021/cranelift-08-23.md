# August 23 project call

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
    1. cfallin: semantics of booleans (#3205)
    1. cfallin: instruction selection DSL (RFC 15)

## Notes

### Attendees

in no particular order:

- CF: Chris Fallin
- AB: Andrew Brown
- B3: bjorn3
- UW: Ulrich Weigand
- SP: Sam Parker
- AK: Anton Kirilov
- Afonso Bordado
- Johnnie Birch
- BB: Benjamin Bouvier

### Notes

- Semantics of booleans: https://github.com/bytecodealliance/wasmtime/issues/3205
    - Inconsistencies between different backends
    - Tribal knowledge about this, mostly
    - New uses of boolean types, e.g. cg_clif
    - Q: should there be repr for a boolean type?
    - Q: what does it mean to have a bool that’s wider than 1 bit?
    - Historically, did have those wider-than-1 bit. Have to be all 0 or 1. Use
    case: bitcast from boolean to other types to get vector masks.
    - SIMD vector compare instructions are better handled for this use case
    - Q: what are the semantics of storing/loading bool from memory + casting to/from
    ints?
    - Historically, validator error to load/store bool from memory
    - Two main options:
        - A. false = 0, true = 1, wider-than-1-bit is 1 (zero-extended)
        - B. wider-than-1-bit is all ones
    - UW: b1’s documentation says it can’t be loaded/stored from/to memory
    - CF: not true as of last week (fuzz bug), need to update doc
    - AB: SIMD bool types must have a known bit repr
    - Q: do we want boolean types at the clif level to behave as the others (can be
    stored/loaded), or do we want to forbid memory accesses to those?
    - SP/UW: Do we know any arch that has sub-byte load/store? Sounds like
    no.
    - AB: fine to not mandate a repr on b1, but useful to have a repr for SIMD
    vectors, since bool vectors are likely to be stored
    - UW: doc is outdated for bool vectors (still mentions forbidden
    loads/stores)
    - Q: why do we want a bool type?
    - CF: we could just remove all the bool types overall
    - AB: what about return values of SIMD compare?
    - CF: only remove all the scalar bool types
    - UW: weird to have bool types only for vector
    - CF: could have b1 for scalar, and b128 for vectors, only
    - UW: what’s the benefit of e.g. b8 over i8 at the IR level?
    - CF: bitmasking stuff will depend on the actual IR type
    - AB: could remove a few `raw_bitcast` if we didn’t have so many bool
    types
    - CF: still want b1, do not allow load/store of bools, do not allow bitcast
    (they don’t have a repr)
    - B3: how would vselect work without bools?
    - AB: bool vectors give guarantees about the actual repr, so that’s nice
    - CF: can’t rely on lowering that the result of loading a b128 from memory is
    actually all ones or zeroes, so would have to canonicalize anyways
    - AK: could have shorter aarch64 sequences if we knew about the repr of
    bool vectors
    - AK: instead of canonicalization, could use pattern-matching up the
    operand tree that the value got produced by an inst that generated all0 or
    all1
    - CF: Proposal: we have wider bool types, and they are guaranteed to be
    canonicalized (insert checks for load/stores/bitcast). Impl could be
    compare-to-0?
    - UW: or shifts, depend on the situation. Would be a factor slower in any
    case.
    - AB: what about the use case where lowering wasm to clif, we load an
    v128 and use it as a mask in another wasm simd op?
    - CF: would need to cast to a bool type
    - Semantics of `raw_bitcast`?
    - Useful to convert from a CLIF type to another, without any change
    at the machine level
    - CF: think about it for some more time, and get back to it?
    - No one disagrees, so everyone agrees
    - Please make suggestions in the issue
- ISLE: https://github.com/bytecodealliance/rfcs/pull/15
    - AK: want to be able to spend less/more time to do pattern-matching according to
    opt level. Would need runtime flags for this. Could this be implemented via the
    extractors?
    - CF: possible to have a switch at meta-compile time to exclude certain rules.
    Should it be a compile-time flag, or a runtime flag (more complicated)?
    - AK: really want a runtime flag to get really fast compile times
    - AK: also need a way to pattern match on CPU extensions
    - CF: would be a runtime flag as well
    - Afonso Bordado: commented about having this kind of predicates on instructions;
    proposal to use the `when` syntax
    - CF: implicit conditioning: no special marking, but if a rule uses an e.g.
    avx512-only inst, automatically detect it and add a predicate on the whole rule
    that it requires the CPU ext.
    - UW: how does it compare with LLVM?
    - CF: Studied related work in pre-RFC (#13). Pattern-matching DSL similar to what
    LLVM does. ISLE is less broad in scope than TableGen and would only be used
    for codegen. ISLE is simpler.
    - UW: in the LLVM community there’s been a push away from SelectionDag
    - CF: bigger compile times. It’s a tradeoff with dev productivity + we did have very
    subtle bugs in the past. LLVM moving to FastISel? because it’s faster. We’re
    building the foundational level of rules, “simple” pattern matching, nice to have a
    DSL at this point. How to make it fast in long run is an open research question.
    - UW: In LLVM, FastISel handles more common use cases and then redirects to
    SelectionDag if complicated cases show up. GlobalISel is supposed to be more
    global (can match across basic blocks).
    - CF: Could have a system with foundational rules + simple optimization rules that
    don’t try to match very deep.
    - AB: would like to try out some code when it’s ready so as to give more targeted
    feedback.
    - BB: risk of scattering code between Rust extern functions + high-level DSL.
    Some old problems are becoming new again. Reinventing many concepts
    present in legalization, concepts overload for newcomers. Risk of seeing bugs in
    the “system”, much harder to debug vs just looking at handwritten code. Tradeoff
    between developer experience and complexity, as said before.
    - CF: re: FFI, mostly isolated. re: complexity, “test and fuzz the crap out of it” :). Re:
    cognitive load, tribal knowledge is starting to appear in the current system (how
    to properly do pattern match without causing subtle errors?). Should be better in
    a lot of ways.
    - AK: emphasis on getting better documentation (blog posts / internal docs).
    - CF: if the system is complicated and requires lots of docs, it’s not ideal. Want to
    make the system easy to understand and have good docs.
    - AB: generated code should be in-tree, for better discoverability.
    - CF: agreed, would help compile times + we could maybe include comments in
    generated code.
- Status updates:
    - UW: CI for s390, qemu patches now in main, some qemu version should work out of the
    box. Yet it (either qemu or wasmtime) doesn’t build anymore on s390. Looking into it
    before being able to run s390 in CI.
    - AK: more aarch64 tests run on qemu. Also have native runners.
