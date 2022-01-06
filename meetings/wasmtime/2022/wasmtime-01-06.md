# January 6th Wasmtime project call

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

## Notes

### Attendees

- abrown
- fitzgen
- acrichton
- cfallin
- LGR
- bnjbvr

### Notes

- nothing on the agenda, so random topics were brought up
- acrichton expressing appreciation of a really bad bug found by fuzzers,
  thanks cfallin for setting that up!
- abrown asking if new ittapi-rs worked out well in Embark's embedding
    - bnjbvr: yes! passes cargo deny now
    - bnjbvr: wasmtime with vtune throws a runtime error on windows, any idea
      why?
    - abrown: will check with johnnie
- bnjbvr: we've tried enabling wasm SIMD in our embedding! Everything seemed to work fine, except for a small bug. Probably
  related to a lib using wasm SIMD intrinsics, investigating.
- fitzgen: what's up with rustix not compiling on nightly?
    - old compile bug in rustix that's been fixed there, not updated upstream
    - fitzgen may follow up with patch bumping the version in wasmtime
