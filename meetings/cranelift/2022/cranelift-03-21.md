# March 21 project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
1. Other agenda items


### Attendees

Andrew Brown
George Kulakowski
Sam Parker-Haynes
Alexa VanHattum
Johnnie Birch
bjorn3

### Notes

AB: merged several PRs related to x64 ISLE lowering; one of them exposed a regalloc bug during fuzzing, looking into it.

GK: getting familiar with the Wasm ecosystem (e.g., Cranelift); read Chris' status about the move to regalloc2 (#3942) which, in summary, is progressing but needs some more refactoring before review.

SP: coming back from vacation, starting to look at aarch64 backend again; continuing work on flexible vectors, working through issues with fully dynamic types in the IR in a way that does not require too many changes to the RFC proposal.

AV: working on IR for verification, hoping to handle multiple rule chains soon.

JB: needs to get a patch ready for review; planning to continue ISLE lowering.

bjorn3: no comments

SP: with the regalloc2 changes, will we need to make changes to the backends?

GK: probably not...
