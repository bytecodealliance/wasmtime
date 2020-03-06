# Coding guidelines

For the most part, Wasmtime and Cranelift follow common Rust conventions and
[pull request] (PR) workflows, though we do have a few additional things to
be aware of.

[pull request]: https://help.github.com/articles/about-pull-requests/

### rustfmt

All PRs must be formatted according to rustfmt, and this is checked in the
continuous integration tests. You can format code locally with:

```sh
$ cargo fmt
```

at the root of the repository. You can find [more information about rustfmt
online](https://github.com/rust-lang/rustfmt) too, such as how to configure
your editor.

### Rustc version support

Wasmtime supports the current stable version of Rust.

Cranelift supports stable Rust, and follows the [Rust Update Policy for
Firefox].

Some of the developer scripts depend on nightly Rust, for example to run
clippy and other tools, however we avoid depending on these for the main
build.

[Rust Update Policy for Firefox]: https://wiki.mozilla.org/Rust_Update_Policy_for_Firefox#Schedule
