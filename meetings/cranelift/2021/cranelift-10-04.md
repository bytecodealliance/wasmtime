# October 4 project call

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
    1. Further discussion of [ISLE DSL](https://github.com/bytecodealliance/rfcs/pull/15)
        - Quick ISLE tutorial ([slides](https://docs.google.com/presentation/d/e/2PACX-1vTL4YHdikG70GZuWvnUOqWdE31egZDBj-2-ajsNfoLkeUn8Bpvk_a5vEFOQqsolcUuR9pmYj2qPF-_J/pub)) 

### Attendees

- cfallin 
- fitzgen
- bbouvier
- akirilov
- uweigand
- bjorn3
- alexcrichton
- afonso

### Notes

* cfallin - one thing on agenda! -- ISLE
* cfallin - made more progress, Nick has been improving the prototype posted
  awhile back. It's generating Rust code for the basic concepts and Nick's been
  improving and adding error reporting. Driving Cranelift integration, assuming
  we do this, as well
* cfallin - continued discussion today to make sure that this is what we want to
  do. Lots of good discussion points raised in the RFC. Nick has some slides if
  folks would find that useful. Shows some code generated. After that some open
  discussion?

... slides! ...

* cfallin - concerns after presentation?
* uweigand - not really a concern but need details like special cases and
  particular constants in particular ranges. Can always do whatever since it's
  custom Rust. Always cases to do something special.
* cfallin - special cases are extractors and constructors -- becomes a trait
  method and you can write whatever you want. In the extreme can do it all in
  Rust with one method, but everything beyond that is making use of
  functionality for pattern matching.
* uweigand - another question - is this going to be another big bang from the
  old way to the new way?
* cfallin - no first goal was a migration story, the idea being that this will
  be driven by doing the "new thing" first and falling back to the "old thing"
  if `None` is returned. Will put ISLE in place, start with "add" or something,
  ask for help with PRs. Individual instructions should be simple and when it's
  all done we can remove the old way.
* akirilov - second escape hatch!
* cfallin - yes still have old way where we can use if ISLE can't handle
  something and we can figure out later if we need to add something to ISLE
* bbouvier - what if ISLE generates slower Rust than what we do today, we won't
  figure this out with incremental rewrite and would need to compare two points
  in time which could have other changes in the middle. Hard to measure?
* cfallin - should have no performance regressions by construction. Should
  generate the same code that we're writing by hand. Shouldn't have regressions
  at this time. If many more complicated patterns arise then we should be in
  theory generating better code. In the future we write more complicated code
  with more complicated patterns that's a new state. Today though we should
  strive to make sure the generated code matches what we do today.
* akirilov - can check for regressions with current backends by not deleting
  code in current backends and skipping ISLE to do comparison.
* cfallin - true, can add all the things and delete separately, but the downside
  is that we have two sources of truth with possible divergences. Have the
  flexibility of doing that though.
* bbouvier - risk that there is an exponential growth of pattern matching trees.
  Could two rules interact in a way that they do deeper matching? Hard to test
  we didn't do that in particular with incremental rewrite.
* cfallin - could be interesting interactions, yeah, currently a property is
  that the output of ISLE is linear in the input size. No iteration to a fixed
  point in combining rules, it instead does only a single layer. Shouldn't get
  exponential blowup in that regard but could be interesting interactions
  perhaps though, but none over what we already do.
* bbouvier - concern about verbosity - if a specific rule is commutative do you
  need to write a rule twice? Or systemic checks about commutativity?
* cfallin - nothing like that right now, would write it twice. Idea is to have
  it be simple first. If we express and can lower it simply then we can do that
  as well yeah.
* fitzgen - I think this is a non-issue since we already have `iadd_imm`
  canonicalized by the preopt pass, no need to re-canonicalize in lowerings.
* cfallin - still good question if we can express a family of patterns. We could
  have a macro system perhaps in the futures where one rule goes to N rules, but
  maybe too much complexity too. For now cases should be simple enough.
* fitzgen - gcc/go have macro in their DSL to be polymorphic over bit-width.
* cfallin - for that specific case we could perhaps be polymorphic on types by binding the type with a pattern variable

... more may be lost as alexcrichton had video issues and dropped ...

* cfallin - if we have few weird special cases easier to consume than a more
  general form.
* bbouvier - main concern is about the developer experience. The systems seem
  reasonable. Having a very simple tutorial is a great thing though. Seems great
  at solving 95% of the problems and for the remaining 5% you need to be an
  expert (meaning writing your own extractor), but maybe this is the right
  tradeoff? It's a regression to deal with two kinds and not see how they
  interact. Embedding concepts from one language in another can be tricky.
* cfallin - type definitions? Probably need a better story such as generating
  that automatically. Could parse Rust definitions in the future? Makes sense
  about two different languages though. I think this is a fundamental thing that
  we would buy into. ISLE has s-expressions that aren't Rust, but in exchange
  we're buying a lot of power. Once everything in ISLE we can change the
  compiler or change the trait methods and it's much easier to make more
  widespread changes. One concrete example is a migration to regalloc2. Started
  down path with handwritten code and stopped at a 4k line diff since everything
  needed to change. If we generate the code we change things once and everything
  "just works". Additionally big thing is consuming the pattern for purposes
  other than the backend such as verification or analysis. Tooling to understand
  the list of rules as data benefits from ISLE. Can theoretically do things like
  this today but can be difficult. May as well extract core rules as explicit
  list.
* bbouvier - makes sense but you can imagine that there could be some issues
  with a source-to-source compiler since you debug Rust code instead of ISLE.
  Have to understand what to do in one source to induce changes in the other.
  Lose tools like rust-analyzer with a custom language. Understand a new
  framework can enable new things but as an end-user I would lose quality of
  life a lot.
* cfallin - unsure, I hear where you're coming from in that it's a new thing
  with no tooling that Rust has for example. As debugging the best you'd get is
  what you see where rules came from. This is a fundamental tradeoff though in
  that we're paying complexity for power, but I think the power comes for the
  backend developer too. A simpler DSL than handwritten-today enables a better
  backend. SIMD in particular is very deeply nested and tangled. A series of
  patterns makes life easier as an instruction lowering engineer.
* fitzgen - you said early 95% easy and 5% expertise, I think this is a much
  better ratio than where we're currently at. I think the level of indirection
  is a hit we'll take since we aren't going to build an LSP. All the other stuff
  seems worth it to me though.
* cfallin - as a former hand-developer this is something I want, but bbouvier
  you write much code to so if you don't want this then that's just two
  anecdotal data points. Concern is can we live with it and if it's necessary
  for other things, without hindering other goals, then is it worth it?
  Subjective call.
* bbouvier - understand you want verification and such. You who are active
  maintainers makes sense you do what feels best. I'm not against this just a
  bit skeptical.
* cfallin - no asking questions is very valuable. If this isn't the right thing
  we shouldn't do this, it's an open question. I think that this is valuable
  even if we don't do things like verification, for the common case of writing
  the backend in hindsight this is what I would have wanted. I would give myself
  the ISLE compiler in 2020 if I could. May be biased now though. Can be scary
  s-expressions for newcomers, but may be able to be simpler to define new
  instructions.
* fitzgen - thanks again for asking questions and pushing, know that it's hard
  when others aren't speaking up. One thing I think may be useful would be
  thinking about if we do do ISLE what can we do to mitigate the loss in
  developer experience around these boundaries? Comments in generated source
  today but what else can we do?
* cfallin - Would second that question, open to input for ideas. Not asking for
  a vote on the RFC, we'll continue to work on the prototype and at a point not
  too far distant we'll be able to see integration with Cranelift. Hopefully
  will see a simplification and benefits can be more clear.
* akirilov - I'd like to try to implement one of the hairiest things to see how
  it goes in addition to seeing simple examples.
* fitzgen - would be good for having a deadline of deciding to do this. My plan
  is to integrate with cranelift soon, simple and hard ones. Could we come back
  and take a look and not be stuck in limbo?
* akirilov - sounds reasonable yeah. Good to have simple examples to see how
  things work and complicated ones as well to see how far we can push ISLE.
* bbouvier - yeah sounds good.
* cfallin - over time! Thanks all!
