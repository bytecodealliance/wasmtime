# Contributing to Cranelift

## Welcome!

Cranelift is a very ambitious project with many goals, and while we're
confident we can achieve some of them, we see many opportunities for people
to get involved and help us achieve even more.

### Ask questions! Give feedback!

This is a relatively young project, and not everything we hope to do with it
is reflected in the code or documentation yet. If you see things that seem
missing or that don't make sense, or even that just don't work the way you
expect them to, we're interested to hear about it!

We have a [stream] in the Bytecode Alliance Zulip instance, and questions are
also welcome as issues in the [Cranelift issue tracker].

[stream]: https://bytecodealliance.zulipchat.com/#narrow/stream/217117-cranelift/topic/general
[Cranelift issue tracker]: https://github.com/bytecodealliance/cranelift/issues/new

### Mentoring

We're happy to mentor people, whether you're learning Rust, learning about
compiler backends, learning about machine code, learning about how Cranelift
does things, or all together at once.

We categorize issues in the issue tracker using a tag scheme inspired by
[Rust's issue tags]. For example, the [E-easy] marks good beginner issues,
and [E-rust] marks issues which likely require some familiarity with Rust,
though not necessarily Cranelift-specific or even compiler-specific
experience. [E-compiler-easy] marks issues good for beginners who have
some familiarity with compilers, or are interested in gaining some :-).

See also the [full list of labels].

Also, we encourage people to just look around and find things they're
interested in. This a good time to get involved, as there aren't a lot of
things set in stone yet.

[Rust's issue tags]: https://github.com/rust-lang/rust/blob/master/CONTRIBUTING.md#issue-triage
[E-easy]: https://github.com/bytecodealliance/cranelift/labels/E-easy
[E-rust]: https://github.com/bytecodealliance/cranelift/labels/E-rust
[E-compiler-easy]: https://github.com/bytecodealliance/cranelift/labels/E-compiler-easy
[full list of labels]: https://github.com/bytecodealliance/cranelift/labels

### Code of Conduct

Cranelift is a [Bytecode Alliance] project, and follows the Bytecode Alliance's [Code of Conduct] and [Organizational Code of Conduct].

[Bytecode Alliance]: https://bytecodealliance.org/
[Code of Conduct]: CODE_OF_CONDUCT.md
[Organizational Code of Conduct]: ORG_CODE_OF_CONDUCT.md

## Coding Guidelines

For the most part, Cranelift follows common Rust conventions and
[pull request] (PR) workflows, though we do have a few additional things to
be aware of.

[pull request]: https://help.github.com/articles/about-pull-requests/

### rustfmt

All PRs must be formatted according to rustfmt, and this is checked in the
continuous integration tests. We use the current stable [rustfmt-preview]
version. See the [rustfmt quickstart] for setup.

[format-all.sh] is a script for running the appropriate version of rustfmt,
which may be convenient when there are multiple versions installed.

[rustfmt-preview]: https://github.com/rust-lang/rustfmt
[rustfmt quickstart]: https://github.com/rust-lang/rustfmt#quick-start
[format-all.sh]: https://github.com/bytecodealliance/cranelift/blob/master/format-all.sh

### Rustc version support

Cranelift supports stable Rust, and follows the
[Rust Update Policy for Firefox].

Some of the developer scripts depend on nightly Rust, for example to run
clippy and other tools, however we avoid depending on these for the main
build.

[Rust Update Policy for Firefox]: https://wiki.mozilla.org/Rust_Update_Policy_for_Firefox#Schedule

## Development Process

We use [issues] for asking questions and tracking bugs and unimplemented
features, and [pull requests] (PRs) for tracking and reviewing code
submissions.

### Before submitting a PR

Consider opening an issue to talk about it. PRs without corresponding issues
are appropriate for fairly narrow technical matters, not for fixes to
user-facing bugs or for feature implementations, especially when those features
might have multiple implementation strategies that usefully could be discussed.

Our issue templates might help you through the process.

### When submitting PRs

 - Please fill in the pull request template as appropriate. It is usually
   helpful, it speeds up the review process and helps understanding the changes
   brought by the PR.

 - Write clear commit messages that start with a one-line summary of the
   change (and if it's difficult to summarize in one line, consider
   splitting the change into multiple PRs), optionally followed by
   additional context. Good things to mention include which areas of the
   code are affected, which features are affected, and anything that
   reviewers might want to pay special attention to.

 - If there is code which needs explanation, prefer to put the explanation in
   a comment in the code, or in documentation, rather than in the commit
   message.

 - For pull requests that fix existing issues, use [issue keywords]. Note that
   not all pull requests need to have accompanying issues.

 - Assign the review to somebody from the [Core Team], either using suggestions
   in the list proposed by Github, or somebody else if you have a specific
   person in mind.

 - When updating your pull request, please make sure to re-request review if
   the request has been cancelled.

### Focused commits or squashing

We generally squash sequences of incremental-development commits together into
logical commits (though keeping logical commits focused). Developers may do
this themselves before submitting a PR or during the PR process, or Core Team
members may do it when merging a PR. Ideally, the continuous-integration tests
should pass at each logical commit.

### Review and merge

Anyone may submit a pull request, and anyone may comment on or review others'
pull requests. However, one review from somebody in the [Core Team] is required
before the Core Team merges it.

Even Core Team members should create PRs for every change, including minor work
items (version bump, removing warnings, etc.): this is helpful to keep track of
what has happened on the repository. Very minor changes may be merged without a
review, although it is always preferred to have one.

[issues]: https://guides.github.com/features/issues/
[pull requests]: https://help.github.com/articles/about-pull-requests/
[issue keywords]: https://help.github.com/articles/closing-issues-using-keywords/
[Core Team]: https://github.com/orgs/bytecodealliance/people/
