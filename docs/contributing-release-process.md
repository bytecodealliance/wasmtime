# Release Process

This is intended to serve as documentation for Wasmtime's release process. It's
largely an internal checklist for those of us performing a Wasmtime release, but
others might be curious in this as well!

## Releasing a major version

Major versions of Wasmtime are released once-a-month. Most of this is automatic
and all that needs to be done is to merge GitHub PRs that CI will
generate. At a high-level the structure of Wasmtime's release process is:

* On the 5th of every month a new `release-X.Y.Z` branch is created with the
  current contents of `main`.
* On the 20th of every month this release branch is published to crates.io and
  release artifacts are built.

This means that Wasmtime releases are always at least two weeks behind
development on `main` and additionally happen once a month. The lag time behind
`main` is intended to give time to fuzz changes on `main` as well as allow
testing for any users using `main`. It's expected, though, that most consumers
will likely use the release branches of wasmtime.

A detailed list of all the steps in the release automation process are below.
The steps requiring interactions are **bolded**, otherwise everything else is
automatic and this is documenting what automation does.

1. On the 5th of every month, (configured via
   `.github/workflows/release-process.yml`) a CI job
   will run and do these steps:
   * Download the current `main` branch
   * Push the `main` branch to `release-X.Y.Z`
   * Run `./scripts/publish.rs` with the `bump` argument
   * Commit the changes
   * Push these changes to a temporary `ci/*` branch
   * Open a PR with this branch against `main`
   * This step can also be [triggered manually][ci-trigger] with the `main`
     branch and the `cut` argument.
2. **A maintainer of Wasmtime merges this PR**
   * It's intended that this PR can be immediately merged as the release branch
     has been created and all it's doing is bumping the version.
3. **Time passes and the `release-X.Y.Z` branch is maintained**
   * All changes land on `main` first, then are backported to `release-X.Y.Z` as
     necessary.
4. On the 20th of every month (same CI job as before) another CI job will run
   performing:
   * Reset to `release-X.Y.Z`
   * Update the release date of `X.Y.Z` to today in `RELEASES.md`
   * Add a special marker to the commit message to indicate a tag should be made.
   * Open a PR against `release-X.Y.Z` for this change
   * This step can also be [triggered manually][ci-trigger] with the `main`
     branch and the `release-latest` argument.
5. **A maintainer of Wasmtime merges this PR**
   * When merged, will trigger the next steps due
     to the marker in the commit message. A maintainer should double-check there
     are [no open security issues][rustsec-issues], but otherwise it's expected
     that all other release issues are resolved by this point.
6. The main CI workflow at `.github/workflow/main.yml` has special logic
   at the end such that pushes to the `release-*` branch will scan the git logs
   of pushed changes for the special marker added by `release-process.yml`. If
   found and CI passes a tag is created and pushed.
