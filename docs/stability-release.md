# Release Process

Wasmtime's release process was [originally designed in an RFC][rfc4] and later
amended with [an LTS process][rfc-lts] and this page is intended to serve as
documentation for the current process as-is today.

The high-level summary of Wasmtime's release process is:

* A new major version of Wasmtime will be made available on the 20th of each
  month.
* Each release that is a multiple of 12 is considered an LTS release and is
  supported for 24 months. Other releases are supported for 2 months.
* Security bugs are guaranteed to be backported to all supported releases.
* Bug fixes are backported on a volunteer basis.

[rfc-lts]: https://github.com/bytecodealliance/rfcs/pull/42

## Current Versions

<div id='version-table'>

This is a table of supported, recent, and some upcoming releases of Wasmtime
along with the dates around their release process. Rows in **bold** are
actively supported at this time.

| Version    | LTS | Branch Date | Release Date | EOL Date |
|------------|-----|-------------|--------------|----------|

<noscript>
JavaScript is disabled so the table above is empty.
</noscript>

In more visual form this is a gantt chart of the current release trains:

<noscript>
JavaScript is disabled there is no gantt chart to show.
</noscript>

</div>


## New Versions

Once a month Wasmtime will issue a new major version. This will be issued with a
semver-major version update, such as 4.0.0 to 5.0.0. Releases are created from
main with a new `release-X.0.0` git branch on the 5th of every month. The
release itself then happens on the 20th of the month, or shortly after if that
happens to fall on a weekend.

Each major release of Wasmtime reserves the right to break both behavior and API
backwards-compatibility. This is not expected to happen frequently, however, and
any breaking change will follow these criteria:

* Minor breaking changes, either behavior or with APIs, will be documented in
  the `RELEASES.md` release notes. Minor changes will require some degree of
  consensus but are not required to go through the entire RFC process.

* Major breaking changes, such as major refactorings to the API, will be
  required to go through the [RFC process]. These changes are intended to be
  broadly communicated to those interested and provides an opportunity to give
  feedback about embeddings. Release notes will clearly indicate if any major
  breaking changes through accepted RFCs are included in a release.

All releases will have an accompanying `RELEASES.md` on the release branch
documenting major and minor changes made during development. Note that each
branch only contains the release notes for that branch, but links are provided
for older release notes.

For maintainers, performing a release is [documented
here](./contributing-release-process.md#releasing-a-major-version).

## Version Support

Wasmtime major version releases are of one of two categories:

* LTS release - this happens every 12 releases of Wasmtime and the version
  number is always divisible by 12. LTS releases are supported for 24 months.
  For example Wasmtime 24.0.0 is supported for 2 years.

* Normal release - this is every release other than an LTS release. Normal
  releases are supported for 2 months. For example Wasmtime 31.0.0 is supported
  for 2 months.

At any one time Wasmtime has two supported LTS releases and up to two supported
normal releases. Once a version of Wasmtime is release the project strives to
maintain binary/version compatibility with dependencies and such throughout the
lifetime of the release. For example the minimum supported version of Rust
required to compile a version of Wasmtime will not increase. Exceptions may be
made to LTS branches though if the versions of tooling to produce the LTS itself
have fallen out-of-date. For example if an LTS was originally produced with a
GitHub Actions runner that is no longer available then the oldest supported
image will be used instead.

## Patch Versions

Patch releases of Wasmtime will only be issued for security and critical
correctness issues for on-by-default behavior in supported releases. For example
if the current version is 39.0.0 then a security issue would issue a new release
for:

* 39.0.x - the current release
* 38.0.x - the last release
* 36.0.x - the current LTS release
* 24.0.x - the last LTS release

Patch releases are guaranteed to maintain API and behavior
backwards-compatibility and are intended to be trivial for users to upgrade to.

The Wasmtime project guarantees backports and patch releases will be made for
any discovered security issue. Other bug fixes are done on a best-effort basis
in accordance with volunteers able to do the backports (see below). The Wasmtime
project does not support backporting new features to older releases, even if a
volunteer performs a backport for the project.

Patch releases for Cranelift will be made for any miscompilations found by
Cranelift, even those that Wasmtime itself may not exercise. Due to the current
release process a patch release for Cranelift will issue a patch release for
Wasmtime as well.

Patch releases do not have a set cadence and are done on an as-needed basis. For
maintainers, performing a patch release is [documented
here](./contributing-release-process.md#releasing-a-patch-version).

## Security Fixes

Security fixes will be issued as patch releases of Wasmtime. They follow the
same process as normal backports except that they're coordinated in private
prior to patch release day.

For maintainers, performing a security release is [documented
here](./security-vulnerability-runbook.md).

## What's released?

At this time the release process of Wasmtime encompasses:

* The `wasmtime` Rust crate
* The C API of Wasmtime
* The `wasmtime` CLI tool through the `wasmtime-cli` Rust crate

Other projects maintained by the Bytecode Alliance will also likely be released,
with the same version numbers, with the main Wasmtime project soon after a
release is made, such as:

* [`wasmtime-dotnet`](https://github.com/bytecodealliance/wasmtime-dotnet)
* [`wasmtime-py`](https://github.com/bytecodealliance/wasmtime-py)
* [`wasmtime-go`](https://github.com/bytecodealliance/wasmtime-go)
* [`wasmtime-cpp`](https://github.com/bytecodealliance/wasmtime-cpp)
* [`wasmtime-rb`](https://github.com/bytecodealliance/wasmtime-rb)

Note, though, that bugs and security issues in these projects do not at this
time warrant patch releases for Wasmtime.

[rfc4]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-one-dot-oh.md
[RFC process]: https://github.com/bytecodealliance/rfcs
