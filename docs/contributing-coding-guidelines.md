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

Note that this process is not in place to prevent new dependencies or prevent
updates, but rather it ensures that development of Wasmtime is done with a
trusted set of code that has been reviewed by trusted parties. We welcome
dependency updates and new functionality, so please don't be too alarmed when
contributing and seeing a failure of `cargo vet` on CI!

### `cargo vet` for Contributors

If you're a contributor to Wasmtime and you've landed on this documentation,
hello and thanks for your contribution! Here's some guidelines for changing the
set of dependencies in Wasmtime:

* If a new dependency is being added it might be worth trying to slim down
  what's required or avoiding the dependency altogether. Avoiding new
  dependencies is best when reasonable, but it is not always reasonable to do
  so. This is left to the judgement of the author and reviewer.

* When updating dependencies this should be done for a specific purpose relevant
  to the PR-at-hand. For example if the PR implements a new feature then the
  dependency update should be required for the new feature. Otherwise it's best
  to leave dependency updates to their own PRs. It's ok to update dependencies
  "just for the update" but we prefer to have that as separate PRs.

Dependency additions or updates require action on behalf of project maintainers
so we ask that you don't run `cargo vet` yourself or update the `supply-chain`
folder yourself. Instead a maintainer will review your PR and perform the `cargo
vet` entries themselves. Reviewers will typically make a separate pull request
to add `cargo vet` entries and once that lands yours will be added to the queue.

### `cargo vet` for Maintainers

Maintainers of Wasmtime are required to explicitly vet and approve all
dependency updates and modifications to Wasmtime. This means that when reviewing
a PR you should ensure that contributors are not modifying the `supply-chain`
directory themselves outside of commits authored by other maintainers. Otherwise
though to add vet entries this is done through one of a few methods:

* For a PR where maintainers themselves are modifying dependencies the `cargo
  vet` entries can be included inline with the PR itself by the author. The
  reviewer knows that the author of the PR is themself a maintainer.

* PRs that "just update dependencies" are ok to have at any time. You can do
  this in preparation for a future feature or for a future contributor. This
  more-or-less is the same as the previous categories.

* For contributors who should not add vet entries themselves maintainers should
  review the PR and add vet entries either in a separate PR or as part of the
  contributor's PR itself. As a separate PR you'll check out the branch, run
  `cargo vet`, then rebase away the contributor's commits and push your `cargo
  vet` commit alone to merge. For pushing directly to the contributor's own PR
  be sure to read the notes below.

Note for the last case it's important to ensure that if you push directly to a
contributor's PR any future updates pushed by the contributor either contain or
don't overwrite your vet entries. Also verify that if the PR branch is rebased
or force-pushed, the details of your previously pushed vetting remain the same:
e.g., versions were not bumped and descriptive reasons remain the same. If
pushing a vetting commit to a contributor's PR and also asking for more changes,
request that the contributor make the requested fixes in an additional commit
rather than force-pushing a rewritten history, so your existing vetting commit
remains untouched. These guidelines make it easier to verify no tampering has
occurred.

### Policy for adding `cargo vet` entries

For maintainers this is intended to document the project's policy on adding
`cargo vet` entries. The goal of this policy is to not make dependency updates
so onerous that they never happen while still achieving much of the intended
benefit of `cargo vet` in protection against supply-chain style attacks.

* For dependencies **that receive at least 10,000 downloads a day** on crates.io
  it's ok to add an entry to `exemptions` in `supply-chain/config.toml`. This
  does not require careful review or review at all of these dependencies. The
  assumption here is that a supply chain attack against a popular crate is
  statistically likely to be discovered relatively quickly. Changes to `main` in
  Wasmtime take at least 2 weeks to be released due to our release process, so
  the assumption is that popular crates that are victim of a supply chain attack
  would be discovered during this time. This policy additionally greatly helps
  when updating dependencies on popular crates that are common to see without
  increasing the burden too much on maintainers.

* For other dependencies a manual vet is required. The `cargo vet` tool will
  assist in adding a vet by pointing you towards the source code, as published
  on crates.io, to be browsed online. Manual review should be done to ensure
  that "nothing nefarious" is happening. For example `unsafe` should be
  inspected as well as use of ambient system capabilities such as `std::fs`,
  `std::net`, or `std::process`, and build scripts. Note that you're not
  reviewing for correctness, instead only for whether a supply-chain attack
  appears to be present.

This policy intends to strike a rough balance between usability and security.
It's always recommended to add vet entries where possible, but the first bullet
above can be used to update an `exemptions` entry or add a new entry. Note that
when the "popular threshold" is used **do not add a vet entry** because the
crate is, in fact, not vetted. This is required to go through an
`[[exemptions]]` entry.
