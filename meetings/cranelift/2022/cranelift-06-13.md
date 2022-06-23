# June 13 project call

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
    1. bnjbvr: Maintainance and code ownership of cranelift-jit-demo (e.g. [this PR](https://github.com/bytecodealliance/cranelift-jit-demo/pull/66))

## Notes

### Attendees

- abrown
- akirilov
- avanhatt
- bjorn3
- bnjbvr
- cfallin
- fitzgen
- jlbirch
- sparker-arm

### Notes

Agenda item:

- bnjbvr: who owns the cranelift-jit-demo repo? is it collective responsibility? meta
  question: how do we decide that in general?
- cfallin: meta question, probably a bytecode alliance thingy, RFC to discuss.
  For this particular repo, would github's CODEOWNERS work?
- bnjbvr: explains how CODEOWNERS works. Not a solution for merge right. Github
  has different allowance access for individuals, so could give merge rights to
  a specific individual.
- cfallin: either open an RFC, or discuss this at next wasmtime meeting

Updates:

- sparker-arm: aarch64 vector work, benchmarking, moving along well, no PR opened yet
- cfallin: worked with egraph, subsumes GVN now, LICM soon. Question is how
  does the rewrite system look like? Now that something works, write up an RFC
  and see what people think.
- avanhatt: lots of verification updates, half way reviewing egraph PR
- bnjbvr: work paused the last two weeks on incremental cache, back to it this week
- akirilov: PAC (pointer authentication) work in fiber has been merged, CI uses PAC now, RFC + patch to
  be updated soon
    - cfallin: that's on linux aarch64, how far are we from enabling for mac
      m1?
    - akirilov: codegen changes was easy, unwinding harder, not sure about
      the complexity for mac m1.
    - bjorn3: mac m1's ABI is slightly different from linux aarch64's one
- jlbirch: talked about security concerns with Alex re: sightglass /
  benchmarking.
    - cfallin: how will this work? bot? manual trigger?
    - jlbirch: anyone with sufficient permissions can comment/open issue to run
      a workflow run (repository dispatch event), private repo will run the
      PRs, do the testing and send the results back to the PR/issue. This will
      be running on private machines (linux x64, linux aarch64).
- sparker-arm: limit egraph optimizations, how does it work?
    - cfallin: fuel mechanism to limit # (number of) rules of application, metric on
      memory usage (# nodes, classes), hard cap this to some multiple of #
      cranelift input nodes.
    - sparker-arm: (*notetaker missed that question*)
    - cfallin: no rewrites at all at the moment, just hash-const. Turn up knob to
      get several optimization rounds.
    - avanhatt: would we do inter-procedural analysis in the future? ie allow
      rules to rewrite across function boundaries somehow?
    - cfallin: prob not. Could blow up memory/time, so we'd need to explore.
      We'd do that only if we inlined that callsite already (so not across
      function boundaries)
