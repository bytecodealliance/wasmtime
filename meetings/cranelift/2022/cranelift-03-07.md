# March 7 project call

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
    1. Verification annotation syntax (avanhatt)

## Notes

### Attendees
- uweigand
- cfallin
- avanhatt
- abrown
- fitzgen
- jlbirch
- Fraser Brown
- bjorn3
- bnjbvr
- kulakowski-wasm
- akirilov
- Adrian Sampson

### Notes

- Verification annotation syntax (avanhatt)
    - quick presentation
        - Syntax proposal for annotations
	        - Comments for individual isle turns
	        - Verification will use annotations if they exist
    	    - Can reuse and infer types, or require a new explicit syntax for types and variable bindings
    	- Common assumptions specify return values, more sophisticated ones can handle e.g. failiable operations
    	- Assumptions passed through to the verification IR, which is slightly higher level than SMT per se
    - fitzgen: Are these always applied to extractors or constructors
    - avanhatt: Applied to both
    - cfallin: I would expect it to be the same for extractors or constructors, it aligns with the goals of rewriting engines
    - fitzgen: Re not repeating ourselves: I think it makes sense to infer types from ISLE parameter types
    - cfallin: I would refine that and say: 1 place and not 0 places. There should be a declaration somewhere
    - fitzgen: Right, I think on the type declarations
    - bjorn3: It would be nice if this was in ISLE instead of comments
    - avanhatt: At what point do we merge this prototype? Maybe it makes sense then to transition from special comments to real ISLE syntax
    - cfallin: Feel free to hack up our parser in your branch, and we could take a change for general annotation syntax out of it too.
    - fitzgen: I dunno how much we want to really copy that, I find it a bit over-engineered. But we could have arbitrary S-expression tags
    - cfallin: I had a question about fallibility on the conceptual level. Is it that : `assumption` encodes the postcondition if the match succeeds? Or is it encoding the condition for the match to succeed at all? I’d expect the first
    - avanhatt: Yes, the first.
    - cfallin: One thing that would be nice to reason about with fallibility is coverage, knowing that our conditions actually cover all cases. Is there a plan to have that?
    - avanhatt: Set up for it and have talked about it, no actual plan for it. Dovetails with multi rule reasoning
    - cfallin: Is there a mechanism for abstraction? “Helper functions” for similar operations
    - avanhatt: Yeah, I think we should. In particular there’s lots of noops. I think we should, but there’s no concrete proposal for it yet.
    - fitzgen: I think we talked about this on Zulip, but the representation for bitwidth is the same on both left and right hand sides? So we don’t have the ability to represent undefined bits in eg lowering registered width
    - avanhatt: Would be cool to do, aren’t doing it yet, can extend to do it.
    - cfallin: Would prefer to not special case things, I’d rather have an S-expression syntax
    - bjorn3: Also need to think about branches and jumps, take variable amounts of arguments

- Standups
    - cfallin
	    - I have been working on ISLE translations. Week before last, was deep in the x64 backend translating arithmetic ops into ISLE. I plan to keep pushing on that when I don’t have other cranelift related tasks as a background thing. I realized that this ISLE translation is not actually on the critical path to regalloc2, as far as I can see: 1 earlier version of regalloc2 which I tried porting Cranelift to only accepted SSA and 2 regalloc2 now has shims for non-ssa code. Can combine with the x64 mov mitosis mechanism for the 2-operand form, and actually land regalloc2 and get rid of lots of movs on the way. Several kilo-lines of code diff, but should be a perf win in compilation time and runtime
    - uweigand
	    - Merged 2 bits of s390 infrastructure, for variable number of arguments and of return values. Both prereqs for calls and returns, and some other variadic operations. Patch outstanding does now work. Would appreciate feedback on this ISLE-driven approach and where all the knowledge about ABI lives (cfallin will take a closer look soon)
        - Also looking at SIMD support, and fixing up ABI support for eg callee saved fp-and-vector registers (cfallin: look at aarch64 which has a similar property re clobbers). Also a big discussion on backwards compatible vector register extensions
    - abrown
	    - Worked on ISLE select lowering, was good and educational. Was going to do integer comparisons and felt it was taking too long around i128. Wondering if we should do it later and focus now on more ordinary register operations.
	        - fitzgen and cfallin: yeah fine as long as we are in working states along the way.
    - akirilov
	    - Pointer verification (mostly) landed in rust, and I kept working on my proposal to align it with what’s in the rust compiler, and hope to have something to land in the next few days.
    - jlbirch
	    - Worked on CI integration, acritchon had a bunch of suggestions around ACLs etc.
    - fitzgen, kulakowski-wasm, Adrian, Fraser, avanhatt, bnjbvr
	    - No other updates