7. Once a tag is created the `.github/workflows/publish-*` workflows run. One
   publishes all crates as-is to crates.io and the other will download all
   artifacts from the `main.yml` workflow and then upload them all as an
   [official release](https://github.com/bytecodealliance/wasmtime/releases).

If all goes well you won't have to read up much on this and after hitting the
Big Green Button for the automatically created PRs everything will merrily
carry on its way.

[rustsec-issues]: https://github.com/bytecodealliance/wasmtime/issues?q=RUSTSEC+is%3Aissue+is%3Aopen+
[ci-trigger]: https://github.com/bytecodealliance/wasmtime/actions/workflows/release-process.yml

## Releasing a patch version

Making a patch release is somewhat more manual than a major version, but like
before there's automation to help guide the process as well and take care of
more mundane bits.

This is a list of steps taken to perform a patch release for 2.0.1 for example.
Like above human interaction is indicated with **bold** text in these steps.

1. **Necessary changes are backported to the `release-2.0.0` branch from
   `main`**
   * All changes must land on `main` first (if applicable) and then get
     backported to an older branch. Release branches should already exist from
     the above major release steps.
   * CI may not have been run in some time for release branches so it may be
     necessary to backport CI fixes and updates from `main` as well.
   * When merging backports maintainers need to double-check that the
     `PUBLIC_CRATES` listed in `scripts/publish.rs` do not have
     semver-API-breaking changes (in the strictest sense). All security fixes
     must be done in such a way that the API doesn't break between the patch
     version and the original version.
   * Don't forget to write patch notes in `RELEASES.md` for backported changes.
2. **The patch release process is [triggered manually][ci-trigger] with
   the `release-2.0.0` branch and the `release-patch` argument**
   * This will run the `release-process.yml` workflow. The `scripts/publish.rs`
     script will be run with the `bump-patch` argument.
   * The changes will be committed with a special marker indicating a release
     needs to be made.
   * A PR will be created from a temporary `ci/*` branch to the `release-2.0.0`
     branch which, when merged, will trigger the release process.
3. **Review the generated PR and merge it**
   * This will resume from step 6 above in the major release process where the
     special marker in the commit message generated by CI will trigger a tag to
     get pushed which will further trigger the rest of the release process.
   * Please make sure to update the `RELEASES.md` at this point to include the
     `Released on` date by pushing directly to the branch associated with the
     PR.

[bump-version]: https://github.com/bytecodealliance/wasmtime/actions/workflows/bump-version.yml

## Releasing a security patch

When making a patch release that has a security-related fix the contents of the
patch are often kept private until the day of the patch release which means that
the process here is slightly different from the patch release process above. In
addition the precise [runbook is currently under discussion in an
RFC](https://github.com/bytecodealliance/rfcs/pull/20) for security patches, so
this intends to document what we've been doing so far and it'll get updated when
the runbook is merged.

1. **The fix for the security issue is developed in a GitHub Security
   Advisory**
   * This will not have any CI run, it's recommended to run `./ci/run-tests.sh`
     locally at least.
   * Develop fixes for all branches that will get a patch release in the
     advisory, one PR per branch. When the advisory is published all branches
     will be merged simultaneously. Be sure to run `./ci/run-tests.sh` in each
     branch.
   * Don't forget to update `RELEASES.md` with notes about the release on
     each branch.
2. **Send a PR for the version bump when an email goes out announcing there will
   be a security release**
   * An email is sent to the bytecodealliance security mailing list ahead of a
     patch release to announce that a patch release will happen. At this time you
     should [trigger the version bump][ci-trigger] against the appropriate
     `release-x.y.z` branches with the `release-patch` argument.
   * This will send a PR, but you should not merge it. Instead use this PR and
     the time ahead of the security release to fix any issues with CI. Older
     `release-x.y.z` branches haven't run CI in awhile so they may need to
     backport fixes of one variety or another. DO NOT include the actual fix for
     the security issue into the PR, that comes in the next step.
3. **Make the advisories/patches public**
   * Publishing the GitHub Security Advisory will merge all the PRs into each
     branch from the advisory. Note that CI will run for release branches but
     `main` will probably fail CI since it expected to be merged through the
     merge queue, but that's ok.
   * Double-check that CI for release branches finishes and completes
     successfully.
4. **Merge the version-bump PR**
   * Like the patch release process this will kick everything else into motion.
     Note that the actual security fixes should be merged either before or as
     part of this PR.

## Releasing Notes

Release notes for Wasmtime are written in the `RELEASES.md` file in the root of
the repository. Management of this file looks like:

* (theoretically) All changes on `main` which need to write an entry in
  `RELEASES.md`.
* When the `main` branch gets a version the `RELEASES.md` file is emptied and
  replaced with `ci/RELEASES-template.md`. An entry for the upcoming release is
  added to the bulleted list at the bottom.
* (realistically) After a `release-X.Y.Z` branch is created release notes are
  updated and edited on the release branch.

This means that `RELEASES.md` only has release notes for the release branch that
it is on. Historical release notes can be found through links at the bottom to
previous copies of `RELEASES.md`
