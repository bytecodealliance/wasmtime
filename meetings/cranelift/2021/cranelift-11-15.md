# November 15 project call

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
    
### Attendees

* Alexa VanHattum
* abrown
* akirilov
* alexcrichton
* bbouvier
* bjorn3
* cfallin
* fitzgen
* jlbirch
* uweigand

### Notes

* cfallin: merged ISLE RFC, ended FCP this morning. Need to review compiler &
  integration, shouldn't take too long hopefully. How to prioritize moving over
  afterwards? I think we should enable moving everything over reasonably
  quickly. Will unlock improvements to backend design such as RA2. How do others
  feel about helping migration?
* uweigand: Agreed ASAP. Busy working on .NET recently, but release is over!
  Look to do more Wasmtime work soon.
* jlbirch: Also agreed ASAP, and should have time to help.
* cfallin: I'll spend time writing docs for the DSL itself.
* fitzgen: Wrote an overview awhile back and need to write more, yes.
* akirilov: should have more time next quarter
* uweigand: can migrate one-by-one, right?
* cfallin: indeed!
* abrown: how much of x64 is left to do?
* fitzgen: unsure on lines of code, but integer arithmetic is all ported. Alex
  has more SSE stuff as well. Maybe halfway?
* cfallin: ballpark estimate nick?
* fitzgen: few weeks?
* uweigand: only simple things?
* fitzgen: complicated things too like i128 and SSE things. For example `shl`
  for i128 is quite large. Currently porting on-by-one as I go through the big
  `match`.

#### Status

* cfallin: internal project mostly, got info from Mozilla it's ok to relicense
  regalloc2 and then "all" we need to do is to review the compatibility shim to
  regalloc.rs API. Alternatively if timing goes the other way if we transition
  to ISLE happens we can port directly to the pure SSA API, but ISLE does
  everything that would otherwise be done by hand. Still some benefits with a
  compat shim to compile time but more benefits with SSA API. Will write docs on
  ISLE soon.
* fitzgen: Lots of ISLE. Also work on `wasm-mutate` is progressing. Also ran
  benchmarks for ISLE and good results.

... discussion of `wasm-mutate`, wasmtime fuzzing, veriwasm, ...

* acrichto: random ISLE x64 lowerings
* bbouvier: no updates
* akirilov: mostly internal thing. Things about CFI as well. Will need to change
  proposal a bit for a rustc backend.

... discussion about CFI ...

* uweigand: no updates, next step is to implement atomics. Will wait for ISLE
  before adding SIMD.
* abrown: no updates
* bjorn3: no updates
* jlbirch: simd fuzz bug fix
