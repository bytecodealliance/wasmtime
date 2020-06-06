# Introduction

Wasmtime, a Bytecode Alliance project, is used to run Webassembly
(WASM) and WebAssembly System Interface (WASI) without a web browser.
Typically, an application or library calls wasmtime to load and run
WASM code. Wasmtime is a standalone runtime, optimized for WASM
alone. It is intended for running WASM and WASI in a wide range of
applications, from command-line utilties to libraries in large
applications.

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
GitHub](https://github.com/bytecodealliance/wasmtime/tree/master/docs) and
contributions are welcome!

[github]: https://github.com/bytecodealliance/wasmtime
[BA]: https://bytecodealliance.org/
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
