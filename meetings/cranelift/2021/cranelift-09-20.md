# September 20 project call

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
    1. Further discussion of ISLE, the proposed instruction selector DSL, as outlined in [RFC #15](https://github.com/bytecodealliance/rfcs/pull/15)

### Attendees

* cfallin
* fitzgen
* uweigand
* alexcrichton
* bjorn3
* afonso360
* akirilov
* jlbirch

### Notes

#### ISLE

* cfallin - Hoping bbouvier could be here but conflict! Will catch up later. Anyone
  have any concerns to discuss?
* jlbirch - what were concerns?
* cfallin - not necessarily as "simple" as what we have right now. Currently
  everything is straightforward to jump in and "see" Rust code. Tracing things
  is easy and you can see what to modify. Also works with IDEs and rust-analyzer
  and other Rust tools. bbouvier wants to preserve this if possible since it's
  open and inviting, minimal knowledge required. Downside is that the benefits
  of the DSL aren't there, fitzgen and I mentioned on thread. There are
  some things we can only do with a DSL such as verification, optimizations,
  refactorings (new regalloc API), ... I think it's also more open and welcoming
  if you can understand the patterns and see them, that way you don't track
  subtle invariants from custom open-coding. More welcoming with custom language
  or to have Rust to read?
* fitzgen - The goal of the DSL should be to thinking about the domain at hand
  rather than the low-level bits and I think it does a good job of that. If you
  see some lowering is missing adding the new operation should be writing a
  pattern and just focusing on the pattern, not also how it fits into the
  hand-written matcher-trees. With the DSL compiler handling all that it's nice
  that it handles performance (optimized lowering) but you're also just talking
  about the pattern you want to match instead of manually doing the lowering.
* cfallin - aspect oriented programming anyone? The DSL brings all the little
  things spread throughout the code into one place in the compiler -- raising
  the level of abstraction and not having to worry about doing unsafe or illegal
  things. Understand Ben's concerns though. Anyone else have similar concerns?
* akirilov - I'm in the middle, leaning towards what you're describing with
  ISLE.
* jlbirch - Worked on compilers awhile ago! No DSL involved. Mostly with
  bytecodealliance I've seen DSLs. Would ISLE looks similar to the wasm backend
  for v8?
* cfallin - not familiar with v8, but you/Andrew have described open-coding, is
  that right?
* jlbirch - looking at a lowering language of some sorts
* cfallin - link?
* jlbirch - should be able to compare what we have to v8 and how it's easy to
  look at and dive in. Haven't had experience debugging v8 though and that's
  presumably where the issues come in.
* cfallin - speaks to a tooling concern and trace what some code becomes and
  why. The output of the DSL should be human readable and should ideally have
  comments for where things came from. Does this in the prototype, not beautiful
  code but still readable. Has comments though and says "this came from ISLE
  line 123". Should be able to step through and see various cases. Maybe higher
  level thing like log/trace to show what was applied? I understand the
  debugging point though, very important.
* fitzgen - regarding what other compilers do, gcc has its own DSL, LLVM has
  tablegen, Go has a DSL for this sort of thing. ISLE does have unique things
  but this shouldn't be too too foreign.
* cfallin - "term rewriting system" - studied for awhile -- not to say it's
  automatically easier. Is understood though.
* jlbirch - Yeah understand it's not too too crazy. Trying to imagine someone
  with no experience in compilers jumping in.
* cfallin - Trying to prevent bugs that have come in with ISLE preventing things
  from being incorrect. Lots of stuff to worry about today with
  regalloc/metadata/sinking loads/etc. Extra mental state we don't want authors
  to have to carry with ISLE.
* jlbirch - generally agree
* cfallin - should catch up with Ben later. Sounds like others agree?
* akirilov - haven't touched the old backend which seems like it has a somewhat
  similar DSL. Would be good to have a guide for how to add a new instruction.
  Main challenge is that there's no guide right now and would be helpful to
  have. Good to know how to add one instruction and to debug.
* cfallin - good idea!
* akirilov - ideally information is close to the project (as opposed to blog
  posts, which are great!) since contributors may not always be aware of
  articles. We have Wasmtime guide with section for contributing? Doesn't cover
  Cranelift though.
* cfallin - Whole `docs` repo to write stuff into, would be great to do.
* fitzgen - would be good to have Cranelift-specific book.
* cfallin - yes!
* akirilov - should link from the Wasmtime book since it appears at the top of
  the repository. Cranelift should be visible too.
* cfallin - agreed! Should document new instructions, new lowerings. Could
  probably source from RFCs and such.
* cfallin - brief mention of progress. The prototype of ISLE exists and they can
  dig into it. Happy to explain more in a call. Nick is going to try to carry
  forward and implement more things end-to-end with polish. Nick?
* fitzgen - Plan is to get one lowering implemented all the way through with
  ISLE and then try ISLE first in existing lowering, falling back to handwritten
  thing. Afterwards knocking out all the patterns. Probably still a week or so.
  This'll quickly be parallelizable where it's mostly just porting patterns,
  talk to me!
* jlbirch - will do my homework and reread these issues and will take you both
  up on the offer and plan to help out
* fitzgen - Looking at pattern -> Rust code translation was very helpful and
  gave me confidence that it's doing what it should do. Confident approach is
  nice and could understand well that what I'm doing maps well.
* cfallin - any other thoughts on ISLE?


#### Endianness

* cfallin - Thank you s390x for making sure we're correct here! Consensus last
  year we have tri-state approach, we have a little/big/native flag on all
  loads/stores. Native important for interacting with the rest of the system.
  Concern that with the interpreter that this makes clif behavior
  platform-dependent. Should have a single defined semantics for clif to prevent
  breaking things up the stack. The suggestion in the issue is that we
  reconsider this and go back to a world where have explicit endian on
  everything, and for native things we bake it in based on the knowledge when we
  generate the clif. Basic approach is to do what other compilers do like LLVM
  with early-bind rather than late-bind. Any objections?
* akirilov - agree! Especially about clif semantics I agree we don't want them
  dependent on the interpreter's host platform.
* cfallin - ok sounds like not much controversy. Sticky point is the API change.
  When you create a clif function or you get a builder you need to give a notion
  of endianness if not more platform information. Corresponds to LLVM which has
  datalayout at the top of every file. Don't think that this will break things
  other than that it's an API change which you should already know.
* uweigand - Confused about how other IRs have been created from the start for a
  particular target and will build different IR for different targets. Have to
  know the target for the IR to do anything with it anyway. Need to keep
  specifying the same target when working with the same file. Or LLVM annotates
  at the top. Sometimes datalayout also has target too. Having an interpreter
  which doesn't know the intended target will really fully work even if
  endianness is fixed. Won't there be other reasons?
* cfallin - one distinction is that the specific target is less important and
  more important about details like endianness. Native loads/stores defined by
  this. Pointer width can also be important. If you give me x86 IR it should be
  possible to in theory compile on a 64-bit big-endian system with byteswaps?
* uweigand - don't have a full overview of the IR, but wondering if we have
  things like pointer offsets which changes offsets and such?
* cfallin - not in the same way of LLVM, the code producer might assume this and
  we may want to check. How would this work. If we're lowering heap_addr on
  64-bit system from IR targeted from 32-bit system maybe...
* uweigand - the specific question seems fine here, tried last year and it
  seemed possible. Lots of code to change though, including code outside of the
  Cranelift repository.
* cfallin - no API stability right now though. Folks here produce clif IR so
  this would impact you. Providing endianness to a builder isn't the biggest
  dealbreaker though since it's often ...
* fitzgen - to what uweigand said, the front/middle generate different IR
  depending on the backend. Are we doing this today in Cranelift? All else being
  equal it would be nice if we always generated the same IR. Some issues with
  endianness though. I liked you recent comment of setting the endianness once
  and still a tri-state native option, but native is explicitly defined. If the
  declaration of what native is the only thing that's different that seems like
  a nice property.
* uweigand - to clarify I wasn't referring to Cranelift, referring to compilers
  like gcc/LLVM where it generates different IR since IR already encodes
  features like the calling convention, struct sizes, etc.
* cfallin - we don't have calling convention details but we do have struct
  layout depending on what the producer is doing. No concept of struct in
  cranelift, but we do have producers that compile structs. Pointer-width fields
  have different sizes.
- acrichto - wasmtime has platform dependent things for VMContext yeah
- afonso - control type for stack\_addr and such is pointer type
- cfallin - if you use 32-bit on 64-bit should be compiled correctly? Would be
  nice to be independent but there's lots of details
- fitzgen - imagining on the cranelift-wasm frontend it's the same
- cfallin - I think it's already true except argument loads/stores?
- uweigand - other way around. All loads/store have explicit endianness. More
  places "leave native" than use little. Most probably use little-endian though
  since it's wasm.
- cfallin - almost have this property nick? maybe don't enforce?
- fitzgen - we have environment traits which customize things we want different.
  Not mad about hook points for those using the frontend. If cranelift-wasm
  decides to ask about the current platform and change the lowering that feels
  bad.
- cfallin - agreed that's bad. This should be used to make behavior
  deterministic. Don't have other platform-specific properties.
- bjorn3 - how test native loads/stores?
- cfallin - different tests? No duplication? Not sure I understand.
- cfallin - other points? Ok sounds like a reasonable approach, Afonso would you
  like to try this?
- afonso - Will probably need guidance but happy to try.

#### Status

- cfallin: ISLE!
- uweigand: s390x - final patches merged and testsuite passes out-of-the-box. PR
  to add s390x to CI. As we were speaking the run finished!
- akirilov: looking into pointer authentication support and code branch target
  indication. These two are related. Just starting to working on an RFC since
  this will probably merit discussions.
