# Running `hello-world.wasm` with Wasmtime

## Installing Wasmtime

The Wasmtime CLI can be installed on Linux and macOS with a small install
script:

```sh
$ curl https://wasmtime.dev/install.sh -sSf | bash
```

You can find more information about installing the Wasmtime CLI in the
[CLI Installation section](./cli-install.md)

## Running `hello-world.wasm`

There are a number of ways to run a `.wasm` file with Wasmtime. In this
tutorial, we'll be using the CLI, Wasmtime can also be embedded in your
applications. More information on this can be found in the
[Embedding Wasmtime section](./lang.md).

If you've built the `hello-world.wasm` file (the instructions for doing so are in the
[previous section](./tutorial-create-hello-world.md)),
you can run it with Wasmtime from the command line like so:

```sh
$ wasmtime target/wasm32-wasi/debug/hello-world.wasm
```
