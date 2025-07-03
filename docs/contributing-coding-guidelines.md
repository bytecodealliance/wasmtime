# Coding guidelines

For the most part, Wasmtime and Cranelift follow common Rust conventions and
[pull request] (PR) workflows, though we do have a few additional things to
be aware of.

[pull request]: https://help.github.com/articles/about-pull-requests/

### `rustfmt`

All PRs must be formatted according to rustfmt, and this is checked in the
continuous integration tests. You can format code locally with:

```console
cargo fmt
```

at the root of the repository. You can find [more information about rustfmt
online](https://github.com/rust-lang/rustfmt) too, such as how to configure
your editor.

### Compiler Warnings and Lints

Wasmtime promotes all compiler warnings to errors in CI, meaning that the `main`
branch will never have compiler warnings for the version of Rust that's being
tested on CI. Compiler warnings change over time, however, so it's not always
guaranteed that Wasmtime will build with zero warnings given an arbitrary
version of Rust. If you encounter compiler warnings on your version of Rust
please feel free to send a PR fixing them.

During local development, however, compiler warnings are simply warnings and the
build and tests can still succeed despite the presence of warnings. This can be
useful because warnings are often quite prevalent in the middle of a
refactoring, for example. By the time you make a PR, though, we'll require that
all warnings are resolved or otherwise CI will fail and the PR cannot land.

Compiler lints are controlled through the `[workspace.lints.rust]` table in the
`Cargo.toml` at the root of the Wasmtime repository. A few allow-by-default
lints are enabled such as `trivial_numeric_casts`, and you're welcome to enable
more lints as applicable. Lints can additionally be enabled on a per-crate basis
such as placing this in a `src/lib.rs` file:

```rust
#![warn(trivial_numeric_casts)]
```

Using `warn` here will allow local development to continue while still causing
CI to promote this warning to an error.

### Clippy

All PRs are gated on `cargo clippy` passing for all workspace crates and
targets. All clippy lints, however, are allow-by-default and thus disabled. The
Wasmtime project selectively enables Clippy lints on an opt-in basis. Lints can
be controlled for the entire workspace via `[workspace.lints.clippy]`:

```toml
[workspace.lints.clippy]
# ...
manual_strip = 'warn'
```

or on a per-crate or module basis by using attributes:

```rust
#![warn(clippy::manual_strip)]
```

In Wasmtime we've found that the default set of Clippy lints is too noisy to
productively use other Clippy lints, hence the allow-by-default behavior.
Despite this though there are numerous useful Clippy lints which are desired for
all crates or in some cases for a single crate or module. Wasmtime encourages
contributors to enable Clippy lints they find useful through workspace or
per-crate configuration.

Like compiler warnings in the above section all Clippy warnings are turned into
errors in CI. This means that `cargo clippy` should always produce no warnings
on Wasmtime's `main` branch if you're using the same compiler version that CI
does (typically current stable Rust). This means, however, that if you enable a
new Clippy lint for the workspace you'll be required to fix the lint for all
crates in the workspace to land the PR in CI.

Clippy can be run locally with:

```console
cargo clippy --workspace --all-targets
```

Contributors are welcome to enable new lints and send PRs for this. Feel free to
reach out if you're not sure about a lint as well.

### Minimum Supported `rustc` Version (MSRV)

Wasmtime and Cranelift support the latest three stable releases of Rust. This
means that if the latest version of Rust is 1.72.0 then Wasmtime supports Rust
1.70.0, 1.71.0, and 1.72.0. CI will test by default with 1.72.0 and there will
be one job running the full test suite on Linux x86\_64 on 1.70.0.

Some of the CI jobs depend on nightly Rust, for example to run rustdoc with
nightly features, however these use pinned versions in CI that are updated
periodically and the general repository does not depend on nightly features.

Updating Wasmtime's MSRV is done by editing the `rust-version` field in the
workspace root's `Cargo.toml`

Note that this policy is subject to change over time (notably it might be
extended to include more rustc versions). Current Wasmtime users don't require a
larger MSRV window to justify the maintenance needed to have a larger window. If
your use case requires a larger MSRV range though please feel free to contact
maintainers to raise your use case (e.g. an issue, in a Wasmtime meeting, on
Zulip, etc).

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

### Crate Organization

The Wasmtime repository is a bit of a monorepo with lots of crates internally
within it. The Wasmtime project and `wasmtime` crate also consists of a variety
of crates intended for various purposes. As such not all crates are treated
exactly the same and so there are some rough guidelines here about adding new
crates to the repository and where to place/name them:

* Wasmtime-related crates live in `crates/foo/Cargo.toml` where the crate name
  is typically `wasmtime-foo` or `wasmtime-internal-foo`.

* Cranelift-related crates live in `cranelift/foo/Cargo.toml` where the crate is
  named `cranelift-foo`.

* Some projects such as Winch, Pulley, and Wiggle are exceptions to the above
  rules and live in `winch/*`, `pulley/*` and `crates/wiggle/*`.

* Some crates are "internal" to Wasmtime. This means that they only exist for
  crate organization purposes (such as optional dependencies, or code
  organization). These crates are not intended for public consumption and are
  intended for exclusively being used by the `wasmtime` crate, for example, or
  other public crates. These crates should be named `wasmtime-internal-foo` and
  live in `crates/foo`. The `[workspace.dependencies]` directive in `Cargo.toml`
  at the root of the repository should rename it to `wasmtime-foo` in
  workspace-local usage, meaning that the "internal" part is only relevant on
  crates.io.

### Use of `unsafe`

Wasmtime is a project that contains `unsafe` Rust code. Wasmtime is also used in
security-critical contexts which means that it's extra-important that this
`unsafe` code is correct. The purpose of this section is to outline guidelines
and guidance for how to use `unsafe` in Wasmtime.

Ideally Wasmtime would have no `unsafe` code. For large components of Wasmtime
this is already true, for these components have little to no `unsafe` code:

* Cranelift - compiling WebAssembly modules.
* Winch - compiling WebAssembly modules.
* Wasmparser - validating WebAssembly.
* `wasmtime-wasi` / `wasmtime-wasi-http` - implementation of WASI.

Without `unsafe` the likelihood of a security bug is greatly reduced with the
riskiest possibility being a DoS vector through a panic, generally considered a
low-severity issue. Inevitably though due to the nature of Wasmtime it's
effectively impossible to 100% remove `unsafe` code. The question then becomes
what is the right balance and how to work with `unsafe`?

Some `unsafe` blocks are effectively impossible to remove. For example somewhere
in Wasmtime we're going to take the output of Cranelift and turn it into a
function pointer to calling it. In doing so the correctness of the `unsafe`
block relies on the correctness of Cranelift as well as the translation from
WebAssembly to Cranelift. This is a fundamental property of the Wasmtime project
and thus can't really be mitigated.

Other `unsafe` blocks, however, ideally will be self-contained and isolated to a
small portion of Wasmtime. For this code Wasmtime tries to follow these
guidelines:

1. Users of the public API of the `wasmtime` crate should never need `unsafe`.
   The API of `wasmtime` should be sound and safe no matter how its combined
   with other safe Rust code. While `unsafe` additions are allowed they should
   be very clearly documented with a precise contract of what exactly is unsafe
   and what must be upheld by the caller. For example `Module::deserialize`
   clearly documents that it could allow arbitrary code execution and thus it's
   not safe to pass in arbitrary bytes, but previously serialized bytes are
   always safe to pass in.

2. Declaring a function as `unsafe` should be accompanied with clear
   documentation on the function declaration indicating why the function is
   `unsafe`. This should clearly indicate all the contracts that need to be
   upheld by callers for the invocation to be safe. There is no way to verify
   that the documentation is correct but this is a useful flag to reviewers and
   readers alike to be more vigilant around such functions.

3. An `unsafe` block within a function should be accompanied with a preceding
   comment explaining why it's safe to have this block. It should be possible to
   verify this comment with local reasoning, for example considering little code
   outside of the current function or module. This means that it should be
   almost trivial to connect the contracts required on the callee function (why
   the `unsafe` block is there in the first place) to the surrounding code. This
   can include the current function being `unsafe` (effectively "forwarding" the
   contract of the callee) or via local reasoning.

4. Implementation of a feature within Wasmtime should not result in excessive
   amounts of `unsafe` functions or usage of `unsafe` functions. The goal here
   is that if two possible designs for a feature are being weighed it's not
   required to favor one with zero unsafe vs one with just a little unsafe, but
   one with a little unsafe should be favored over one that is entirely unsafe.
   An example of this is Wasmtime's implementation of the GC proposal with a
   sandboxed heap where the data on the heap is never trusted. This comes at a
   minor theoretical performance loss on the host but has the benefit of all
   functions within the implementation are all safe. These sorts of design
   tradeoffs are not really possible to codify in stone, but the general
   guideline is to try to favor safer implementations so long as the
   hypothetical sacrifice in performance isn't too great.

It should be noted that Wasmtime is a relatively large and old codebase and thus
does not perfectly follow these guidelines for preexisting code. Code not
following these guidelines is considered technical debt that must be paid down
at one point. Wasmtime tries to [keep track of known issues][unsafe-code-tag] to
burn down this list over time. New features to Wasmtime are allowed to add to
this list, but it should be clear how to burn down the list in time for any new
entries added.

[unsafe-code-tag]: https://github.com/bytecodealliance/wasmtime/labels/wasmtime%3Aunsafe-code
