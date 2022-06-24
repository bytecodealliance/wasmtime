# June 6 project call

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
    1. _Submit a PR to add your item here_

## Notes

### Attendees

- abrown
- akirilov
- avanhattum
- cfallin
- fitzgen
- sparkerhaynes

### Notes

- ISLE porting status and plans?
  - abrown: Quarterly goals at Intel, port a number of instructions over to ISLE.
    - Want to continue doing so.
    - Issue is that all the hard ones are left.
    - We need more people to look at it.
  - akirilov: Quarterly basis too.
    - Our approach was different, we've gone for the harder ones first.
    - Trying to improve test coverage.
  - abrown: How are you guys tracking coverage?
  - akirilov: It's just manual, the bar is currently quite low so easy to see where they're missing.
    - Interpreter has failed to handle some cases, something to do with splat.
    - Some run tests don't check the return values.
    - Sometimes disabling the interpreter tests.
  - cfallin:  Sounds like the right approach.
  - abrown: What's the state of SIMD interpreter support? afonso was working on it.
  - cfallin: Don't know what happened to afonso, work has stalled on it.

- akirilov: Regalloc2 limitation of only 2 reg classes
  - SVE will need a predicate file, same for AVX-512.
- cfallin: Bit packing issue, could remove the 'mod' operands and reuse that bit.
- akirilov: Will four classes be enough..? It is probably fine for AArch64.
- cfallin: Will keep this on my back burner.

# standups

- fitzgen: No updates.

- avanhattum: Better semantics for verification.
  - SAIL for x64 and arm.
  - Figuring out what work is needed to use modern SAIL for x64.
  - Working on shim code to avoid annoting machine instructions.

- akirilov: cranelift CFI patch updates and Fiber changes for MacOS.
  - bjorn3 gave feedback and noticed codegen wasn't right.
  - some branches were elided and the branch target instructions ended up as the last instruction, not first.
  - pointer authentication is still disabled in qemu.

- cfallin: Implemented alias analysis for load elimination, with okay speedups.
  - could have benefitted some option optimizations, such as GVN
  - So developing a unified framework for rewrite rules in the mid-end.
  - egraphs seem the right approach.
  - need an adapter to convert the egraph to LowerCtx: clif -> egraph -> vcode
  - Then need to figure out isle rewrite rules.
  - Framework will subsume all the existing mid-end optimisations.
  - Also worked on a fix regalloc2 issue, reducing stack size usage.
    - alexa: static rewrite rules or equality saturation?
    - chris: equality.

- abrown: shared memory in wasmtime.
  - impacts cranelift through `memory_size` instruction.
  - How are we gonna test this? Looking for ideas and feedback.
  - Ported some atomic operations for x64, not sure CAS is right.

- sparker: Dynamic vector RFC is up in code form.
  - It's a bit rough and would greatly appreciate some feedback.
    - cfallin: does qemu support sve?
    - sparker: yes.
