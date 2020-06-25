# Introduction

[Wasmtime][github] is a [Bytecode Alliance][BA] project that is a standalone
wasm-only optimizing runtime for [WebAssembly] and [WASI]. It runs WebAssembly
code [outside of the Web], and can be used both as a command-line utility or as
a library embedded in a larger application.

Wasmtime strives to be a highly configurable and embeddable runtime to run on
any scale of application. Many features are still under development so if you
have a question don't hesitate to [file an issue][issue].

This guide is intended to server a number of purposes and within you'll find:

* [How to create simple wasm modules](tutorial-create-hello-world.md)
* [How to use Wasmtime from a number of languages](lang.md)
* [How to use install and use the `wasmtime` CLI](cli.md)
* Information about [stability](stability.md) and [security](security.md) in
  Wasmtime.

... and more! The source for this guide [lives on
GitHub](https://github.com/bytecodealliance/wasmtime/tree/main/docs) and
contributions are welcome!

[github]: https://github.com/bytecodealliance/wasmtime
[BA]: https://bytecodealliance.org/
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
