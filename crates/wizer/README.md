<div align="center">
  <h1>Wizer</h1>

  <p>
    <strong>The WebAssembly Pre-Initializer!</strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://bytecodealliance.zulipchat.com/#narrow/stream/223391-wasm"><img src="https://img.shields.io/badge/zulip-join_chat-brightgreen.svg" alt="zulip chat" /></a>
    <a href="https://docs.rs/wasmtime-wizer"><img src="https://docs.rs/wasmtime-wizer/badge.svg" alt="Documentation Status" /></a>
  </p>

  <h3>
    <a href="https://docs.rs/wasmtime-wizer">API Docs</a>
    <span> | </span>
    <a href="https://github.com/bytecodealliance/wasmtime/blob/main/CONTRIBUTING.md">Contributing</a>
    <span> | </span>
    <a href="https://bytecodealliance.zulipchat.com/#narrow/stream/223391-wasm">Chat</a>
  </h3>
</div>

* [About](#about)
* [Install](#install)
* [Example Usage](#example-usage)
* [Caveats](#caveats)
* [Using Wizer as a Library](#using-wizer-as-a-library)
* [How Does it Work?](#how-does-it-work)

## About

Don't wait for your Wasm module to initialize itself, pre-initialize it! Wizer
instantiates your WebAssembly module, executes its initialization function, and
then snapshots the initialized state out into a new WebAssembly module. Now you
can use this new, pre-initialized WebAssembly module to hit the ground running,
without making your users wait for that first-time set up code to complete.

The improvements to start up latency you can expect will depend on how much
initialization work your WebAssembly module needs to do before it's ready. Some
initial benchmarking shows between 1.35 to 6.00 times faster instantiation and
initialization with Wizer, depending on the workload:

| Program                | Without Wizer | With Wizer | Speedup          |
|------------------------|--------------:|-----------:|-----------------:|
| [`regex`][regex-bench] | 248.85 us     | 183.99 us  | **1.35x faster** |
| [UAP][uap-bench]       | 98.297 ms     | 16.385 ms  | **6.00x faster** |

[regex-bench]: https://github.com/bytecodealliance/wasmtime/tree/main/crates/wizer/benches/regex-bench
[uap-bench]: https://github.com/bytecodealliance/wasmtime/tree/main/crates/wizer/benches/uap-bench

Not every program will see an improvement to instantiation and start up
latency. For example, Wizer will often increase the size of the Wasm module's
`Data` section, which could negatively impact network transfer times on the
Web. However, the best way to find out if your Wasm module will see an
improvement is to try it out! Adding an initialization function isn't too hard.

Finally, you can likely see further improvements by running
[`wasm-opt`][binaryen] on the pre-initialized module. Beyond the usual benefits
that `wasm-opt` brings, the module likely has a bunch of initialization-only
code that is no longer needed now that the module is already initialized, and
which `wasm-opt` can remove.

[binaryen]: https://github.com/WebAssembly/binaryen

## Example Usage

First, make sure your Wasm module exports an initialization function named
`wizer-initialize`. For example, in Rust you can export it like this:

```rust
#[unsafe(export_name = "wizer-initialize")]
pub extern "C" fn init() {
    // Your initialization code goes here...
}
```

For a complete C++ example, see [this](https://github.com/bytecodealliance/wizer/tree/main/examples/cpp).

Then, if your Wasm module is named `input.wasm`, run the `wizer` CLI:

```shell-session
wasmtime wizer input.wasm -o initialized.wasm
```

Now you have a pre-initialized version of your Wasm module at
`initialized.wasm`!

More details, flags, and options can be found via `--help`:

```shell-session
wasmtime wizer --help
```

## Caveats

* The initialization function may not call any imported functions. Doing so will
  trigger a trap and `wizer` will exit. You can, however, allow WASI calls via
  the `--allow-wasi` flag.

* The Wasm module may not import globals, tables, or memories.

* Reference types are not supported yet. It isn't 100% clear yet what the best
  approach to snapshotting `externref` tables is.

## Using Wizer as a Library

Add a dependency in your `Cargo.toml`:

```shell-session
cargo add wasmtime-wizer
```

And then use the `wizer::Wizer` builder to configure and run Wizer:

```rust
use wizer::Wizer;

let input_wasm = get_input_wasm_bytes();
let initialized_wasm_bytes = Wizer::new()
    .run(&mut store, &input_wasm, |store, module| {
        linker.instantiate(store, module)
    })?;
```

## How Does it Work?

First we instantiate the input Wasm module with Wasmtime and run the
initialization function. Then we record the Wasm instance's state:

* What are the values of its globals?
* What regions of memory are non-zero?

Then we rewrite the Wasm binary by intializing its globals directly to their
recorded state, and removing the module's old data segments and replacing them
with data segments for each of the non-zero regions of memory we recorded.

Want some more details? Check out the talk ["Hit the Ground Running: Wasm
Snapshots for Fast Start
Up"](https://fitzgeraldnick.com/2021/05/10/wasm-summit-2021.html) from the 2021
WebAssembly Summit.
