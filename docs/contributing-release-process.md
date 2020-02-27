# Release Process

This is intended to serve as documentation for wasmtime's release process. It's
largely an internal checklist for those of us performing a wasmtime release, but
others might be curious in this as well!

To kick off the release process someone decides to do a release. Currently
there's not a schedule for releases or something similar. Once the decision is
made (there's also not really a body governing these decisions, it's more
whimsical currently, or on request from others) then the following steps need to
be executed to make the release:

1. `git pull` - make sure you've got the latest changes
1. Update the version numbers in `Cargo.toml` for all crates
  * Edit `scripts/bump-wasmtime-version.sh`, notable the `version` variable
  * Run the script
  * Commit the changes
1. Make sure `RELEASES.md` is up-to-date, and fill it out if it doesn't have an
   entry yet for the current release.
1. Send this version update as a PR to the wasmtime repository, wait for a merge
1. After merging, tag the merge as `vA.B.C`
1. Push the tag to the repository
  * This will trigger the release CI which will create all release artifacts and
    publish them to GitHub releases.
1. Run `scripts/publish-all.sh` to publish all crates to crates.io

And that's it, then you've done a wasmtime release.
