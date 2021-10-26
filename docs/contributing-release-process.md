# Release Process

This is intended to serve as documentation for Wasmtime's release process. It's
largely an internal checklist for those of us performing a Wasmtime release, but
others might be curious in this as well!

## Releasing a major version

Major versions of Wasmtime are relased once-a-month. Most of this is automatic
and **all that needs to be done is to merge the GitHub PR that CI will
generate** on the second Monday of each month.

Specifically what happens for a major version release is:

1. One day a month (configured via `.github/workflows/bump-version.yml`) a CI job
   will run. This CI job will:
  * Download the current `main` branch
  * Run `./scripts/publish.rs` with the `bump` argument
  * Commit the changes with a special marker in the commit message
  * Push these changes to a branch
  * Open a PR with this branch against `main`
1. A maintainer of Wasmtime signs off on the PR and merges it.
  * Most likely someone will need to push updates to `RELEASES.md` beforehand.
  * A maintainer should double-check there are [no open security issues][rustsec-issues].
1. The `.github/workflow/push-tag.yml` workflow is triggered on all commits to
   `main`, including the one just created with a PR merge. This workflow will:
   * Scan the git logs of pushed changes for the special marker added by
     `bump-version.yml`.
   * If found, tags the current `main` commit and pushes that to the main
     repository.
1. Once a tag is created CI runs in full on the tag itself. CI for tags will
   create a GitHub release with release artifacts and it will also publish
   crates to crates.io. This is orchestrated by `.github/workflows/main.yml`.

If all goes well you won't have to read up much on this and after hitting the
Big Green Button for the automatically created PR everything will merrily carry
on its way.

[rustsec-issues]: https://github.com/bytecodealliance/wasmtime/issues?q=RUSTSEC+is%3Aissue+is%3Aopen+

## Releasing a patch release

Making a patch release is somewhat more manual than a major version. At this
time the process for making a patch release of `2.0.1` the process is:

1. All patch release development should be happening on a branch
   `release-2.0.1`.
  * Maintainers need to double-check that the `PUBLIC_CRATES` listed in
    `scripts/publish.rs` do not have semver-API-breaking changes (in the
    strictest sense). All security fixes must be done in such a way that the API
    doesn't break between the patch version and the original version.
1. Locally check out `release-2.0.1` and make sure you're up-to-date.
1. Run `rustc scripts/publish.rs`
1. Run `./publish bump-patch`
1. Update `RELEASES.md`
1. Commit the changes. Include the marker
   `[automatically-tag-and-release-this-commit]` in your commit message.
1. Make a PR against the `release-2.0.1` branch.
1. Merge the PR when CI is green
  * Note that if historical branches may need updates to source code or CI to
    pass itself since the CI likely hasn't been run in a month or so. When in
    doubt don't be afraid to pin the Rust version in use to the rustc version
    that was stable at the time of the branch's release.

From this point automated processes should take care of the rest of the steps,
basically resuming from step 3 above for major releases where `push-tag.yml`
will recognize the commit message and push an appropriate tag. This new tag will
then trigger full CI and building of release artifacts.
