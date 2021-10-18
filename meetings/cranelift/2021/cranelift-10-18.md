# October 18 project call

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
    1. (fitzgen) update on ISLE integration
    1. _Sumbit a PR to add your item here_

## Notes

### Attendees

- cfallin
- fitzgen
- acrichto
- uweigand
- bjorn3
- sparker
- jlbirch
- bbouvier

### Notes

#### ISLE

- fitzgen: first few rules working and integrated into Cranelift with tests
  passing, (few == 3, iconst, bconst, and catch-all for add). Glue isn't too bad
  and now plan to work on complicated lowerings next. If there are ideas of
  things to prove out the DSL let me know. Next is add with sinking loads, add
  with an immediate, popcnt lowering, compare-and-branch, etc. Hope to have
  final yay/nay in 2 weeks. Personally have enjoyed ISLE so far and it was
  super-easy to add case to 0-immediates are xor reg/reg. Didn't involve fitting
  into match tree and was nice to write as a standalone rule.
- cfallin: gist?
- fitzgen: not PR ready but sure!
- cfallin: this is definitely something I would have wanted last year, but take
  that with a grain of salt. Need to evaluate whether it's worth the complexity.
  Any questions/comments?
- cfallin: no vote today at all, just a progress update, but after two weeks
  hopefully will find consensus one way or another to move forward. Hope to make
  a decision soon though so this doesn't continue indefinitely.
- fitzgen: plan to have enough by next mtg for sure, yeah.
- jlbirch: which aspect is this supplementing or replacing? [the files posted]
- fitzgen: the handwritten match statements in lowering code going from clif to
  machinst.
- jlbirch: these looks like comments for types and not instructions?
- cfallin: oh there's inst.isle and lower.isle

...
discussion of
https://github.com/fitzgen/wasmtime/blob/isle/cranelift/codegen/src/isa/x64/lower.isle
...

...
discussion of
https://github.com/fitzgen/wasmtime/blob/isle/cranelift/codegen/src/isa/x64/inst.isle
...

- sparker: looks quite good for addressing modes on memory operations
- fitzgen: next goal for me!
- bjorn3: looks like a lot of new dependencies?
- fitzgen: uses miette - current branch unconditionally rebuilds isle crate,
  but shouldn't have to rebuild any of this unless you're changing various data
  types. Most don't touch this, so dependencies should only matter if you're
  actively hacking on these areas.
- cfallin: eventually we may not have the meta step or might check in more of
  the code. Would improve build times by a lot, but this is a separate
  discussion with different pros/cons. If build time is a concern there are
  avenues to take.

#### Status

- cfallin: not much, working on non-cranelift things. RA2 progressing a bit,
  jseward planning on finishing review for regalloc.rs adapter. Should be able
  to release to crates.io after that and license bits sorted.
- fitzgen: no other updates than from above.
- bbouvier: no update
- acrichto: no update
- jlbirch: starting to look at some fuzz bugs
- uweigand: working on atomics, everything is cas loops, tedious. Seems like
  clif supports a lot of atomic ops which are easy on x86 as a single
  instruction but not as easy on other platforms. Should we remove bits from
  clif ir?
- cfallin: github issues to remove things and can discuss there?
- uweigand: building wasi binaries from Rust needs a linker which isn't
  available on s390x.
- sparker: looking at flexible vectors, gonna try to put into cranelift somehow.
  Hoping to use ISLE to convert between flexible vectors and neon.
- cfallin: new clif instructions?
- sparker: using existing instructions but getting size of vector into system
  and have backends never see it later ideally.
- cfallin: feel free to post an issue and we can discuss there too
- jlbirch: based on flexible vector proposal?
- sparker: yes
- jlbirch: I would be interested! Would be great to have shared infrastructure.
- sparker: hoping to separate into 3 tiers, first being current simd. Want to
  have shared infrastructure to map to existing targets first.
- bjorn3: cleaning up after old x86 backend removal
- cfallin: great to see so much deleted!
