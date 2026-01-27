# Development Process

We use [issues] for asking questions ([open one here][newissue]!) and tracking
bugs and unimplemented features, and [pull requests] (PRs) for tracking and
reviewing code submissions. We triage new issues at each of our bi-weekly
[Wasmtime meetings][meetings].

### Before submitting a PR

Consider opening an issue to talk about it. PRs without corresponding issues
are appropriate for fairly narrow technical matters, not for fixes to
user-facing bugs or for feature implementations, especially when those features
might have multiple implementation strategies that usefully could be
discussed. Changes that will significantly affect stakeholders should first be
proposed in an [RFC](./contributing-rfc-process.md).

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
pull requests. However, one approval from a maintainer is required before a PR
can be merged. Maintainers must also create PRs and get review from another
maintainer for every change, including minor work items such as version bumps,
removing warnings, etc.

PR approvals may come with comments about additional minor changes that are
requested. Contributors and maintainers alike should address these comments, if
any, and then the PR is ready for merge. Wasmtime uses a [merge queue] to ensure
that all tests pass before pushing to `main`. Note that the [merge queue] will
run more tests than is run on PRs by default.

Contributors should expect Wasmtime maintainers to add the PR to the merge queue
for them. If a PR hasn't been added, and it's approved with all comments
addressed, feel free to leave a comment to notify maintainers that it's ready.
Maintainers can add their own PRs to the merge queue. When approving a PR
maintainers may also add the PR to the merge queue at that time if there are no
remaining comments.

Note that if CI is failing on a PR then GitHub will automatically block adding a
PR to the [merge queue]. PR authors will need to resolve PR CI before it can be
added to the merge queue. If the merge queue CI fails then the PR will be
removed from the merge queue and GitHub will leave a marker on the timeline and
send a notification to the PR author. PR authors are expected to review CI logs
and fix any failures in the PR itself. When ready maintainers can re-add their
own PR for minor fixes and contributors can leave a comment saying that the PR
is ready to be re-added to the queue.

To run full CI on the PR before the merge queue, include the string
`prtest:full` in any commit in the PR. That can help debug CI failures without
going through the merge queue if necessary.

[issues]: https://guides.github.com/features/issues/
[pull requests]: https://help.github.com/articles/about-pull-requests/
[issue keywords]: https://help.github.com/articles/closing-issues-using-keywords/
[newissue]: https://github.com/bytecodealliance/wasmtime/issues/new
[meetings]: https://github.com/bytecodealliance/meetings/tree/main/wasmtime
[merge queue]: https://github.com/bytecodealliance/wasmtime/queue/main
