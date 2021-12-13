# December 13 project call

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
    1. _Sumbit a PR to add your item here_

## Notes

### Attendees

* Chris Fallin
* Nick Fitzgerald
* Alexa VanHattum
* Ulrich Weigand
* Anton Kirilov
* Johnnie Birch
* Benjamin Bouvier

### Notes

* Chris Fallin:
  * project update blog post is out
  * 2022 roadmap is out, please leave comments / suggestions / feedback on the
    RFC
* Nick Fitzgerald:
  * ISLE porting
  * wasm-mutate stuff, lots of "good first issue" type stuff if people want to
    contribute!
* Alexa VanHattum:
  * Been looking into verifying ISLE
  * `iadd`s that compile down into `sub`s
  * SAIL(sp?) seems easier to work with in aarch64 than x86-64, probably
    starting there
* Ulrich Weigand
  * Looking into ISLE
* Anton Kirilov
  * Finished an initial pointer auth impl in aarch64 backend
  * Going to start working on BTI support
  * Issues with A vs B keys and unwinder
  * Current pauth prototype doesn't use nop-space instructions
  * Need to figure out how this integrates with Wasmtime's fibers
    * Apple ABI docs suggest that you can just find the return pointer and frame
      pointer as long as you don't keep arbitrary code pointers in registers
* Johnnie Birch
  * Doing a little benchmarking work
  * Triaging and cleaning up old issues, could use some help from anyone who has
    time
* Benjamin Bouvier
  * No updates
* Andrew Brown (via Johnnie/Chris)
  * Working on the ISLE lowering for `select` on x64
