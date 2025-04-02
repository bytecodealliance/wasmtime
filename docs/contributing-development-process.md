# Development Process

We use [issues] for asking questions ([open one here][newissue]!) and tracking
bugs and unimplemented features, and [pull requests] (PRs) for tracking and
reviewing code submissions. We triage new issues at each of our bi-weekly
[Wasmtime meetings][meetings].

### Before submitting a PR

Consider opening an issue to talk about it. PRs without corresponding issues
are appropriate for fairly narrow technical matters, not for fixes to
user-facing bugs or for feature implementations, especially when those features
might have multiple implementation strategies that usefully could be discussed.

Our issue templates might help you through the process.

### When submitting PRs

 - Please answer the questions in the pull request template. They are the
   minimum information we need to know in order to understand your changes.

 - Write clear commit messages that start with a one-line summary of the
   change (and if it's difficult to summarize in one line, consider
   splitting the change into multiple PRs), optionally followed by
   additional context. Good things to mention include which areas of the
   code are affected, which features are affected, and anything that
   reviewers might want to pay special attention to.

 - If there is code which needs explanation, prefer to put the explanation in a
   comment in the code, or in documentation, rather than in the commit message.
   Commit messages should explain why the new version is better than the old.

 - Please include new test cases that cover your changes, if you can. If you're
   not sure how to do that, we'll help you during our review process.

 - For pull requests that fix existing issues, use [issue keywords]. Note that
   not all pull requests need to have accompanying issues.

 - When updating your pull request, please make sure to re-request review if
   the request has been cancelled.

### Focused commits or squashing

We are not picky about how your git commits are structured. When we merge your
PR, we will squash all of your commits into one, so it's okay if you add fixes
in new commits.

We appreciate it if you can organize your work into separate commits which each
make one focused change, because then we can more easily understand your
changes during review. But we don't require this.

Once someone has reviewed your PR, it's easier for us if you _don't_ rebase it
when making further changes. Instead, at that point we prefer that you make new
commits on top of the already-reviewed work.

That said rebasing (or merging from `main`) may still be required in situations
such as:

* Your PR has a merge conflict with the `main` branch.
* CI on your PR is failing for unrelated reasons and a fix was applied to `main`
  which needs to be picked up on your branch.
* Other miscellaneous technical reasons may cause us to ask for a rebase.

If you need help rebasing or merging, please ask!

### Review and merge

Anyone may submit a pull request, and anyone may comment on or review others'
pull requests. However, one review from somebody in the [Core Team] is required
before the Core Team merges it.

Even Core Team members must create PRs and get review from another Core Team
member for every change, including minor work items such as version bumps,
removing warnings, etc.

[issues]: https://guides.github.com/features/issues/
[pull requests]: https://help.github.com/articles/about-pull-requests/
[issue keywords]: https://help.github.com/articles/closing-issues-using-keywords/
[Core Team]: https://github.com/orgs/bytecodealliance/people/
[newissue]: https://github.com/bytecodealliance/wasmtime/issues/new
[meetings]: https://github.com/bytecodealliance/meetings/tree/main/wasmtime
