use std::sync::Arc;
use tokio::time::Duration;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Error, Store};
use wasmtime_wasi::p2::bindings::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create an environment shared by all wasm execution. This contains
    // the `Engine` and the `Component` we are executing.
    let env = Environment::new()?;

    // The inputs to run_wasm are `Send`: we can create them here and send
    // them to a new task that we spawn.
    let inputs1 = Inputs::new(env.clone(), "Gussie");
    let inputs2 = Inputs::new(env.clone(), "Willa");
    let inputs3 = Inputs::new(env, "Sparky");

    // Spawn some tasks. Insert sleeps before run_wasm so that the
    // interleaving is easy to observe.
    let join1 = tokio::task::spawn(async move { run_wasm(inputs1).await });
    let join2 = tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(750)).await;
        run_wasm(inputs2).await
    });
    let join3 = tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1250)).await;
        run_wasm(inputs3).await
    });

    // All tasks should join successfully.
    join1.await??;
    join2.await??;
    join3.await??;
    Ok(())
}

/// The per-store host state: a WASI context plus the resource table that
/// host resources (including WASI) live ni.
struct ComponentRunStates {
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

#[derive(Clone)]
struct Environment {
    engine: Engine,
    component: Component,
    linker: Arc<Linker<ComponentRunStates>>,
}

impl Environment {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::new();
        // Consume fuel for guests so that they can co-operatively yield during
        // execution.
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let component =
            Component::from_file(&engine, "target/wasm32-wasip2/debug/tokio-wasi.wasm")?;

        // A `Linker` is shared in the environment amongst all stores, and this
        // linker is used to instantiate the `component` above. This example
        // only adds WASI functions to the linker, notably the async versions
        // built on tokio.
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

        Ok(Self {
            engine,
            component,
            linker: Arc::new(linker),
        })
    }
}

struct Inputs {
    env: Environment,
    name: String,
}

impl Inputs {
    fn new(env: Environment, name: &str) -> Self {
        Self {
            env,
            name: name.to_owned(),
        }
    }
}

async fn run_wasm(inputs: Inputs) -> Result<(), Error> {
    let wasi = WasiCtx::builder()
        // Let wasi print to this process's stdout.
        .inherit_stdout()
        // Set an environment variable so the wasm knows its name.
        .env("NAME", &inputs.name)
        .build();
    let state = ComponentRunStates {
        wasi_ctx: wasi,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&inputs.env.engine, state);

    // Put effectively unlimited fuel so it can run forever.
    store.set_fuel(u64::MAX)?;
    // WebAssembly execution will be paused for an async yield every time it
    // consumes 10000 fuel.
    store.fuel_async_yield_interval(Some(10000))?;

    // Instantiate into our own unique store using the shared linker, afterwards
    // acquiring the `wasi:cli/run` export of the component and executing it.
    let command =
        Command::instantiate_async(&mut store, &inputs.env.component, &inputs.env.linker).await?;
    command
        .wasi_cli_run()
        .call_run(&mut store)
        .await?
        .map_err(|()| wasmtime::format_err!("{} exited with an error", inputs.name))?;

    Ok(())
}
