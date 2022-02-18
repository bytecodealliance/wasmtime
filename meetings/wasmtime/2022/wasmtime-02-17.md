# February 17th Wasmtime project call

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
    1. Conrad Watt: verified Wasm interpreter as fuzzing oracle

## Notes

### Attendees

- Conrad Watt
- cfallin
- acrichton
- abrown
- till
- Dan
- fitzgen
- LGR
- Anton Kirilov
- Kevin Hoffman
- jlbirch

### Notes

- Spec interpreter and fuzzing (Conrad Watt)
  - (slides)
  - Conrad: reference interpreter; Wasmtime fuzzes with it; but quadratic
    behavior
  - Conrad: WasmCert-Isabelle (formally verified Wasm semantics) can extract an
    interpreter; fixes quadratic behavior. Should we use this?
  - fitzgen: does interpreter have a concept of execution fuel?
  - Conrad: yes
  - fitzgen: great; we can get rid of our wasm-smith fuel instrumentation
  - Conrad: perf should be roughly equivalent to cfallin's fork of ref
    interpreter with quadratic behavior fixed
  - Conrad can open a PR
  - Conrad: hazards: "teething pains" -- bugs once interpreter is exposed to fuzzing
    - fitzgen: just fuzz locally a bit before turning on in ossfuzz
  - Conrad: hazards: no line numbers
    - cfallin: doesn't matter too much, we just verify that trap or no trap is
      same on both sides
  - abrown: add a toplevel CLI tool to run ref interpreter?
  - Conrad: post-MVP story.
    - SIMD: can defer to original ref interpreter
    - cfallin: back to quadratic behavior then, or avoid?
    - Conrad: no, just uses arithmetic library/semantics part
  - cfallin: host interaction? GC, imports, etc
    - Conrad: can call imports all the same; GC is mostly internal to interpreter
  - fitzgen: multi-module?
    - Conrad: should be handled
  - Till: plans about component model?
    - Conrad: two parts, standardized imports / WASI-like things, and semantics
      of interface types
    - Conrad: rely on polyfills for now
    - fitzgen: module linking moved into component model, supporting that is
      valuable
    - Conrad: doable, needs Isabelle model of component model
  - cfallin: can we fall back to unverified official reference interpreter at
    the top level (for things like module linking, component model) in addition
    to SIMD?
    - Conrad: technically possible
    - not clear whether this will be implemented in ref interpreter in same way
      as "lower level" things like SIMD
    - harder to do "middle-end" things like exception handling without
      deferring all control flow back to unverified interpreter
  - fitzgen: stack switching?
    - Conrad: unclear current state; unaware of concrete proposal close to
      being brought forward
  - cfallin: future plans?
    - Conrad: looking into reference types, bulk memory
    - Conrad: within 6 mos-1 yr hope to support all currently standardized features
    - Conrad: want to keep patce with standards track in general
  - dgohman: checked against formal model in original Wasm paper?
    - Conrad: "eyeball correspondence": formal model lines up with original
      paper spec; then interpreter is mechanically verified against this formal
      model

- Till: CVE published yesterday in pooling allocator
  - made us discuss more about ensuring we're fuzzing all configurations.
    Previously had rule about fuzzing all implemented Wasm specs for two weeks;
    now extended to all configs as well
  - fitzgen wrote a GitHub bot to post a checklist on config changes ensuring this

- cfallin: updates on memfd, lazy table, epochs
  - instantiation got faster! SpiderMonkey.wasm instantiation from ~a few ms
    down to 3µs
  - not on by default in 0.34; should be in 0.35 (letting it bake in fuzzing
    for one more week)
  - epochs: faster way to do cooperative timeslicing than fuel; 1.5-2x better;
    only downside is nondeterministism

- Liam: KubeCon, call for talks
