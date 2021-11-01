# November 1 project call

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
    1. (fitzgen) ISLE presentation; discussion; decision.
       * [slides](https://docs.google.com/presentation/d/1b6psjaIKkVTZGaGzMcw9h_Ahgih-ctUcnz0Dbjn11GI/edit)
    3. _Sumbit a PR to add your item here_

### Attendees

* cfallin
* fitzgen
* sparker
* akirilov
* acrichto
* jlbirch
* bjorn3

### Notes

#### ISLE

* cfallin: one item for today, Nick's presentation on ISLE!
* fitzgen: ... makes presentation ...
* cfallin: main result now is whether or not we start the FCP process, any
  showstopper concerns at this point?
* jlbirch: what happens next if we move forward?
* cfallin: there's a section in the RFC, but the idea is that at the top level
  ISLE is used and if it returns `None` use what's there today, a top-level
  switch. We'll merge the framework and write more lowering in the DSL over
  time, but the old backend is there the whole time. Everything can still be
  handled because it's the fallback case. When the last logic is added to ISLE
  we can delete the old backend as it's not used.
* jlbirch: would be great to compare and have baseline yeah
* bjorn3: no new build deps?
* cfallin: yes will check in build code. I will make a motion to finalize and
  will need another group to +1. Once RFC has merged we can merge the prototype
  and go from there. Any objections?

#### Status

* cfallin: no updates
* fitzgen: my presentation!
* jlbirch: no updates
* akirilov: working on prototype for CFI. Function prologues now need to be
  ISA-specific and requires changes. Working on getting all tests passing with
  prototype.
* acrichto: ... questions about CFI and Rust implications ...
* sparker: working on flexible vectors, got an example working with splat + add.
  Some questions! Hard limit of 256 lanes?
* cfallin: probably an efficient representation? Likely predates much of us. Is
  there a statically defined but arbitrary vector size? or dynamic?
* sparker: proposal should be dynamic as a value in SSA form. Implemeneted for
  AArch64 so just return 128 sizes. Wondering if I can take a bit to represent a
  dynamic vector.
* cfallin: seems unlikely to have more than 256 lanes so seems fine.
* sparker: looks like an opaque vector and the return value is that it's a
  size-less vector.
* cfallin: seems safe because lots of code may cast. Arbitrary size so is it in
  a register?
* sparker: it's in a register and gets weird if you do lane operations. Special
  immediate type there. Can't do some things you can do in fixed-width nicely.
* cfallin: feels like an arbitrarily sized types. Seems more like a struct than
  a fixed-width vector. Feels right to make it a new kind of thing though.
* akirilov: this sounds like a similar situation in LLVM
* sparker: represented as a vector in LLVM so IR shows vector code and the width
  is the minimum width. Flexible vectors aren't well-defined really.
* cfallin: advantage of using existing infrastructure would be for future use
  with tiling/blocking but unsure if it's useful enough for now.
* sparker: looking to not have to update all the "is this a vector" blocks
* cfallin: seems like the right thing to do for prototyping and can figure out
  more details later.
* bjorn3: no updates
* cfallin: I will motion to finalize the ISLE RFC. That should be it!
