# WASI

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasi/main.rs

This example shows how to use the [`wasmtime-wasi`] crate to define WASI
functions within a [`Linker`] which can then be used to instantiate a
WebAssembly module.

[`wasmtime-wasi`]: https://crates.io/crates/wasmtime-wasi
[`Linker`]: https://docs.rs/wasmtime/*/wasmtime/struct.Linker.html

## Wasm Source code

```rust
{{#include ../examples/wasi/wasm/wasi.rs}}
```

## `wasi.rs`

```rust,ignore
{{#include ../examples/wasi/main.rs}}
```

## WASI state with other custom host state

The [`add_to_linker`] function requires that the host state of a [`Store`]
implements the [`BorrowMut<WasiCtx>`] trait. In the above example this is true
because the host state itself is [`WasiCtx`], but you may also have custom host
state you'd like to store adjacent to the [`WasiCtx`]. An example of doing this
looks like:

[`add_to_linker`]: https://docs.rs/wasmtime-wasi/*/wasmtime_wasi/sync/fn.add_to_linker.html
[`Store`]: https://docs.rs/wasmtime/0.26.0/wasmtime/struct.Store.html
[`BorrowMut<WasiCtx>`]: https://doc.rust-lang.org/stable/std/borrow/trait.BorrowMut.html
[`WasiCtx`]: https://docs.rs/wasmtime-wasi/*/wasmtime_wasi/struct.WasiCtx.html

```rust
# extern crate wasmtime;
# extern crate wasmtime_wasi;
# extern crate anyhow;
use anyhow::Result;
use std::borrow::{Borrow, BorrowMut};
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, sync::WasiCtxBuilder};

struct MyState {
    message: String,
    wasi: WasiCtx,
}

impl Borrow<WasiCtx> for MyState {
    fn borrow(&self) -> &WasiCtx {
        &self.wasi
    }
}

impl BorrowMut<WasiCtx> for MyState {
    fn borrow_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker)?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build()?;
    let mut store = Store::new(&engine, MyState {
        message: format!("hello!"),
        wasi,
    });

    // ...

# let _linker: Linker<MyState> = linker;
    Ok(())
}
```
