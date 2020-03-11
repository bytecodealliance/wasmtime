# Running `hello-world.wasm` with Wasmtime

Now you have built the `hello-world.wasm` binary (If you haven't,
the instructions for doing so are in the
[previous section](https://bytecodealliance.github.io/wasmtime/tutorial-create-hello-world.html)).

You can then run the binary with Wasmtime, like so:

```sh
$ wasmtime target/wasm32-wasi/debug/hello-world.wasm
```
