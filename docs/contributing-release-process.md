# Release Process

This is intended to serve as documentation for Wasmtime's release process. It's
largely an internal checklist for those of us performing a Wasmtime release, but
others might be curious in this as well!

To kick off the release process someone decides to do a release. Currently
there's not a schedule for releases or something similar. Once the decision is
made (there's also not really a body governing these decisions, it's more
whimsical currently, or on request from others) then the following steps need to
be executed to make the release:

1. Double-check that there are no open [rustsec advisory
   issues][rustsec-issues] on the Wasmtime repository.
1. `git pull` - make sure you've got the latest changes
1. Run `rustc scripts/publish.rs`
1. Run `./publish bump`
  * Review and commit the changes
  * Note that this bumps all cranelift/wasmtime versions as a major version bump
    at this time. See the `bump_version` function in `publish.rs` to tweak this.
1. Make sure `RELEASES.md` is up-to-date, and fill it out if it doesn't have an
   entry yet for the current release.
1. Send this version update as a PR to the `wasmtime` repository, wait for a merge
1. After merging, tag the merge as `vA.B.C`
1. Push the tag to the repository
  * This will trigger the release CI which will create all release artifacts and
    publish them to GitHub releases.
1. Run `./publish publish`
  * This will fail on some crates, but that's expected.
  * Keep running this script until all crates are published. Note that crates.io
    won't let you publish something twice so rerunning is only for crates which
    need the index to be udpated and if it hasn't yet. It's recommended to wait
    a bit between runs of the script.

And that's it, then you've done a Wasmtime release.

[rustsec-issues]: https://github.com/bytecodealliance/wasmtime/issues?q=RUSTSEC+is%3Aissue+is%3Aopen+
