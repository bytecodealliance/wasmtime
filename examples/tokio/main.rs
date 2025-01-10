use anyhow::Error;
use std::sync::Arc;
use tokio::time::Duration;
use wasmtime::{Config, Engine, Linker, Module, Store};
// For this example we want to use the async version of wasi_common.
// Notably, this version of wasi uses a scheduler that will async yield
// when sleeping in `poll_oneoff`.
use wasi_common::{WasiCtx, tokio::WasiCtxBuilder};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create an environment shared by all wasm execution. This contains
    // the `Engine` and the `Module` we are executing.
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

#[derive(Clone)]
struct Environment {
    engine: Engine,
    module: Module,
    linker: Arc<Linker<WasiCtx>>,
}

impl Environment {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::new();
        // We need this engine's `Store`s to be async, and consume fuel, so
        // that they can co-operatively yield during execution.
        config.async_support(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let module = Module::from_file(&engine, "target/wasm32-wasip1/debug/tokio-wasi.wasm")?;

        // A `Linker` is shared in the environment amongst all stores, and this
        // linker is used to instantiate the `module` above. This example only
        // adds WASI functions to the linker, notably the async versions built
        // on tokio.
        let mut linker = Linker::new(&engine);
        wasi_common::tokio::add_to_linker(&mut linker, |cx| cx)?;

        Ok(Self {
            engine,
            module,
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
    let wasi = WasiCtxBuilder::new()
        // Let wasi print to this process's stdout.
        .inherit_stdout()
        // Set an environment variable so the wasm knows its name.
        .env("NAME", &inputs.name)?
        .build();
    let mut store = Store::new(&inputs.env.engine, wasi);

    // Put effectively unlimited fuel so it can run forever.
    store.set_fuel(u64::MAX)?;
    // WebAssembly execution will be paused for an async yield every time it
    // consumes 10000 fuel.
    store.fuel_async_yield_interval(Some(10000))?;

    // Instantiate into our own unique store using the shared linker, afterwards
    // acquiring the `_start` function for the module and executing it.
    let instance = inputs
        .env
        .linker
        .instantiate_async(&mut store, &inputs.env.module)
        .await?;
    instance
        .get_typed_func::<(), ()>(&mut store, "_start")?
        .call_async(&mut store, ())
        .await?;

    Ok(())
}
