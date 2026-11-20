# Contributing

We're excited to work on Wasmtime and/or Cranelift together with you! This guide
should help you get up and running with Wasmtime and Cranelift development. But
first, make sure you've read the [Code of Conduct](./contributing-coc.md)!

Wasmtime and Cranelift are very ambitious projects with many goals, and while
we're confident we can achieve some of them, we see many opportunities for
people to get involved and help us achieve even more.

## AI Tool Use Policy

The Bytecode Alliance has an [AI Tool Use
Policy](https://github.com/bytecodealliance/governance/blob/main/AI_TOOL_POLICY.md)
that applies to the Wasmtime project. Please read and understand this policy
before making any contributions.

## Approved Contributors

The Wasmtime project has adopted [home-grown automation] that closes PRs
opened by unknown contributors. This automation was reluctantly adopted by
Wasmtime project maintainers to stem the flood of AI-generated PRs that have
made outsize demands on the limited time and energy we have for code review.

The Wasmtime project maintainers believe the most important thing our project
creates is a community of humans who understand the Wasmtime project's code,
the project's goals, and the context the project exists in for embedders and
users. The code in the Wasmtime repository is second in importance behind that
community. Wasmtime's community has continuity back to 2017, and fostering
this community was a major reason the [Bytecode Alliance] was created.

We have always welcomed humans to join the Wasmtime project's community.
However, with the advent of AI tools, we can no longer assume that opening a
PR means there is a human behind it. Unfortunately, a large and growing
portion of the PRs opened in 2026 have been [extractive contributions], which
cost our maintainers time and energy, and diminish our community. The
project's maintainers all have numerous responsibilities besides reviewing
PRs, so we want to focus our time and energy on reviewing PRs from people who
demonstrate they have thought about our project.

If you are a human who would like to contribute to Wasmtime, we're asking that
you start a conversation in a suitable channel of the [Bytecode Alliance Zulip]
and describe your goals and what sort of contribution you'd like to make. When
doing so, please don't use any kind of AI-generated text. (Using an AI tool to
translate from another language to English is allowed, but only if you are
writing the original text yourself.) We don't want this to be a super high bar
for anyone to clear, just demonstrate to us that you are putting the sort of
mental effort into understanding our project and your contribution that we
could take for granted when receiving a PR in the pre-AI era.

Once you have engaged with the Wasmtime community on Zulip and a maintainer has
asked you to open a PR, that maintainer can leave a `/vouch` comment on the PR,
which will have our automation re-open your PR (if it was closed) and add
you to our approved contributor allowlist.

[home-grown automation]: https://github.com/bytecodealliance/approved-contributors
[Bytecode Alliance]: https://bytecodealliance.org
[extractive contributions]: https://github.com/bytecodealliance/governance/blob/main/AI_TOOL_POLICY.md#extractive-contributions
[Bytecode Alliance Zulip]: https://bytecodealliance.zulipchat.com/

## Join Our Chat

We chat about Wasmtime and Cranelift development on Zulip &mdash; [join
us!](https://bytecodealliance.zulipchat.com/). You can also join specific
streams:

* [#wasmtime](https://bytecodealliance.zulipchat.com/#narrow/stream/217126-wasmtime)
* [#cranelift](https://bytecodealliance.zulipchat.com/#narrow/stream/217117-cranelift)

If you're having trouble building Wasmtime or Cranelift, aren't sure why a test
is failing, or have any other questions, feel free to ask on Zulip. Not
everything we hope to do with these projects is reflected in the code or
documentation yet, so if you see things that seem missing or that don't make
sense, or even that just don't work the way you expect them to, we're also
interested to hear about that!

As always, you're more than welcome to [open an
issue](https://github.com/bytecodealliance/wasmtime/issues/new) too!

Finally, we have biweekly project meetings, hosted on Zoom, for Wasmtime and
Cranelift. For more information, see our [meetings agendas/minutes
repository](https://github.com/bytecodealliance/meetings). Please feel free to
contact us via Zulip if you're interested in joining!

## Finding Something to Hack On

If you're looking for something to do, these are great places to start:

* [Issues labeled "good first
  issue"](https://github.com/bytecodealliance/wasmtime/labels/good%20first%20issue)
  &mdash; these issues tend to be simple, what needs to be done is well known,
  and are good for new contributors to tackle. The goal is to learn Wasmtime's
  development workflow and make sure that you can build and test Wasmtime.

* [Issues labeled "help
  wanted"](https://github.com/bytecodealliance/wasmtime/labels/help%20wanted)
  &mdash; these are issues that we need a little help with!

If you're unsure if an issue is a good fit for you or not, feel free to ask in a
comment on the issue, or in chat.

### Mentoring

We're happy to mentor people, whether you're learning Rust, learning about
compiler backends, learning about machine code, learning about wasm, learning
about how Cranelift does things, or all together at once.

We categorize issues in the issue tracker using a tag scheme inspired by
[Rust's issue tags]. For example, the [E-easy] marks good beginner issues,
and [E-rust] marks issues which likely require some familiarity with Rust,
though not necessarily Cranelift-specific or even compiler-specific
experience. [E-compiler-easy] marks issues good for beginners who have
some familiarity with compilers, or are interested in gaining some :-).

See also the [full list of Cranelift labels].

Also, we encourage people to just look around and find things they're
interested in. This a good time to get involved, as there aren't a lot of
things set in stone yet.

### Avoiding Trivial Pull Requests

Merging a Pull Request comes at a fundamental cost, from adding to the volume
of notifications maintainers receive, to at least one maintainer having to
review the proposed changes, to the many hours of machine time our CI consumes
before greenlighting a PR.

As such, we request that contributors don't open Pull Requests with trivial
changes, such as typo fixes in documentation. Instead, those changes should be
included, as separate commits, in more substantive Pull Requests. Wasmtime
maintainers will close Pull Requests containing only trivial changes with a
reference to this section of the docs.

[Rust's issue tags]: https://github.com/rust-lang/rust/blob/master/CONTRIBUTING.md#issue-triage
[E-easy]: https://github.com/bytecodealliance/wasmtime/labels/cranelift%3AE-easy
[E-rust]: https://github.com/bytecodealliance/wasmtime/labels/cranelift%3AE-rust
[E-compiler-easy]: https://github.com/bytecodealliance/wasmtime/labels/cranelift%3AE-compiler-easy
[full list of Cranelift labels]: https://github.com/bytecodealliance/wasmtime/labels?q=cranelift
