# WASI

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi/main.rs

This example shows how to use the [`wasmtime-wasi`] crate to define WASI
functions within a [`Linker`] which can then be used to instantiate a
WebAssembly module.

[`wasmtime-wasi`]: https://crates.io/crates/wasmtime-wasi
[`Linker`]: https://docs.rs/wasmtime/*/wasmtime/struct.Linker.html

### WebAssembly module source code

For this WASI example, this Hello World program is compiled to a WebAssembly module using the WASI Preview 2 API.

`wasi.rs`
```rust
{{#include ../examples/wasi/wasm/wasi.rs}}
```

> Building instructions:
> 1. Update your Rust toolchain if it is not >= 1.83.0, as Rust 1.83.0 brings Tier 2 support for WASI Preview 2.
>   `rustup update stable`
> 2. Add WASIp2 target if you haven't already: `rustup target add wasm32-wasip2`
> 3. `cargo build --target wasm32-wasip2`

Building this program generates `target/wasm32-wasip2/debug/wasi.wasm`, used below.

### Invoke the WASM component

This example shows adding and configuring the WASI imports to invoke the above WASM component.

`main.rs`
```rust,ignore
{{#include ../examples/wasi/main.rs}}
```

TODO: Update the following section after `../examples/wasi/main.rs` is fixed

## WASI state with other custom host state

The [`add_to_linker`] takes a second argument which is a closure to access `&mut
WasiCtx` from within the `T` stored in the `Store<T>` itself. In the above
example this is trivial because the `T` in `Store<T>` is `WasiCtx` itself, but
you can also store other state in `Store` like so:

[`add_to_linker`]: https://docs.rs/wasi-common/*/wasi_common/sync/fn.add_to_linker.html
[`Store`]: https://docs.rs/wasmtime/*/wasmtime/struct.Store.html
[`BorrowMut<WasiCtx>`]: https://doc.rust-lang.org/stable/std/borrow/trait.BorrowMut.html
[`WasiCtx`]: https://docs.rs/wasi-common/*/wasi_common/struct.WasiCtx.html

```rust
# extern crate wasmtime;
# extern crate wasi_common;
# extern crate anyhow;
use anyhow::Result;
use std::borrow::{Borrow, BorrowMut};
use wasmtime::*;
use wasi_common::{WasiCtx, sync::WasiCtxBuilder};

struct MyState {
    message: String,
    wasi: WasiCtx,
}

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasi_common::sync::add_to_linker(&mut linker, |state: &mut MyState| &mut state.wasi)?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    let mut store = Store::new(&engine, MyState {
        message: format!("hello!"),
        wasi,
    });

    // ...

# let _linker: Linker<MyState> = linker;
    Ok(())
}
```

## WASI Preview 2

An experimental implementation of the WASI Preview 2 API is also available, along with an adapter layer for  WASI Preview 1 WebAssembly modules. In future this `preview2` API will become the default. There are some features which are currently only accessible through the `preview2` API such as async support and overriding the clock and random implementations.

### Async example

This [async example code][code2] shows how to use the [wasmtime-wasi::preview2][`preview2`] module to
execute the same WASI Preview 1 WebAssembly module from the example above. This example requires the `wasmtime` crate `async` feature to be enabled.

This does not require any change to the WebAssembly module, it's just the WASI API host functions which are implemented to be async. See [wasmtime async support](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_support).

[code2]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi-async/main.rs
[`preview2`]: https://docs.rs/wasmtime-wasi/*/wasmtime_wasi/preview2/index.html

```rust,ignore
{{#include ../examples/wasi-async/main.rs}}
```

You can also [browse this source code online][code2] and clone the wasmtime
repository to run the example locally.
