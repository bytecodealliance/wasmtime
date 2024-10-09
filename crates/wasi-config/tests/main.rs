use anyhow::{anyhow, Result};
use test_programs_artifacts::{foreach_config, CONFIG_GET_COMPONENT};
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Store,
};
use wasmtime_wasi::{add_to_linker_async, bindings::Command, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};

struct Ctx {
    table: ResourceTable,
    wasi_ctx: WasiCtx,
    wasi_config_vars: WasiConfigVariables,
}

impl WasiView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

async fn run_wasi(path: &str, ctx: Ctx) -> Result<()> {
    let engine = test_programs_artifacts::engine(|config| {
        config.async_support(true);
    });
    let mut store = Store::new(&engine, ctx);
    let component = Component::from_file(&engine, path)?;

    let mut linker = Linker::new(&engine);
    add_to_linker_async(&mut linker)?;
    wasmtime_wasi_config::add_to_linker(&mut linker, |h: &mut Ctx| {
        WasiConfig::from(&h.wasi_config_vars)
    })?;

    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    command
        .wasi_cli_run()
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow!("command returned with failing exit status"))
}

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}

foreach_config!(assert_test_exists);

#[tokio::test(flavor = "multi_thread")]
async fn config_get() -> Result<()> {
    run_wasi(
        CONFIG_GET_COMPONENT,
        Ctx {
            table: ResourceTable::new(),
            wasi_ctx: WasiCtxBuilder::new().build(),
            wasi_config_vars: WasiConfigVariables::from_iter(vec![("hello", "world")]),
        },
    )
    .await
}
