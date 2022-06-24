# May 16 project call

**See the [instructions](../README.md) for details on how to attend**

## Agenda
1. Opening, welcome and roll call
    1. Note: meeting notes linked in the invite.
    1. Please help add your name to the meeting notes.
    1. Please help take notes.
    1. Thanks!
1. Announcements
    1. no announcements
1. Other agenda items
    1. no fixed agenda

## Notes

### Attendees

* Alexa VanHattum (avanhatt)
* Andrew Brown (abrown)
* Anton Kirilov (akirilov-arm)
* Benjamin Bouvier (bnjbvr)
* bjorn3
* Chris Fallin (cfallin)
* Johnnie Birch (jlb6740)
* Nick Fitzgerald (fitzgen)
* Ulrich Weigand (uweigand)

### Notes

* regalloc2 regressions
    * Ulrich Weigand: rebased the PR migrating call and return instructions to ISLE for s390x
    * Ulrich Weigand: on top of regalloc2 and noticed regressions in the generated code, i.e.
    * Ulrich Weigand: lower quality results
    * Ulrich Weigand: old implementation would move directly into the output register
    * Ulrich Weigand: the new one creates a new vreg that is aliased to the output register
    * Chris Fallin: would like to have a look, is there a public branch to check out
    * Chris Fallin: don't expect extra vregs and moves
    * Ulrich Weigand: also, the register allocator uses callee-saved registers instead of
    * Ulrich Weigand: caller-saved ones
    * Ulrich Weigand: the second issue is that with the previous register allocator there was
    * Ulrich Weigand: a way to influence the order in which registers are preferred
    * Ulrich Weigand: s390x uses load and store multiple that accept ranges of registers and that
    * Ulrich Weigand: become inefficient if successive registers are not used
    * Chris Fallin: random choice of registers is a deliberate design decision that led to
    * Chris Fallin: improvements on x86-64; is the issue that non-contiguous ranges are used?
    * Ulrich Weigand: yes, the code saves and restores unnecessary registers to fill in the gaps
* Status updates
    * Nick Fitzgerald: no Cranelift updates
    * Chris Fallin: regalloc2 changes, many ISLE-related things - improvements such as if-let,
    * Chris Fallin: changes to the build system (no checked-in generated source code), various small
    * Chris Fallin: fixes; the major project right now is working on the middle end, e.g. machine-
    * Chris Fallin: independent optimizations, starting with alias analysis; the hope is that by
    * Chris Fallin: using ISLE we will enable interesting optimizations such as fusion, while
    * Chris Fallin: making verification of the middle end easier
    * Alexa VanHattum: working on ISLE verification, have a PR not to inline internal constructors,
    * Alexa VanHattum: also looking into the ISA semantics of Arm and x86 to include one of them
    * Alexa VanHattum: into the verification process
    * Benjamin Bouvier: working on the incremental cache idea that has been discussed before, have a
    * Benjamin Bouvier: heavy use case for hot reload - makes things much faster; the next step is
    * Benjamin Bouvier: to open a GitHub issue for discussion; the implementation will need a
    * Benjamin Bouvier: key-value store to keep compilation artifacts
    * Ulrich Weigand: fixes for ISA feature flag handling and various logic errors in the bitwise
    * Ulrich Weigand: operation implementations
    * Andrew Brown: working mostly on the shared linear memory implementation in Wasmtime, which is
    * Andrew Brown: expected to have some impact on Cranelift
    * Johnnie Birch: started working with ISLE by implementing square root operations, looking
    * Johnnie Birch: forward to do more
    * bjorn3: looking into implementing exception handling (`eh_cleanup` branch on Wasmtime fork),
    * bjorn3: but a bit stuck at how to deal with caller-saved registers
    * Anton Kirilov: working on migrating `bitselect`, `vselect` and `splat` to ISLE; after the
    * Anton Kirilov: associated RFC has been accepted, has finalized the forward-edge CFI
    * Anton Kirilov: implementation, which is now ready for review
* sightglass discussion
    * Andrew Brown: Docker had been introduced to sightglass to facilitate reproducible builds
    * Andrew Brown: recently a PR on the handling of build metadata that failed CI testing led to
    * Andrew Brown: the idea of getting rid of Docker
    * Nick Fitzgerald: Docker is frequently a source of inconvenience
    * Chris Fallin: Docker is also Linux-specific, which might be a problem in the future if we
    * Chris Fallin: decide to support other platforms
    * Johnnie Birch: another problematic use case - internal framework to run stuff everywhere, e.g.
    * Johnnie Birch: in the cloud; it uses Docker within Docker, which has also caused trouble
    * Andrew Brown: so let's remove Docker, but would be the replacement - something to encode a
    * Andrew Brown: sequence of commands in an OS-agnostic way?
    * Chris Fallin: what are we using Docker exactly for?
    * Andrew Brown: cloning repositories and `cargo build`, but there are knobs for configuratrion
    * Andrew Brown: parameters, so that it is possible to use your repository, for example
    * Nick Fitzgerald: there are so many different ways to build something, so we should push back
    * Nick Fitzgerald: on supporting everything, just the things we really need
    * Nick Fitzgerald: How many knobs are there? If it is just a commit ID, then it is fine, but
    * Nick Fitzgerald: adding support for more than that might open a can of worms
    * Andrew Brown: repository location, commit ID, build flags
    * Nick Fitzgerald: other engines have other settings - Make flags, `configure` flags, etc.
    * Chris Fallin: environment variables as well
    * Nick Fitzgerald: is there a use case for using non-default build flags?
    * Andrew Brown: it is not huge
    * Nick Fitzgerald: the feeling is that sightglass should be simpler than it is right now
    * Nick Fitzgerald: currently we usually compare either two branches or compare commits over time
    * Nick Fitzgerald: on the other hand, we should record enough metadata, so that a suitably
    * Nick Fitzgerald: motivated individual could reproduce the environment manually
