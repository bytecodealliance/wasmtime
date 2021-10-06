# Release Process

Wasmtime's release process was [originally designed in an RFC][rfc4] and this
page is intended to serve as documentation for the current process as-is today.
The high-level summary of Wasmtime's release process is:

* A new major version of Wasmtime will be made available once a month.
* Security bugs and correctness fixes will be backported to the latest two
  releases of Wasmtime and issued as patch releases.

Once a month Wasmtime will issue a new major version. This will be issued with a
semver-major version update, such as 4.0.0 to 5.0.0. The precise schedule of
Wasmtime's release may fluctuate slightly depending on public holidays and
availability of release resources, but the general cadence will be once-a-month.

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

Patch releases of Wasmtime will only be issued for security and correctness
issues for on-by-default behavior in the previous releases. If Wasmtime is
currently at version 5.0.0 then 5.0.1 and 4.0.1 will be issued as patch releases
if a bug is found. Patch releases are guaranteed to maintain API and behavior
backwards-compatibility and are intended to be trivial for users to upgrade to.

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

Note, though, that bugs and security issues in these projects do not at this
time warrant patch releases for Wasmtime.

[rfc4]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-one-dot-oh.md
[RFC process]: https://github.com/bytecodealliance/rfcs
