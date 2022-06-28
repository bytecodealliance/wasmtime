# June 27 project call

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

### Attendees

- abrown
- cfallin
- fitzgen
- afonso360
- akirilov-arm
- bjorn3
- Jamey Sharp
- jlbirch6740
- uweigand

### Notes

No published agenda, proceeding to status updates:

fitzgen: have been looking into stack walking; will eventually look into
unwinding as a part of exception handling; sync up with bjorn3 to row in the same
direction

afonso360: (welcome back!) have continued work on Cranelift interpreter,
starting to figure out ISLE, posted several aarch64 patches

uweigand: out on vacation, upstreamed some changes to QEMU to fix breakage with
Wasmtime in v7.0 (fix is in 7.1); also, merging a change to add a "build
Wasmtime regression test" to QEMU's CI; planning on moving to vector
registers/instructions in s390x backend

bjorn3: no updates

Jamey Sharp: listening in, no updates

akirilov: CFI final version is up for review--feedback needed; started porting
splat to ISLE for aarch64

abrown: some sightglass investigation with Yury (performance analysis Chris
might be interested in); continued shared memory changes in Wasmtime

jlbirch6740: discussed benchmarking infrastructure for CI with fitzgen and
abrown, still need to publish PR; submitted a PR to fix a profiling flag in the
C API

cfallin: out sick, investigated performance problem re: splitting in regalloc2
brought by alexcrichton, want to review and merge before next release; also is
investigating a regalloc2 checker violation reported by bjorn3 re: pinned vregs
(high priority fix!); eventual plan would be to kill pinned vregs and use
operands with constraints instead; finishing up e-graphs RFC (will post an
initial PR soon)
