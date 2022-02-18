# January 10 project call

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
    1. Testing strategy; golden tests and auto-updating (PR #3612)
    1. _Submit a PR to add your item here_

## Notes

### Attendees

- abrown
- acrichton
- akirilov
- avanhattum
- bjorn3
- bnjbvr
- cfallin
- fitzgen
- jlbirch
- sparkerhaynes
- uweigand

### Notes

- [Regenerating golden output for tests automatically](https://github.com/bytecodealliance/wasmtime/pull/3612)
    - acrichton: we have manually written tests in clif, using filecheck
        - requires manual updates everytime there's a large codegen change, e.g. not using FP for leaf functions
        - the idea: having an env variable that regenerates the golden output for tests on demand
        - when a large codegen changes happen, run with the env variable set, which regenerates the tests
        - no effort to update all the tests!
        - cfallin: likes it: precise compile tests are becoming more rare, we need more execution tests.
        - akirilov: could have been and be useful for some aarch64 codegen work
        - uweigand: LLVM has filecheck and a python script that updates the tests automatically like that.
        - abrown: notes other change made by this PR: change the display of displayed code in the tests by using VCode's `Debug` display
        - cfallin: in the future, we should prefer run tests (for testing execution), vs precise compile tests ought to be used only for use cases like checking lowering produced the expected patterns
        - acrichton: should we in a single PR modify all the compile tests to be all precise tests, so they can modified by the tool automatically?
            - yes
        - acrichton: as there's no objections, will merge the PR

# standups

- alexa: work in progress for verification, Fraser Brown from CMU also interested to join effort.
        - why are things backwards for the extractor? from "return value" to "arguments"
        - one reason:
            - there exist extractors with one return value into multiple values
            - no multiple values in the lang semantics itself
        - another reason: some terms can have extractor and constructor
    - is there a guarantee that vector types in Cranelift are clearly defined? (reliability of types in IR in general)
        - vec types defined in cranelift are up to 128 bits, no arbitrary width
        - acrichton: one could use popcnt on f32
        - cfallin: verifier should be seen as addition to formal verification effort; assume verifier checks code first
- fitzgen: more progress on converting x64 ops to isle
- acrichton: some x64 conversions too. did test the patch release process this month
- abrown: working on migrating select to isle on x64
    - fitzgen: can land for *some* types even if not all of them are implemented
- uweigand: no updates. Will start working on migrating backend to isle.
- bnjbvr: no updates.
- sparkerhaynes: figuring out variable-width vector. for SVE fixed-width impl, b/o too much ambiguity related to size in the IR.
    - backend flag to use fixed sizes in the short term, can do better later.
    - bjorn3: could this be in cranelift-wasm instead of within each backend?
        - sam: started with accepting a new IR type, instructions will come later
- jlbirch: plan to work on ISLE
- akirilov: proof of concept for CFI for basic blocks which are targets of indirect branches on aarch64, using the BTI instruction
    - excluding indirect function calls, as there's pointer auth for those
    - only *br_table* implementation generates indirect branches
    - cfallin: any overhead?
        - anton: runtime hard to measure because BTI not well supported in hardware right now.
    - empty basic blocks were not materialized because of empty block folding, now every block that's the target of an indirect branch may have BTI instructions at the top => more materialized blocks!
        - cfallin: have `MachBuffer` emit BTI, or know about BTI which would be "optional" instructions
    - cfallin: measurements will tell us whether we want to enable this by default, or configuration option. Probably config option.
        - anton: if BTI instructions are supported by hardware, generate them, otherwise not.
    - abrown: intel has something similar to BTI, works differently. Requires a kernel that supports Intel CT, compile wasmtime with special support, etc. Is that the case with ARM too?
        - anton: those instructions become NOPs if not supported by the hardware. Can flag singular memory pages for BTI support when mmapping. It's not mandatory.
        - abrown: can imagine in the future there's a single flag that is platform independent
        - anton: challenge with AOT, if generated code has no BTI instructions and it runs on some platforms that enables BTI support for every page by default, will cause runtime exceptions.
            - cfallin: will require some metadata in addition to the AOT code
            - akirilov: in general, can we assume that AOT-compiled code runs within the same runtime environment?
            - acrichton: runtime (Rust) errors when the actual hardware running AOT-compiled code doesn't match such requirements, could use this for BTI.
            - akirilov: could also use this for large vector support later
- cfallin:
    - Fuzz bug, involving type info lost during regalloc. When emitting moves between values, equivalence classes are created to coalesce moves; an arbitrary class leader is used to get the type and storage size for all vregs in the class. Later when spilling, this size could be smaller than the actual storage of another vreg in same equiv class.
    - Short term fix: use the largest size that's possible for this reg class. Means wasting some space in some cases (e.g. could allocate 128 bits when spilling f32).
    - Better fix would be to keep the type information precise, but that's a regalloc invariant we're not maintaining with respect to move instructions. Need to "typecheck" moves and make sure lowerings are correct.
