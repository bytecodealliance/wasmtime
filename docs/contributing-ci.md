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

Currently the full CI test suite runs on every Pull Request. All PRs need to
have that lovely green checkmark before being candidates for being merged. If a
test is failing you'll want to check out the logs on CI and fix it before the PR
can be merged.

PR authors are expected to fix CI failures in their PR, unless the CI failure is
systemic and unrelated to the PR. In that case other maintainers should be
alerted to ensure that the problem can be addressed.

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
  happens on [oss-fuzz](https://oss-fuzz.com).

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

* Tarballs of the Python extension - also produced on the main three platforms
  these wheels are compiled on each commit.

* Book and API documentation - the book is rendered with `mdbook` and we also
  build all documentation with `cargo doc`.

Artifacts are produced for every single commit and every single PR. You should
be able to find a downloadable version of all artifacts produced on the "runs"
page in GitHub Actions. For example [here's an example
job](https://github.com/bytecodealliance/wasmtime/actions/runs/50372673), and if
you're looking at [a specific
builder](https://github.com/bytecodealliance/wasmtime/runs/488719677?check_suite_focus=true)
you can see the artifacts link in the top right. Note that artifacts don't
become available until the whole run finishes.

Commits merged into the `main` branch will rerun CI and will also produce
artifacts as usual. On the `main` branch, however, documentation is pushed to
the `gh-pages` branch as well, and binaries are pushed to the `dev` release on
GitHub. Finally, tagged commits get a whole dedicated release to them too.
