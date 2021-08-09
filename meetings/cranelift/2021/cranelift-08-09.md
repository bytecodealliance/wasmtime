# August 9 Cranelift project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
- cfallin: instruction selection pre-RFC
- cfallin: update on RA2 (review in progress; licensing)
- jlbirch: Simd fuzzbugs ... following a consistent set of standards when lowering.
- General status updates

## Attendees

* Nick Fitzgerald (nf)
* Chris Fallin (cf)
* Andrew Brown (abrown)
* Johnny Birch (jb)
* Afonso Bordado (abordado)
* bjorn3 (b3)
* Benjamin Bouvier (bb)

## Notes

* (cf) instruction selection pre-RFC
  * things have gotten complicated enough that a DSL would be nice
  * "why will this be different from the old DSL we used to have?"
  * learned things, passed a complexity boundary
  * would love your comments and discussion on the pre-RFC!
  * (abrown) read the pre-RFC, it was good, not convinced that we couldn't just
    add some abstractions to the existing hand-written backend without going
    full DSL
    * fwiw, felt the same way about the original old backend, so maybe just
      biased towards fixing existing stuff
    * mostly concerned with easily understanding what is going on
    * depends on what the DSL looks like
  * (cf) does it depend on the DSL semantics? if it is really clear what the DSL
    maps down to thats better?
    * (abrown) the more clear the better
  * (bb) also interested in refactorings for the existing backend and how far
    that can take us
    * with the old backend, we needed better error messages in the DSL and a
      debugger for the DSL, etc
    * building that is a lot of work
  * (abrown) wouldn't mind keeping generated code in-tree if we go DSL route
    * don't have to search for the proper cargo out directory to inspect
      generated code
    * (cf) interesting. the idiomatic rust approach would be to generate in
      build.rs
    * (abrown) didn't peepmatic keep generated stuff in tree?
    * (nf) yes, but mostly so that everyone building cranelift and not touching
      peepmatic doesn't have to have z3, and anything we start new shouldn't
      depend on z3, so it should be a non-issue
    * (b3) rust-analyzer keeps everything in tree
  * (cf) prototyping one design point in this space, lots of open details,
    trying to make sense of it myself, will share once it is more formed
  * (b3) the DSL should be optional
    * (cf) the existing APIs should be kept, need a gradual transition, see the
      horizontal and vertical integration stuff in the pre-RFC
* (cf) update on regalloc2
  * being reviewed by Julian Seward from Mozilla and Amaneiu from the Rust
    Project
  * Looking to relicense from MPL to Apache + LLVM extension
    * Some code derived from SpiderMonkey's regalloc, which is MPL
    * Trying to align with other bytecode alliance projects
* (jb) more SIMD fuzz bugs coming in
  * should we have some sort of criteria/guidance for approaches to lowering?
  * when to use assertions?
  * when to use move helper functions vs emit a particular instruction directly?
  * mostly want consistency across the code base
  * (cf) we should document what invariants we already have, eg:
    * invariants regalloc.rs expects
    * missed other example
* status updates:
  * (bb): none
  * (abrown):
    * working on wasm spec interpreter fuzzing PR
  * (abordado):
    * fuzzing clif
    * adding heap support to filetest infra
    * making sure we don't access invalid memory in the clif interpreter
      * starting with stack memory
      * types of accesses that need to be checked:
        * stack
        * heap
        * tables
        * globals
  * (nf): none
  * (b3):
    * waiting on a review for https://github.com/rust-lang/rust/pull/81746
  * (jb): none
  * (cf):
    * pre-RFC and prototype about one point in the design space to learn more
    * regalloc2
    * thinking about verification in Cranelift
      * thinking that it may make more sense to do end-to-end verification,
        similar to VeriWasm
      * carry symbolic info from wasm through to generated code? similar to a
        recent ASPLOS paper
      * thinking that this is easier and more trustworthy than verifying
        particular lowerings
      * (abrown) we can probably make this easier if we kill some old cranelift
        opcodes, since we are moving towards pattern matching to combine
        instructions in the lowerings
      * (bb) we already have two IRs and if we introduce a DSL we have three
        languages. is this making it harder to verify? also are we still trying
        to push vcode up and replace clif?
      * (cf) replacing clif is not a big priority
      * (b3) vcode not amenable to optimizations that we do on clif
      * (abrown) does cg_clif use all of clif opcodes?
      * (abordado) doesn't use booleans larger than b1
      * (nf) if we do end-to-end verification doesn't matter too much that we
        have muiltiple IRs and languages, since we are essentially just looking
        at the final output, but if we are verifying individual
        lowerings/peephole optimizations, then it matters a lot
      * (cf) similar to unit testing vs integration testing
* (abordado) more questions about checking memory accesses in the clif
  interpreter
  * using native memory+addresses vs indirect tables/maps in the interpreter
  * (nf) using tables/maps in interpreter is obviously correct because
    everything is bounds checked through rust, using native memory+addresses is
    a bit more a whack-a-mole scenario
  * (cf) sort of like allow-list vs deny-list
  * (abrown) I like tables/maps in interpreter but don't want to slow down any
    PRs
  * (cf) we want this to be deterministic for replaying fuzz failures, this is a
    little harder with native memory and different architectures
  * (abordado) will prototype something
