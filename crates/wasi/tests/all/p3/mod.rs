use std::path::Path;

use anyhow::{Context as _, anyhow};
use wasmtime::Store;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::p3::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi::{DirPerms, FilePerms};

use test_programs_artifacts::*;

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to assert it exists")]
        use self::$name as _;
    };
}

struct Ctx {
    table: ResourceTable,
    p2: wasmtime_wasi::p2::WasiCtx,
    p3: WasiCtx,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.p3
    }
}

// TODO: Remove once test components are not built for `wasm32-wasip1`
impl wasmtime_wasi::p2::WasiView for Ctx {
    fn ctx(&mut self) -> &mut wasmtime_wasi::p2::WasiCtx {
        &mut self.p2
    }
}

// TODO: Remove once test components are not built for `wasm32-wasip1`
impl wasmtime_wasi::p2::IoView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

async fn run(path: &str) -> anyhow::Result<()> {
    let path = Path::new(path);
    let engine = test_programs_artifacts::engine(|config| {
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let component = Component::from_file(&engine, path).context("failed to compile component")?;

    let mut linker = Linker::new(&engine);
    // TODO: Remove once test components are not built for `wasm32-wasip1`
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;

    let mut builder = WasiCtxBuilder::new();
    let name = path.file_stem().unwrap().to_str().unwrap();
    let tempdir = tempfile::Builder::new()
        .prefix(&format!("wasi_components_{name}_",))
        .tempdir()?;
    builder
        .args(&[name, "."])
        .inherit_network()
        .allow_ip_name_lookup(true);
    println!("preopen: {tempdir:?}");
    builder.preopened_dir(tempdir.path(), ".", DirPerms::all(), FilePerms::all())?;
    for (var, val) in test_programs_artifacts::wasi_tests_environment() {
        builder.env(var, val);
    }
    let table = ResourceTable::default();
    let p2 = wasmtime_wasi::p2::WasiCtx::builder().build();
    let p3 = builder.build();
    let mut store = Store::new(&engine, Ctx { table, p2, p3 });
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let command =
        Command::new(&mut store, &instance).context("failed to instantiate `wasi:cli/command`")?;
    let run = command.wasi_cli_run().call_run(&mut store);
    instance
        .run(&mut store, run)
        .await
        .context("failed to call `wasi:cli/run#run`")?
        .context("guest trapped")?
        .map_err(|()| anyhow!("`wasi:cli/run#run` failed"))
}

foreach_p3!(assert_test_exists);

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_clocks_sleep() -> anyhow::Result<()> {
    run(P3_CLOCKS_SLEEP_COMPONENT).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn p3_random_imports() -> anyhow::Result<()> {
    run(P3_RANDOM_IMPORTS_COMPONENT).await
}
