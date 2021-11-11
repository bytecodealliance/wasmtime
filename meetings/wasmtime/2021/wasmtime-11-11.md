# November 11 Wasmtime project call

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
    1. _Sumbit a PR to add your item here_

## Notes

### Attendees

- WGR
- cfallin
- acrichton
- tschneidereit
- bjorn3
- abrown
- fitzgen
- Will W
- jlbirch

### Notes

* Till: milestone to report: Fastly recently hit 1 trillion Wasm instantiations; no
  operational issues at scale (using Lucet with Cranelift)
  - Close to switching over to Wasmtime
  - Results from security assessment on integration of Wasmtime with Fastly's
    environment; no issues found

  - jlbirch: how will the switchover work?
  - Till: Not a prolonged period with two separate runtimes
  - Till: will update the group here once it's in production! In testing, we're
    seeing significant performance gains, excited and optimistic.

* Till: looking at optimizing the Python embedding, more info soon

* abrown: back after two months (welcome!), what are we excited about and
  looking forward to in the near future?
  - Till: Alex put together release infra (automated), RFC for 1.0 release.
    - before we do this, maybe a once-over on the documentation would be good
    - also planning around announcement/publicity
  - Till: Fastly has an intern working on wasm-mutate
    - fitzgen: this is for fuzzing; taking a valid Wasm module and tweaking it
      in some way to generate another case, for custom mutator hook
  - cfallin: ISLE DSL in Cranelift
  - WASI: adopted new format for the IDL, renamed it from witx. Better, more
    approachable developer experience. Dan working on applying all of this to
    wasi-libc, wasi-sdk, to make sure we have everything needed.

  - Till: fuzzing?
    - Alex: SIMD disabled until fuzzbugs fixed
    - spec interpreter fuzzing disabled for now due to timeouts
    - V8 differential fuzzing
    - cfallin: alternate way of using spec interpeter for individual
      instruction semantics possible
    - Till: would be good to take stock of SIMD status eventually and see if we
      might want to enable it
  - Andrew: relaxed SIMD, flexible vectors?
    - Alex: relaxed SIMD parsing from Yury at Mozilla
    - cfallin: flexible vectors in progress by Sam Parker at ARM

