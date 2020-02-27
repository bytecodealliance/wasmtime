# Using the `wasmtime` CLI

In addition to the embedding API which allows you to use Wasmtime as a
library, the Wasmtime project also provies a `wasmtime` CLI tool to conveniently
execute WebAssembly modules from the command line.

This section will provide a guide to the `wasmtime` CLI and major functionality
that it contains. In short, however, you can execute a WebAssembly file
(actually doing work as part of the `start` function) like so:

```sh
$ wasmtime foo.wasm
```

Or similarly if you want to invoke a "start" function, such as with WASI
modules, you can execute

```sh
$ wasmtime --invoke _start foo.wasm
```

For more information be sure to check out [how to install the
CLI](cli-install.md) as well as [the list of options you can
pass](cli-options.md).
