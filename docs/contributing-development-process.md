# Development Process

We use [issues] for asking questions ([open one here][newissue]!) and tracking
bugs and unimplemented features, and [pull requests] (PRs) for tracking and
reviewing code submissions.

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
[newissue]: https://github.com/bytecodealliance/wasmtime/issues/new
