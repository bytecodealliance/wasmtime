# `wasi_snapshot_preview1.wasm`

> **Note**: This repository is a work in progress. This is intended to be an
> internal tool which not everyone has to look at but many might rely on. You
> may need to reach out via issues or
> [Zulip](https://bytecodealliance.zulipchat.com/) to learn more about this
> repository.

This repository currently contains an implementation of a WebAssembly module:
`wasi_snapshot_preview1.wasm`. This module bridges the `wasi_snapshot_preview1`
ABI to the preview2 ABI of the component model. At this time the preview2 APIs
themselves are not done being specified so a local copy of `wit/*.wit` is used
instead.

## Building

This adapter can be built with:

```sh
$ cargo build -p wasi-preview1-component-adapter --target wasm32-unknown-unknown --release
```

And the artifact will be located at
`target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm`.

This by default builds a "reactor" adapter which means that it only provides
adaptation from preview1 to preview2. Alternatively you can also build a
"command" adapter by passing `--features command --no-default-features` which
will additionally export a `run` function entrypoint. This is suitable for use
with preview1 binaries that export a `_start` function.

Alternatively the latest copy of the command and reactor adapters can be
[downloaded from the `dev` tag assets][dev-tag]

[dev-tag]: https://github.com/bytecodealliance/wasmtime/releases/tag/dev

## Using

With a `wasi_snapshot_preview1.wasm` file on-hand you can create a component
from a module that imports WASI functions using the [`wasm-tools`
CLI](https://github.com/bytecodealliance/wasm-tools)

```sh
$ cat foo.rs
fn main() {
    println!("Hello, world!");
}
$ rustc foo.rs --target wasm32-wasip1
$ wasm-tools print foo.wasm | grep '(import'
  (import "wasi_snapshot_preview1" "fd_write" (func ...
  (import "wasi_snapshot_preview1" "environ_get" (func ...
  (import "wasi_snapshot_preview1" "environ_sizes_get" ...
  (import "wasi_snapshot_preview1" "proc_exit" (func ...
$ wasm-tools component new foo.wasm --adapt wasi_snapshot_preview1.wasm -o component.wasm

# Inspect the generated `component.wasm`
$ wasm-tools validate component.wasm --features component-model
$ wasm-tools component wit component.wasm
```

Here the `component.wasm` that's generated is a ready-to-run component which
imports wasi preview2 functions and is compatible with the wasi-preview1-using
module internally.
