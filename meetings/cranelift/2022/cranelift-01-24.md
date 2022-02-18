# January 24 project call

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
    1. cfallin: 2022 roadmap: thoughts, or merge?
       - [link](https://github.com/bytecodealliance/rfcs/pull/18)
    1. cfallin: ISLE migration coordination
    1. cfallin: platform support tiers; arm32 and updates?
    1. _Submit a PR to add your item here_

## Notes

### Attendees

* fitzgen
* cfallin
* alex
* alexa
* akirilov
* abrown
* jlbirch
* ulrich
* bnjbvr

### Notes

* 2022 roadmap
  * cfallin: hope most of you have seen the RFC I authored, with feedback from
    many folks
  * cfallin: if no one has quibbles, we can probably push it forward and
    finalize it as we are 1/12 into 2022
  * ulrich: had a quick one over the other day and it all looks great
  * cfallin: okay I will make a motion to merge it
* ISLE coordination
  * cfallin: saw the quest issue that fitzgen created the other day
  * cfallin: wanted to make sure that everyone was aware of where we are
  * fitzgen: planning on working on x64 ISLE at about 50% time
  * alex: not working on aarch64 anymore
  * cfallin: I can spare some cycles for aarch64
  * ulrich: outstanding ISLE unknown: how to do ABI stuff and calling
    conventions the ISLE way
  * [missed something about arm32]
  * cfallin: idea: platform tiers
  * cfallin: similar to Rust and its tier 1/2/3
  * cfallin: top tier: guaranteed to compile, pass tests, have CI, and all
    that. lower tier: guaranteed to compile. even lower tier: allow compilation
    failures
  * cfallin: the alternative to the last tier is just remove the WIP arm32
    backend, we don't want it to slow us down
  * akirilov: ARM position is to focus on 64-bit, at least for
    Wasmtime/Cranelift, doubt that any ARM engineers will work on this, ARM is
    in the process of removing 32-bit support from CPUs entirely
  * fitzgen: is it even worth having this code in tree if we don't even check
    that it compiles?
  * ulrich: GCC has a policy that every platform has to have an active
    maintainer who is responsible for fixing things when there are breaking
    changes that affect every platform, if they are MIA then the steering
    committee holds a meeting and decides what to do (potentially removing the
    platform support from GCC)
  * fitzgen: I like that approach, we can do something like that
  * ulrich: yes, just have to ping the platform maintainer first, rather than
    just delete it immediately
  * abrown: every week, I see one or two issues about "is X supported?" we
    should have some documentation about platform support
* ISLE support for calls and ABI code
  * ulrich: ABI/calling convention code is a bit special, it isn't SSA-y
  * ulrich: impossible to use slices of values without borrow check errors
  * cfallin: fitzgen and I talked a little about this, splitting immutable and
    mutable contexts
  * fitzgen: issues with splitting if we don't want to re-engineer non-ISLE data
    structures
  * ulrich: can avoid slices by using `ValueList` instead of `ValueSlice`
  * ulrich: for each argument, call `AbiCaller` impl stuff, do sign extending or
    whatever and all that, then back into target code for generating moves into
    specific registers and all that, all this is incompatible with ISLE, because
    it has its own buffer of to-be-emitted instructions. this will get screwed
    up because ISLE won't be able to rename registers for the stuff emitted by
    platform agnostic code.
  * cfallin: would make sense to move that stuff into ISLE just to simplify the
    moving parts here, have not just `lower` but also `lower_call` and
    `lower_prologue` and `lower_epilogue`. when playing with SSA mach insts for
    initial regalloc2 experiments, had one macro instruction for these things
  * fitzgen: were these macro instructions doing SSA-y things like "new_sp =
    bump old_sp"? and then ensuring that only one sp is live at any time?
  * cfallin: no, just single instructions that did whole prologue/epilogue.
  * cfallin: creating new ISLE that is platform independent would be our first
    platform-independent ISLE
  * ulrich: just have an `extern constructor` in ISLE that calls this ABI stuff
    and emits into ISLE rather than the existing lower context. not very
    ISLE-like. could maybe have some shared code similar to `prelude.isle`?
  * fitzgen: could implement a shared `abi.isle` that relies upon certain
    symbols being defined by platform, but if you have those implemented, then
    everything Just Works
* status updates:
  * jlbirch: wrote some skeleton code for a sightglass benchmarking server,
    still working on the workflow, just wanted to let people know that this is
    being actively worked on
  * fitzgen: more ISLE x64 porting. adding newtypes for GPRs vs XMMs.
  * cfallin: been adding new approaches to cooperative interruption in Wasmtime
    that are faster than fuel, uses cranelift, but doesn't really affect
    cranelift
  * ulrich: big update is the ISLE migration, thanks for reviews. just need
    branches, traps, calls, and returns are still needed. working on traps right
    now. smaller issues: need to add safepoints support for some traps. fixing a
    bug in the `clif.isle` compile-time generator. branches need to know their
    targets when they are lowered. not sure how to get this info inside of
    ISLE. added a context callback to make it work for now, but its a bit ugly.
    * fitzgen: extern extractor to grab branch target for a branch instruction?
    * cfallin: a new entry point into ISLE that is `lower_branch` and has extra
      arguments? call this rather than regular `lower` when lowering branch
      instructions?
  * alexa: just starting an SMT-based interpreter for ISLE rules, no concrete
    results yet.
  * abrown: been working on migrating `select` to ISLE
  * akirilov: no updates, but Fredd(y|ie)? is another ARM engineer who is going
    to join the ISLE aarch64 porting efforts. going to start with atomics and
    SIMD instructions. can coordinate on the ISLE quest issue.
  * alex: a question: would it make sense to start trying to port s390x to
    regalloc2 as a learning experience for other backends?
    * cfallin: there is so much shared code it is basically impossible to do one
      backend at a time. putting this off until we are closer to finishing the
      ISLE migration, and don't have to be so speculative.
  * ulrich: in general ISLE has been very nice and will make adding new
    lowerings much easier. but have also found a couple issues. ISLE code that
    compiles okay but the generated Rust won't compile. working with immediates
    was a little tedious as well, because you end up writing trivial
    constructors for things like addition.
