# Introduction

[Wasmtime][github] is a standalone optimizing runtime for [WebAssembly],
[the Component Model], and [WASI] by the [Bytecode Alliance][BA]. It runs
WebAssembly code [outside of the Web], and can be used both as a command-line
utility or as a library embedded in a larger application. Wasmtime strives to be
a highly configurable and embeddable runtime to run on any scale of application.

This documentation is intended to serve a number of purposes and within you'll
find:

* [How to use Wasmtime from a number of languages](lang.md)
* [How to install and use the `wasmtime` CLI](cli.md)
* Information about [stability](stability.md) and [security](security.md) in
  Wasmtime.
* Documentation about [contributing](contributing.md) to Wasmtime.

... and more! The source for this guide [lives on
GitHub](https://github.com/bytecodealliance/wasmtime/tree/main/docs) and
contributions are welcome!

[github]: https://github.com/bytecodealliance/wasmtime
[BA]: https://bytecodealliance.org/
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
[the Component Model]: https://github.com/WebAssembly/component-model
