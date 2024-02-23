# Using the `wasmtime` CLI

In addition to the [embedding API](https://docs.rs/wasmtime/latest/wasmtime/)
which allows you to use Wasmtime as a library, the Wasmtime project also
provides a `wasmtime` CLI tool to conveniently execute WebAssembly modules from
the command line.

The `wasmtime` CLI executes the WebAssembly module or component provided to it
as a CLI argument:

```sh
$ wasmtime foo.wasm
```

CLI arguments can be passed to the WebAssembly file itself too:

```sh
$ wasmtime example.wasm arg1 --flag1 arg2
```

For more information be sure to check out [how to install the
CLI](cli-install.md), [the list of options you can
pass](cli-options.md), and [how to enable logging](cli-logging.md).
