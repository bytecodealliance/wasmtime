# Introduction

[Wasmtime][github] is a standalone optimizing runtime for [WebAssembly],
[the Component Model], and [WASI] by the [Bytecode Alliance][BA]. It runs
WebAssembly code [outside of the Web], and can be used both as a command-line
utility or as a library embedded in a larger application. Wasmtime strives to be
a highly configurable and embeddable runtime to run on any scale of application.

This documentation covers high-level information about the project such as:

* [How to install and use the `wasmtime` CLI](cli.md)
* [Examples of what Wasmtime supports](examples.md)
* Information about [stability](stability.md) and [security](security.md) in
  Wasmtime.
* Documentation about [contributing](contributing.md) to Wasmtime.

If you're using the [`wasmtime` crate](https://crates.io/crates/wasmtime) its
API reference documentation can be found [on
docs.rs](https://docs.rs/wasmtime/latest/wasmtime/), documentation for the C
API [lives here](https://docs.wasmtime.dev/c-api/), and for using Wasmtime in
other languages see [the corresponding link in the
`README.md`](https://github.com/bytecodealliance/wasmtime?tab=readme-ov-file#language-support)
for what's supported.

If you're interested in learning more about the Component Model or want to learn
about building WebAssembly components, see [this
excellent documentation](https://component-model.bytecodealliance.org/) which
lives outside of the Wasmtime repository.

The source for this documentation [lives on
GitHub](https://github.com/bytecodealliance/wasmtime/tree/main/docs) and
contributions are welcome!

[github]: https://github.com/bytecodealliance/wasmtime
[BA]: https://bytecodealliance.org/
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
[the Component Model]: https://github.com/WebAssembly/component-model
