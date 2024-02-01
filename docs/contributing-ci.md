# Continuous Integration (CI)

The Wasmtime and Cranelift projects heavily rely on Continuous Integration (CI)
to ensure everything keeps working and keep the final end state of the code at
consistently high quality. The CI setup for this repository is relatively
involved and extensive, and so it's worth covering here how it's organized and
what's expected of contributors.

All CI currently happens on GitHub Actions and is configured in the [`.github`
directory][dir] of the repository.

[dir]: https://github.com/bytecodealliance/wasmtime/tree/main/.github

## PRs and CI

Currently on sample of the full CI test suite is run on every Pull Request. CI
on PRs is intended to be relatively quick and catch the majority of mistakes and
errors. By default the test suite is run on x86\_64 Linux but this may change
depending on what files the PR is modifying. The intention is to run "mostly
relevant" CI on a PR by default.

PR authors are expected to fix CI failures in their PR, unless the CI failure is
systemic and unrelated to the PR. In that case other maintainers should be
alerted to ensure that the problem can be addressed. Some reviewers may also
wait to perform a review until CI is green on the PR as otherwise it may
indicate changes are needed.

The Wasmtime repository uses GitHub's Merge Queue feature to merge PRs which.
Entry in to the merge queue requires green CI on the PR beforehand. Maintainers
who have approved a PR will flag it for entry into the merge queue, and the PR
will automatically enter the merge queue once CI is green.

When entering the merge queue a PR will have the full test suite executed which
may include tests that weren't previously run on the PR. This may surface new
failures, and contributors are expected to fix these failures as well.

To force PRs to execute the full test suite, which takes longer than the default
test suite for PRs, then contributors can place the string "prtest:full"
somewhere in any commit of the PR. From that point on the PR will automatically
run the full test suite as-if it were in the merge queue. Note that when going
through the merge queue this will rerun tests.

## Tests run on CI

While this may not be fully exhaustive, the general idea of all the checks we
run on CI looks like this:

* Code formatting - we run `cargo fmt -- --check` on CI to ensure that all code
  in the repository is formatted with rustfmt. All PRs are expected to be
  formatted with the latest stable version of rustfmt.

* Book documentation tests - code snippets (Rust ones at least) in the book
  documentation ([the `docs`
  folder](https://github.com/bytecodealliance/wasmtime/tree/main/docs)) are
  tested on CI to ensure they are working.

* Crate tests - the moral equivalent of `cargo test --all` and `cargo test --all
  --release` is executed on CI. This means that all workspace crates have their
  entire test suite run, documentation tests and all, in both debug and release
  mode. Additionally we execute all crate tests on macOS, Windows, and Linux, to
  ensure that everything works on all the platforms.

* Fuzz regression tests - we take a random sampling of the [fuzz
  corpus](https://github.com/bytecodealliance/wasmtime-libfuzzer-corpus) and run
  it through the fuzzers. This is mostly intended to be a pretty quick
  regression test and testing the fuzzers still build, most of our fuzzing
  happens on [oss-fuzz](https://oss-fuzz.com). Found issues are recorded in
  the [oss-fuzz bug tracker](https://bugs.chromium.org/p/oss-fuzz/issues/list?q=-status%3AWontFix%2CDuplicate%20-component%3AInfra%20proj%3Awasmtime&can=1)

While we do run more tests here and there, this is the general shape of what you
can be expected to get tested on CI for all commits and all PRs. You can of
course always feel free to expand our CI coverage by editing the CI files
themselves, we always like to run more tests!

## Artifacts produced on CI

Our CI system is also responsible for producing all binary releases and
documentation of Wasmtime and Cranelift. Currently this consists of:

* Tarballs of the `wasmtime` CLI - produced for macOS, Windows, and Linux we try
  to make these "binary compatible" wherever possible, for example producing the
  Linux build in a really old CentOS container to have a very low glibc
  requirement.

* Tarballs of the Wasmtime C API - produced for the same set of platforms as the
  CLI above.

* Book and API documentation - the book is rendered with `mdbook` and we also
  build all documentation with `cargo doc`.

* A source code tarball which is entirely self-contained. This source tarball
  has all dependencies vendored so the network is not needed to build it.

* WebAssembly adapters for the component model to translate
  `wasi_snapshot_preview1` to WASI Preview 2.

Artifacts are produced as part of the full CI suite. This means that artifacts
are not produced on a PR by default but can be requested via "prtest:full". All
runs through the merge queue though, which means all merges to `main`, will
produce a full suite of artifacts. The latest artifacts are available through
Wasmtime's [`dev` release][dev] and downloads are also available for recent CI
runs through the CI page in GitHub Actions.

[dev]: https://github.com/bytecodealliance/wasmtime/releases/tag/dev
