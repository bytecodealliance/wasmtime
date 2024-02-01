# Coding guidelines

For the most part, Wasmtime and Cranelift follow common Rust conventions and
[pull request] (PR) workflows, though we do have a few additional things to
be aware of.

[pull request]: https://help.github.com/articles/about-pull-requests/

### `rustfmt`

All PRs must be formatted according to rustfmt, and this is checked in the
continuous integration tests. You can format code locally with:

```sh
$ cargo fmt
```

at the root of the repository. You can find [more information about rustfmt
online](https://github.com/rust-lang/rustfmt) too, such as how to configure
your editor.

### Minimum Supported `rustc` Version

Wasmtime and Cranelift support the latest three stable releases of Rust. This
means that if the latest version of Rust is 1.72.0 then Wasmtime supports Rust
1.70.0, 1.71.0, and 1.72.0. CI will test by default with 1.72.0 and there will
be one job running the full test suite on Linux x86\_64 on 1.70.0.

Some of the CI jobs depend on nightly Rust, for example to run rustdoc with
nightly features, however these use pinned versions in CI that are updated
periodically and the general repository does not depend on nightly features.

Updating Wasmtime's MSRV is done by editing the `rust-version` field in the
workspace root's `Cargo.toml`

### Dependencies of Wasmtime

Wasmtime and Cranelift have a higher threshold than default for adding
dependencies to the project. All dependencies are required to be "vetted"
through the [`cargo vet` tool](https://mozilla.github.io/cargo-vet/). This is
checked on CI and will run on all modifications to `Cargo.lock`.

A "vet" for Wasmtime is not a meticulous code review of a dependency for
correctness but rather it is a statement that the crate does not contain
malicious code and is safe for us to run during development and (optionally)
users to run when they run Wasmtime themselves. Wasmtime's vet entries are used
by other organizations which means that this isn't simply for our own personal
use. Wasmtime additionally uses vet entries from other organizations as well
which means we don't have to vet everything ourselves.

New vet entries are required to be made by trusted contributors to Wasmtime.
This is all configured in the `supply-chain` folder of Wasmtime. These files
generally aren't hand-edited though and are instead managed through the `cargo
vet` tool itself. Note that our `supply-chain/audits.toml` additionally contains
entries which indicates that authors are trusted as opposed to vets of
individual crates. This lowers the burden of updating version of a crate from a
trusted author.

When put together this means that contributions to Wasmtime and Cranelift which
update existing dependencies or add new dependencies will not be mergeable by
default (CI will fail). This is expected from our project's configuration and
this situation will be handled one of a few ways:

* If a new dependency is being added it might be worth trying to slim down
  what's required or avoiding the dependency altogether. Avoiding new
  dependencies is best when reasonable, but it is not always reasonable to do
  so. This is left to the judgement of the author and reviewer.

* When updating dependencies this should be done for a specific purpose relevant
  to the PR-at-hand. For example if the PR implements a new feature then the
  dependency update should be required for the new feature. Otherwise it's best
  to leave dependency updates to their own PRs. It's ok to update dependencies
  "just for the update" but we prefer to have that as separate PRs.

* If a new dependency or dependency update is required, then a trusted
  contributor of Wasmtime will be required to perform a vet of the new
  crate/version. This will be done through a separate PR to Wasmtime so we ask
  contributors to not run `cargo vet` themselves to get CI passing. Reviewers
  understand what `cargo vet` failures are on CI and how it doesn't reflect on
  the quality of the PR itself. Once the reviewer (or another maintainer) merges
  a PR adding the vet entries necessary for the original contribution it can be
  rebased to get CI passing.

Note that this process is not in place to prevent new dependencies or prevent
updates, but rather it ensures that development of Wasmtime is done with a
trusted set of code that has been reviewed by trusted parties. We welcome
dependency updates and new functionality, so please don't be too alarmed when
contributing and seeing a failure of `cargo vet` on CI!
