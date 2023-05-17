# Code Review

We only merge changes submitted as GitHub Pull Requests, and only after they've
been approved by at least one Core Team reviewer who did not author the PR. This
section covers expectations for the people performing those reviews. These
guidelines are in addition to expectations which apply to everyone in the
community, such as following the Code of Conduct.

It is our goal to respond to every contribution in a timely fashion. Although we
make no guarantees, we aim to usually provide some kind of response within about
one business day.

That's important because we appreciate all the contributions we receive, made by
a diverse collection of people from all over the world. One way to show our
appreciation, and our respect for the effort that goes into contributing to this
project, is by not keeping contributors waiting. It's no fun to submit a pull
request and then sit around wondering if anyone is ever going to look at it.

That does not mean we will review every PR promptly, let alone merge them. Some
contributions have needed weeks of discussion and changes before they were ready
to merge. For some other contributions, we've had to conclude that we could not
merge them, no matter how much we appreciate the effort that went into them.

What this does mean is that we will communicate with each contributor to set
expectations around the review process. Some examples of good communication are:

- "I intend to review this but I can't yet. Please leave me a message if I
  haven't responded by (a specific date in the near future)."

- "I think (a specific other contributor) should review this."

- "I'm having difficulty reviewing this PR because of (a specific reason, if
  it's something the contributor might reasonably be able to help with). Are you
  able to change that? If not, I'll ask my colleagues for help (or some other
  concrete resolution)."

If you are able to quickly review the PR, of course, you can just do that.

You can find open Wasmtime pull requests for which your review has been
requested with this search:

<https://github.com/bytecodealliance/wasmtime/pulls?q=is:open+type:pr+user-review-requested:@me>

## Auto-assigned reviewers

We automatically assign a reviewer to every newly opened pull request. We do
this to avoid the problem of diffusion of responsibility, where everyone thinks
somebody else will respond to the PR, so nobody does.

To be in the pool of auto-assigned reviewers, a Core Team member must commit to
following the above goals and guidelines around communicating in a timely
fashion.

We don't ask everyone to make this commitment. In particular, we don't believe
it's fair to expect quick responses from unpaid contributors, although we
gratefully accept any review work they do have time to contribute.

If you are in the auto-assignment pool, remember: **You are not necessarily
expected to review the pull requests which are assigned to you.** Your only
responsibility is to ensure that contributors know what to expect from us, and
to arrange that _somebody_ reviews each PR.

We have several different teams that reviewers may be auto-assigned from. You
should be in teams where you are likely to know who to re-assign a PR to, if you
can't review it yourself. The teams are determined by the `CODEOWNERS` file at
the root of the Wasmtime repository. But despite the name, membership in these
teams is _not_ about who is an authority or "owner" in a particular area. So
rather than creating a team for each fine-grained division in the repository
such as individual target architectures or WASI extensions, we use a few
coarse-grained teams:

- [wasmtime-compiler-reviewers][]: Cranelift and Winch
- [wasmtime-core-reviewers][]: Wasmtime, including WASI
- [wasmtime-fuzz-reviewers][]: Fuzz testing targets
- [wasmtime-default-reviewers][]: Anything else, including CI and documentation

[wasmtime-compiler-reviewers]: https://github.com/orgs/bytecodealliance/teams/wasmtime-compiler-reviewers
[wasmtime-core-reviewers]: https://github.com/orgs/bytecodealliance/teams/wasmtime-core-reviewers
[wasmtime-fuzz-reviewers]: https://github.com/orgs/bytecodealliance/teams/wasmtime-fuzz-reviewers
[wasmtime-default-reviewers]: https://github.com/orgs/bytecodealliance/teams/wasmtime-default-reviewers

Ideally, auto-assigned reviewers should be attending the regular Wasmtime or
Cranelift meetings, as appropriate for the areas they're reviewing. This is to
help these reviewers stay aware of who is working on what, to more easily hand
off PRs to the most relevant reviewer for the work. However, this is only
advice, not a hard requirement.

If you are not sure who to hand off a PR review to, you can look at GitHub's
suggestions for reviewers, or look at `git log` for the paths that the PR
affects. You can also just ask other Core Team members for advice.

## General advice

This is a collection of general advice for people who are reviewing pull
requests. Feel free to take any that you find works for you and ignore the rest.
You can also open pull requests to suggest more references for this section.

[The Gentle Art of Patch Review][gentle-review] suggests a "Three-Phase
Contribution Review" process:

[gentle-review]: https://sage.thesharps.us/2014/09/01/the-gentle-art-of-patch-review/

1. Is the idea behind the contribution sound?
2. Is the contribution architected correctly?
3. Is the contribution polished?

Phase one should be a quick check for whether the pull request should move
forward at all, or needs a significantly different approach. If it needs
significant changes or is not going to be accepted, there's no point reviewing
in detail until those issues are addressed.

On the other end, it's a good idea to defer reviewing for typos or bikeshedding
about variable names until phase three. If there need to be significant
structural changes, entire paragraphs or functions might disappear, and then any
minor errors that were in them won't matter.

The full essay has much more advice and is recommended reading.
