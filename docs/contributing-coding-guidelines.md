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

Wasmtime supports the latest three stable releases of Rust.

Cranelift supports stable Rust, and follows the [Rust Update Policy for
Firefox].

Some of the CI jobs depend on nightly Rust, for example to run rustdoc with
nightly features, however these use pinned versions in CI that are updated
periodically and the general repository does not depend on nightly features.

[Rust Update Policy for Firefox]: https://wiki.mozilla.org/Rust_Update_Policy_for_Firefox#Schedule
