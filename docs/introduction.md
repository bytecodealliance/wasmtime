# Introduction

[Wasmtime][github] is a standalone runtime for WebAssembly, WASI, and the
Component Model by the [Bytecode Alliance][BA].

[WebAssembly] (abbreviated Wasm) is a binary instruction format that is designed
to be a portable compilation target for programming languages. Wasm binaries
typically have a `.wasm` file extension. In this documentation, we'll also use
the textual representation of the binary files, which have a `.wat` file
extension.

[WASI] (the WebAssembly System Interface) defines interfaces that provide a
secure and portable way to access several operating-system-like features such as
filesystems, networking, clocks, and random numbers.

[The Component Model] is a Wasm architecture that provides a binary format for
portable, cross-language composition. More specifically, it supports the use of
interfaces via which components can communicate with each other. WASI
is defined in terms of component model interfaces.

Wasmtime runs WebAssembly code [outside of the Web], and can be used both as a
command-line utility or as a library embedded in a larger application. It
strives to be

- **Fast**: Wasmtime is built on the optimizing [Cranelift] code generator.
- **Secure**: Wasmtime's development is strongly focused on correctness and
  security.
- **Configurable**: Wasmtime uses sensible defaults, but can also be configured
  to provide more fine-grained control over things like CPU and memory
  consumption.
- **Standards Compliant**: Wasmtime passes the official WebAssembly test suite
  and the Wasmtime developers are intimately engaged with the WebAssembly
  standards process.

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
[Cranelift]: https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/README.md
[WebAssembly]: https://webassembly.org/
[WASI]: https://wasi.dev
[outside of the Web]: https://webassembly.org/docs/non-web/
[issue]: https://github.com/bytecodealliance/wasmtime/issues/new
[The Component Model]: https://github.com/WebAssembly/component-model
