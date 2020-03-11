# Running `hello-world.wasm` with Wasmtime

## Installing Wasmtime

The Wasmtime CLI can be installed on Linux and macOS with a small install
script:

```sh
$ curl https://wasmtime.dev/install.sh -sSf | bash
```

Windows or otherwise interested users can download installers and binaries
directly from the
[GitHub Releases](https://github.com/bytecodealliance/wasmtime/releases) page.

## Running `hello-world.wasm`

There are a number of ways to run a `.wasm` file with Wasmtime. In this
tutorial, we'll be using the CLI, Wasmtime can also be embedded in your
applications. More information on this can be found in the
[Embedding Wasmtime section](https://bytecodealliance.github.io/wasmtime/embed.html).

If you've built the `hello-world.wasm` file (the instructions for doing so are in the
[previous section](https://bytecodealliance.github.io/wasmtime/tutorial-create-hello-world.html)),
you can run it with Wasmtime from the command line like so:

```sh
$ wasmtime target/wasm32-wasi/debug/hello-world.wasm
```
