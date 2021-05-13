use anyhow::{anyhow, Error};
use std::future::Future;
use tokio::time::Duration;
use wasmtime::{Config, Engine, Linker, Module, Store};
// For this example we want to use the async version of wasmtime_wasi.
// Notably, this version of wasi uses a scheduler that will async yield
// when sleeping in `poll_oneoff`.
use wasmtime_wasi::tokio::{Wasi, WasiCtxBuilder};

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
}

impl Environment {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::new();
        // We need this engine's `Store`s to be async, and consume fuel, so
        // that they can co-operatively yield during execution.
        config.async_support(true);
        config.consume_fuel(true);

        // Install the host functions for `Wasi`.
        Wasi::add_to_config(&mut config);

        let engine = Engine::new(&config)?;
        let module = Module::from_file(&engine, "target/wasm32-wasi/debug/tokio-wasi.wasm")?;

        Ok(Self { engine, module })
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

fn run_wasm(inputs: Inputs) -> impl Future<Output = Result<(), Error>> {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    // IMPORTANT: The current wasmtime API is very challenging to use safely
    // on an async runtime. This RFC describes a redesign of the API that will
    // resolve these safety issues:
    // https://github.com/alexcrichton/rfcs-2/blob/new-api/accepted/new-api.md

    // This is a "marker type future" which simply wraps some other future and
    // the only purpose it serves is to forward the implementation of `Future`
    // as well as have `unsafe impl Send` for itself, regardless of the
    // underlying type.
    //
    // Note that the qctual safety of this relies on the fact that the inputs
    // here are `Send`, the outputs (just () in this case) are `Send`, and the
    // future itself is safe tu resume on other threads.
    //
    // For an in-depth discussion of the safety of moving Wasmtime's `Store`
    // between threads, see
    // https://docs.wasmtime.dev/examples-rust-multithreading.html.
    struct UnsafeSend<T>(T);

    // Note the `where` cause specifically ensures the output of the future to
    // be `Send` is required. We specifically dont require `T` to be `Send`
    // since that's the whole point of this function, but we require that
    // everything used to construct `T` is `Send` below.
    unsafe impl<T> Send for UnsafeSend<T>
    where
        T: Future,
        T::Output: Send,
    {
    }
    impl<T: Future> Future for UnsafeSend<T> {
        type Output = T::Output;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T::Output> {
            // Note that this `unsafe` is unrelated to `Send`, it only has to do with "pin
            // projection" and should be safe since it's all we do with the `Pin`.
            unsafe { self.map_unchecked_mut(|p| &mut p.0).poll(cx) }
        }
    }

    // This is a crucial assertion that needs to be here. The compiler
    // typically checks this for us, but do to our `UnsafeSend` type the
    // compiler isn't automatically checking  this. The assertion here must
    // assert that all arguments to this function are indeed `Send` because
    // we're closing over them and sending them to other threads. It's only
    // everything *internal* to the computation of this function which doesn't
    // have to be `Send`.
    fn assert_send<T: Send>(_t: &T) {}
    assert_send(&inputs);

    // Wrap up the `_run_wasm` function, which is *not* `Send`, but is safe to
    // resume on other threads.
    UnsafeSend(_run_wasm(inputs))
}

async fn _run_wasm(inputs: Inputs) -> Result<(), Error> {
    let store = Store::new(&inputs.env.engine);

    // WebAssembly execution will be paused for an async yield every time it
    // consumes 10000 fuel. Fuel will be refilled u32::MAX times.
    store.out_of_fuel_async_yield(u32::MAX, 10000);

    Wasi::set_context(
        &store,
        WasiCtxBuilder::new()
            // Let wasi print to this process's stdout.
            .inherit_stdout()
            // Set an environment variable so the wasm knows its name.
            .env("NAME", &inputs.name)?
            .build()?,
    )
    .map_err(|_| anyhow!("setting wasi context"))?;

    let linker = Linker::new(&store);

    // Instantiate
    let instance = linker.instantiate_async(&inputs.env.module).await?;
    instance
        .get_typed_func::<(), ()>("_start")?
        .call_async(())
        .await?;

    Ok(())
}
