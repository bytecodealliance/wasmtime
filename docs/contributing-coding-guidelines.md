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

[Rust Update Policy for Firefox]: https://wiki.mozilla.org/Rust_Update_Policy_for_Firefox#Schedule
