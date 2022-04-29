# April 28th Wasmtime project call

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

- sunfishcode
- fitzgen
- LGR
- abrown
- jlbirch
- gkulakowski
- akirilov
- acrichton
- cfallin
- npmccallum

### Notes

- wasi-common polling interface
    - Nathaniel: can we add a custom poller interface, maybe based on traits,
      to wasi-common? Need this to virtualize a TLS socket fd in Enarx: polling
      on underlying fd may cause spurious wakeups otherwise (bytes for TLS
      layer do not necessarily become ready bytes for user).
    - Dan: yes, interested in this general discussion/idea. One thing to note
      is wasi-common will be rewritten soon in terms of wit-bindgen and
      streams, etc (more writeup/details to come). Short-term fix: use a Unix
      pipe or socket pair?
    - Nathaniel: actually we can't do that in our situation, trust boundary
      issue: kernel not allowed to see plaintext, it stays within confidential
      sandbox.
    - Dan: Cargo feature to add custom code? Also, maybe we can do a dedicated
      meeting to brainstorm.
    - Nathaniel: sure, happy to meet. Basic high-level idea to keep in mind is
      not to assume that hostcalls at WASI layer go to kernel; current impl is
      written in a way that assumes it is a thin wrapper around kernel.
    - Dan: streams in wit-bindgen
    - Nathaniel: (more ideas about trait design, missed some details)
    - Dan: interesting question in general: how do we do a general poll that
      polls over real IO and also synthesized/virtualized IO all at once?
    - Nathaniel: trait approach would allow per-platform default impl, and
      maybe people would wrap this with their own custom stuff
    - George: to Dan: is async/wit-bindgen refactor written up?
    - Dan: working on it now! Will post it soon.
