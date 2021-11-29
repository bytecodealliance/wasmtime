# November 29 project call

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
    1. Bug scrub - Maybe this belongs as a Wasmtime open but there are 369 Wasmtime issues dating back to 2016. Surely many are cranelift specific where many are bugs likely in code that no longer exists. What's the best way to scrub these?

### Attendees

* Alexa VanHattum
* abrown
* akirilov
* alexcrichton
* bjorn3
* cfallin
* fitzgen
* jlbirch

### Status

* cfallin: two PRs with documentation! Nick has documented language bindings for
  Cranelift and how to get started with new lowerings. I also posted docs for
  ISLE language semantics itself. Should ideally be clear how language works and
  how to add lowerings. Plan on working on a roadmap for 2022, similar to
  pseudo-rfc document of last year. Let me know if you have ideas! RFC coming
  soon.
* fitzgen: worked on documentation and ISLE.
* alexcrichton: Worked on AArch64 ISLE tidbits. Initial lowering and some sample
  instructions.
* alexcrichton: along the lines of toml-defined `MInst` I thought porting to
  ISLE worked quite well.
* fitzgen: eventually want more information for things like register allocation
  too.
* alexcrichton: perhaps an ISLE "annotation" syntax?
* ... more discussion of how best to represent this ...
* akirilov: sam has a prototype for flexible vectors. Working on understanding
  ISLE and trying to use it. For me there's a PAC prototype and fixing some
  tests.
* abrown: Fixed a too-tight assertion. Wanted to switch `select` to ISLE but
  seemed significant. Fuzzing for simd was also turned on this past week?
* alexcrichton: ah yes! Fuzzing enabled over Thanksgiving and no new fuzz bugs
  have appeared.
* alexa: is it known simd is being fuzzed?
* ... discussion of wasm-smith and csmith heritage ...
* jlbirch: work on wasm64, more coming soon...
* bjorn3: work on getting the blog os compiling with cg_clif
* alexa: working on starting to verify the correctness of individual instruction
  lowerings in ISLE.

### Bug Scrubbing

* jlbirch: lots of really old bugs and wondering if we should scrub some old
  bugs and close out?
* cfallin: one roadmap item is stability and push for quality. Tech debt that
  needs to be finished and things like that. If anyone wants to go through issue
  in free time that's always appreciated. Perhaps can organize something early
  next year.
* akirilov: are wasmtime issues still applicable?
* cfallin: part of task is probably labeling anything with a codegen component
  as cranelift.
* akirilov: are there guidelines for labeling issues?
* fitzgen: labels good for discovery so I don't think there's any need to be
  stingy.
* abrown: some old issues probably need some more labels, e.g. some x64 labels.
  Triage sounds like a good idea.
* akirilov: is there duplication in the labels?
* cfallin: haven't removed any labels ever! Can certainly triage labels
  themselves too.
* fitzgen: if interested in specific areas we do have the tagging bot to get
  tagged for certain labels. Another bot to automatically label PRs with changes
  in certain subdirectories.
