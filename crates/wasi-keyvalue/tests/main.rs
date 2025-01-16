use anyhow::{anyhow, Result};
use test_programs_artifacts::{foreach_keyvalue, KEYVALUE_MAIN_COMPONENT};
use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Store,
};
use wasmtime_wasi::{bindings::Command, IoView, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};

struct Ctx {
    table: ResourceTable,
    wasi_ctx: WasiCtx,
    wasi_keyvalue_ctx: WasiKeyValueCtx,
}

impl IoView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
impl WasiView for Ctx {
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
    wasmtime_wasi::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_keyvalue::add_to_linker(&mut linker, |h: &mut Ctx| {
        WasiKeyValue::new(&h.wasi_keyvalue_ctx, &mut h.table)
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
        #[expect(unused_imports, reason = "just here to assert it exists")]
        use self::$name as _;
    };
}

foreach_keyvalue!(assert_test_exists);

#[tokio::test(flavor = "multi_thread")]
async fn keyvalue_main() -> Result<()> {
    run_wasi(
        KEYVALUE_MAIN_COMPONENT,
        Ctx {
            table: ResourceTable::new(),
            wasi_ctx: WasiCtxBuilder::new().inherit_stderr().build(),
            wasi_keyvalue_ctx: WasiKeyValueCtxBuilder::new()
                .in_memory_data([("atomics_key", "5")])
                .build(),
        },
    )
    .await
}
