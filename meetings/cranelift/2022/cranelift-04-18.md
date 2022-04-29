# April 18 project call

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
    1. Revisit generated-ISLE-code-checked-in? Rebasing pain in PRs (developer
       experience, CI time) vs. build time impact, and what we could do to mitigate
    1. Generalized left-hand sides (predicates) in ISLE?
    1. Plans and timeline for finishing ISLE migration
    1. _Submit a PR to add your item here_

## Notes

### Attendees

### Notes

* ISLE, checking in code, and rebasing
  * cfallin: merge conflict any time two PRs change any ISLE at all
  * cfallin: makes it annoying to rebase, even when you "shouldn't" have to
  * cfallin: original motivation was to avoid deps for cranelift-codegen to keep build times down and to make the code legible
  * cfallin: also it is a bit unidiomatic to have generated code checked in, annoying to have to specify a cargo feature to rebuild
  * cfallin: interested in generating the code again on every build again
  * abrown: a pro for having it checked in is that during debugging you don't have to figure out which of the various cargo target output directories is the one actually being used to set a breakpoint in
  * cfallin: we could in the fullness of time make an LSP server..
  * fitzgen: if we get that far, we've done somethign wrong
  * cfallin: we could add tracing of which rules/LHSes matched
  * bjorn3: At least for crates.io releases I really want the generated code to be published. Maybe it could rebuild when using it from git. 
  * bjorn3: The current isle code is much slower than the original meta crate. At least in terms of compile times. 
  * bjorn3: Another thing is that it brings in a lot of dependencies which would all have to be whitelisted in rustc's tidy checker. 
  * avanhattum: can we separate the debuggability of local source from whether the code is checked in or not?
  * cfallin: maybe a cargo feature for this
  * abrown: I've also felt the pain of rebase conflicts in ISLE, fwiw
  * fitzgen: will rebase problems go away after we are done porting instruction selection to ISLE?
  * cfallin: will be annoying for new contributors, an extra speed bump
  * avanhattum: can we avoid merge conflicts by generating a more stable output? eg a new rust file for every rule in the extreme
  * fitzgen: add a cargo flag to include the source position comments, have it off by default
  * cfallin: yes, we could have a mode for "I'm debugging ISLE source" and a default mode for not
  * [missed some stuff; talking about adding logging to the generated code]
  * fitzgen: this should be a cargo feature too since logging is not zero overhead even when disabled
* Left-hand sides in ISLE
  * cfallin: came up when talking with abrown a week or two ago
  * cfallin: sometimes hard to use extractors to specify what we want
  * cfallin: case where want to check if two `u32`s can be added and not overflow
  * cfallin: so we have an extractor with an in-arg, feels very awkward and not like the right solution
  * cfallin: other pattern matching languages just have top level predicates or conditions
  * cfallin: combinator for a list of patterns to match instead of a single one
  * cfallin: would all be rewritten away after the `islec` frontend when translating from `sema` to `ir`
  * fitzgen: would like to see some examples
* ISLE migration
  * cfallin: how is it going? what timelines do you have? how can we help?
  * jlbirch: not currently involved, but it is one of our internal goals for the quarter, looking to start contributing soon
  * abrown: the easy stuff is easy, the hard stuff is not as easy, room for improvement in ISLE syntax (last topic) but also being able to do basic math/arithmetic in ISLE would be nice
  * abrown: have given up on i128 for now; it is a whole different world
  * cfallin: going to prioritize meta issues of making ISLE itself better
  * abrown: afaik cg_clif is the biggest user of i128; do you want to help with migrating the i128 lowerings?
  * cfallin: also is there a world where cg_clif handles i128 itself? would make it easier for us, but also fine if not; other option is we do this in the mid end
  * bjorn3: can try helping out with i128 lowerings but not for a little bit
  * bjorn3: cg_clif can't handle primitive values consisting of multiple Cranelift Value's at the moment. In addition only the backend can do efficient carrying and some ABI's may require Cranelift integration too.
