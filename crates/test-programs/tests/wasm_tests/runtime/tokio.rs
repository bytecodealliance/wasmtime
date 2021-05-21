use anyhow::Context;
use std::path::Path;
use wasi_common::pipe::WritePipe;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::tokio::{Wasi, WasiCtxBuilder};

pub fn instantiate(data: &[u8], bin_name: &str, workspace: Option<&Path>) -> anyhow::Result<()> {
    run(data, bin_name, workspace, false)
}
pub fn instantiate_inherit_stdio(
    data: &[u8],
    bin_name: &str,
    workspace: Option<&Path>,
) -> anyhow::Result<()> {
    run(data, bin_name, workspace, true)
}

fn run(
    data: &[u8],
    bin_name: &str,
    workspace: Option<&Path>,
    inherit_stdio: bool,
) -> anyhow::Result<()> {
    let stdout = WritePipe::new_in_memory();
    let stdout_ = stdout.clone();
    let stderr = WritePipe::new_in_memory();
    let stderr_ = stderr.clone();

    let r = tokio::runtime::Runtime::new()
        .expect("create runtime")
        .block_on(async move {
            let mut config = Config::new();
            config.async_support(true);
            Wasi::add_to_config(&mut config);
            let engine = Engine::new(&config)?;
            let store = Store::new(&engine);

            // Create our wasi context.
            let mut builder = WasiCtxBuilder::new();

            if inherit_stdio {
                builder = builder.inherit_stdio();
            } else {
                builder = builder
                    .stdout(Box::new(stdout_.clone()))
                    .stderr(Box::new(stderr_.clone()));
            }

            builder = builder.arg(bin_name)?.arg(".")?;

            if let Some(workspace) = workspace {
                println!("preopen: {:?}", workspace);
                let preopen_dir = unsafe { cap_std::fs::Dir::open_ambient_dir(workspace) }?;
                builder = builder.preopened_dir(preopen_dir, ".")?;
            }

            for (var, val) in super::test_suite_environment() {
                builder = builder.env(var, val)?;
            }

            // tokio does not yet support the sync family of fdflags, because cap-std-sync
            // does not.
            builder = builder.env("NO_FDFLAGS_SYNC_SUPPORT", "1")?;

            Wasi::set_context(&store, builder.build())
                .map_err(|_| anyhow::anyhow!("wasi set_context failed"))?;

            let module =
                Module::new(store.engine(), &data).context("failed to create wasm module")?;
            let linker = Linker::new(&store);
            let instance = linker.instantiate_async(&module).await?;
            let start = instance.get_typed_func::<(), ()>("_start")?;
            start.call_async(()).await.map_err(anyhow::Error::from)
        });

    match r {
        Ok(()) => Ok(()),
        Err(trap) => {
            let stdout = stdout
                .try_into_inner()
                .expect("sole ref to stdout")
                .into_inner();
            if !stdout.is_empty() {
                println!("guest stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
            }
            let stderr = stderr
                .try_into_inner()
                .expect("sole ref to stderr")
                .into_inner();
            if !stderr.is_empty() {
                println!("guest stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
            }
            Err(trap.context(format!("error while testing Wasm module '{}'", bin_name,)))
        }
    }
}
