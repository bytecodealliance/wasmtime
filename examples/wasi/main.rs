//! Example of instantiating a wasm module which uses WASI imports.

/*
You can execute this example with:
    cmake examples/
    cargo run --example wasi
*/

use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder};
use wasmtime::component::{Component, Linker, ResourceTable};

pub struct ComponentRunStates {
    // These two are required basically as a standard way to enable the impl of WasiView
    // impl of WasiView is required by [`wasmtime_wasi::add_to_linker_sync`]
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

fn main() -> Result<()> {
    // Define the WASI functions globally on the `Config`.
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    // Create a WASI context and put it in a Store; all instances in the store
    // share this context. `WasiCtxBuilder` provides a number of ways to
    // configure what the target program will have access to.
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()
        .build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    // Instantiate our component with the imports we've created, and run it.
    let component = Component::from_file(&engine, "target/wasm32-wasip2/debug/wasi.wasm")?;
    let instance = linker.instantiate(&mut store, &component)?;
    let exp_idx= instance.get_export(&mut store, None, "wasi:cli/run@0.2.0").unwrap();
    // FIXME: Why can't I get an exported function here? The exp_idx is valid
    let func = instance.get_func(&mut store, exp_idx).unwrap();
    let typed = func.typed::<(), ()>(&store)?;
    typed.call(&mut store, ())?;
    Ok(())
}
