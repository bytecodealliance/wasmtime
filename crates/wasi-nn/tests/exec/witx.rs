use super::PREOPENED_DIR_NAME;
use crate::check::artifacts_dir;
use anyhow::Result;
use std::path::Path;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::{preview1::WasiP1Ctx, DirPerms, FilePerms, WasiCtxBuilder};
use wasmtime_wasi_nn::{witx::WasiNnCtx, Backend, InMemoryRegistry};

/// Run a wasi-nn test program. This is modeled after
/// `crates/wasi/tests/all/main.rs` but still uses the older preview1 API
/// for file reads.
pub fn run(path: &str, backend: Backend, preload_model: bool) -> Result<()> {
    let path = Path::new(path);
    let engine = Engine::new(&Config::new())?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi_nn::witx::add_to_linker(&mut linker, |s: &mut Ctx| &mut s.wasi_nn)?;
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |s: &mut Ctx| &mut s.wasi)?;
    let module = Module::from_file(&engine, path)?;
    let mut store = Store::new(&engine, Ctx::new(&artifacts_dir(), preload_model, backend)?);
    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;
    Ok(())
}

/// The host state for running wasi-nn tests.
struct Ctx {
    wasi: WasiP1Ctx,
    wasi_nn: WasiNnCtx,
}

impl Ctx {
    fn new(preopen_dir: &Path, preload_model: bool, mut backend: Backend) -> Result<Self> {
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdio().preopened_dir(
            preopen_dir,
            PREOPENED_DIR_NAME,
            DirPerms::READ,
            FilePerms::READ,
        )?;
        let wasi = builder.build_p1();

        let mut registry = InMemoryRegistry::new();
        let mobilenet_dir = artifacts_dir();
        if preload_model {
            registry.load((backend).as_dir_loadable().unwrap(), &mobilenet_dir)?;
        }
        let wasi_nn = WasiNnCtx::new([backend.into()], registry.into());

        Ok(Self { wasi, wasi_nn })
    }
}
