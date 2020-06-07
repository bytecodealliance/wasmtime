# Introduction

[Wasmtime][github], a [Bytecode Alliance][BA] project, is used to run
[Webassembly][Wasm] (Wasm) programs [without a web browser].  
[Wasm] is a low-level executable assembly language.  Typically, an application
or library calls wasmtime to load and run Wasm code. Or, the Wasmtime command
line utility can run standalone Wasm programs.

Wasm programs are generally written in high-level languages, and compiled to
Wasm.  [WebAssembly System Interface][Wasi] (Wasi) provides a library of
essential functions, such as reading files, somewhat similar libc. Wasmtime
enables them to run inside another application.

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
[Wasm]: https://webassembly.org/
[Wasi]: https://wasi.dev
[without a web browser]: https://webassembly.org/docs/non-web/
[Wasm-bindgen]: https://rustwasm.github.io/docs/wasm-bindgen/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
