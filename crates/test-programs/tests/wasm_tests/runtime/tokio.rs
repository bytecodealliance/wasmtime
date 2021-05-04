use anyhow::Context;
use std::path::Path;
use wasi_common::pipe::WritePipe;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::tokio::{Wasi, WasiCtxBuilder};

pub fn instantiate(data: &[u8], bin_name: &str, workspace: Option<&Path>) -> anyhow::Result<()> {
    let stdout = WritePipe::new_in_memory();
    let stdout_ = stdout.clone();
    let stderr = WritePipe::new_in_memory();
    let stderr_ = stderr.clone();

    let r = tokio::runtime::Runtime::new()
        .expect("create runtime")
        .block_on(async move {
            let mut config = Config::new();
            config.async_support(true);
            config.consume_fuel(true);
            Wasi::add_to_config(&mut config);
            let engine = Engine::new(&config)?;
            let store = Store::new(&engine);

            // Create our wasi context.
            // Additionally register any preopened directories if we have them.
            let mut builder = WasiCtxBuilder::new();

            builder = builder
                .arg(bin_name)?
                .arg(".")?
                .stdout(Box::new(stdout_))
                .stderr(Box::new(stderr_));

            if let Some(workspace) = workspace {
                println!("preopen: {:?}", workspace);
                let preopen_dir = unsafe { cap_std::fs::Dir::open_ambient_dir(workspace) }?;
                builder = builder.preopened_dir(preopen_dir, ".")?;
            }

            #[cfg(windows)]
            {
                builder = builder
                    .env("ERRNO_MODE_WINDOWS", "1")?
                    .env("NO_DANGLING_FILESYSTEM", "1")?
                    .env("NO_FD_ALLOCATE", "1")?
                    .env("NO_RENAME_DIR_TO_EMPTY_DIR", "1")?
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                builder = builder.env("ERRNO_MODE_UNIX", "1")?;
            }
            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .env("ERRNO_MODE_MACOS", "1")?
                    .env("NO_FD_ALLOCATE", "1")?;
            }

            // cap-std-sync does not yet support the sync family of fdflags
            builder = builder.env("NO_FDFLAGS_SYNC_SUPPORT", "1")?;

            store.out_of_fuel_async_yield(u32::MAX, 10000);
            Wasi::set_context(&store, builder.build()?)
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

pub fn instantiate_inherit_stdio(
    data: &[u8],
    bin_name: &str,
    workspace: Option<&Path>,
) -> anyhow::Result<()> {
    let r = tokio::runtime::Runtime::new()
        .expect("create runtime")
        .block_on(async {
            let mut config = Config::new();
            config.async_support(true);
            config.consume_fuel(true);
            Wasi::add_to_config(&mut config);
            let engine = Engine::new(&config)?;
            let store = Store::new(&engine);

            // Create our wasi context.
            // Additionally register any preopened directories if we have them.
            let mut builder = WasiCtxBuilder::new();

            builder = builder.arg(bin_name)?.arg(".")?.inherit_stdio();

            if let Some(workspace) = workspace {
                println!("preopen: {:?}", workspace);
                let preopen_dir = unsafe { cap_std::fs::Dir::open_ambient_dir(workspace) }?;
                builder = builder.preopened_dir(preopen_dir, ".")?;
            }

            store.out_of_fuel_async_yield(u32::MAX, 10000);
            Wasi::set_context(&store, builder.build()?)
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
        Err(trap) => Err(trap.context(format!("error while testing Wasm module '{}'", bin_name,))),
    }
}
