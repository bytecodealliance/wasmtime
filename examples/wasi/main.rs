//! Example of instantiating a wasm module which uses WASI imports.

/*
You can execute this example with:
    cmake examples/
    cargo run --example wasi
*/

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::*;
use wasmtime_wasi::bindings::sync::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

pub struct ComponentRunStates {
    // These two are required basically as a standard way to enable the impl of WasiView
    // impl of WasiView is required by [`wasmtime_wasi::add_to_linker_sync`]
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    // You can add other custom host states if needed
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
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    // Instantiate our component with the imports we've created, and run it.
    let component = Component::from_file(&engine, "target/wasm32-wasip2/debug/wasi.wasm")?;
    let command = Command::instantiate(&mut store, &component, &linker)?;
    let program_result = command.wasi_cli_run().call_run(&mut store)?;
    if program_result.is_err() {
        std::process::exit(1)
    }

    // Alternatively, instead of using `Command`, just instantiate it as a normal component
    // New states
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    // Instantiate it as a normal component
    let instance = linker.instantiate(&mut store, &component)?;
    // Get the index for the exported interface
    let interface_idx = instance.get_export(&mut store, None, "wasi:cli/run@0.2.0").unwrap();
    // Get the index for the exported function in the exported interface
    let parent_export_idx = Some(&interface_idx);
    let func_idx = instance.get_export(&mut store, parent_export_idx, "run").unwrap();
    let func = instance.get_func(&mut store, func_idx).unwrap();
    // As the `run` function in `wasi:cli/run@0.2.0` takes no argument and return a WASI result that correspond to a `Result<(), ()>`
    // Reference:
    // * https://github.com/WebAssembly/wasi-cli/blob/main/wit/run.wit
    // * Documentation for [Func::typed](https://docs.rs/wasmtime/latest/wasmtime/component/struct.Func.html#method.typed) and [ComponentNamedList](https://docs.rs/wasmtime/latest/wasmtime/component/trait.ComponentNamedList.html)
    let typed = func.typed::<(), (Result<(), ()>,)>(&store)?;
    let (result,) = typed.call(&mut store, ())?;
    result.map_err(|_| anyhow::anyhow!("error"))
}
