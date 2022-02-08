# February 7 project call

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
    1. _Submit a PR to add your item here_
    2. avanhatt: ISLE rule verification update/open questions (Alexa, Fraser, Monica).

## Notes

### Attendees
- cfallin
- avanhatt
- Monica Pardeshi
- abrown
- bnjbvr
- jlbirch
- uweigand
- sparker-arm
- Fraser Brown

### Notes

avanhatt presented ISLE SMT rule verification prototype, currently type and term variants for IR-to-IR verification.

- avanhatt: open question, what should the syntax be? LHS, extractor, is more complex.
- cfallin: Feels like global type inference. Annotations on prelude or standard library, keep them out of lowering rules.
- avanhatt: Agreed, that it would be good to not have to specify downstream rules.
- cfallin: Recursion could be a problem.
- uweigand: Not sure I understand the big picture. How do we define formal semantics of backend ISAs?
- avanhatt: Current research and tools are available for x86 and Arm.
- cfallin: We don't even have a good prose descriptions of CLIF operations yet.
- avanhatt: The Alive project managed without formal semantics.
- uweigand: Will look to see if there's anything for S390.
- uweigand: Could the semantics annotations be used to generate the isel code?
- cfallin: Would depend whether it could produce efficient code, would be good for asserts though.
- avanhatt: Yes, we could use it for asserts to strengthen fuzzing.
- FB: Could be used for equivalence checks on rust code too.

# standups

- sparker-arm: Sizeless vector RFC updated, atomics isle porting.
- uweigand: Branches and traps lowering changes merged. Implemented atomics, which was complicated for narrow types, involved emitting loops.
        - sparker-arm: Would the SMT verifier be capable of handling loops?
- uweigand: Still not sure to do about call lowering, as calls can return more than two values.
        - cfallin: Should we do it?
        - uweigand: S390 ABI wants extended values and isle could be useful for this.
        - cfallin: Separate call instruction from arg/return setup..?
- uweigand: Reorganising ABICaller(Callee?) to simplify the communication between the backend.
- uweigand: Next moving onto SIMD.
- abrown: ISLE select lowering for x64, seems like more flag handling infrastructure needs to be added. Now looking at sightglass for instantiation metrics.
- jlbirch: Worked on a patch for sightglass to automatically trigger benchmarking when a patch is committed. Would like to know why this isn't a good idea for github actions.
        - cfallin: Need to trust that a PR isn't malicious.
- bnjbvr: Has been working on ittapi for cross-platform Vtune support.
        - abrown: Thanks so much for this.
- cfallin: Will be thinking about isle rule precedence.
