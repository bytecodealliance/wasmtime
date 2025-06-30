use anyhow::{Result, anyhow};
use wasmtime::{
    Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView, bindings::Command};
use wasmtime_wasi_tls::{LinkOptions, WasiTls, WasiTlsCtx, WasiTlsCtxBuilder};

struct Ctx {
    table: ResourceTable,
    wasi_ctx: WasiCtx,
    wasi_tls_ctx: WasiTlsCtx,
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

async fn run_test(path: &str) -> Result<()> {
    let provider = Box::new(wasmtime_wasi_tls_nativetls::NativeTlsProvider::default());
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi_ctx: WasiCtxBuilder::new()
            .inherit_stderr()
            .inherit_network()
            .allow_ip_name_lookup(true)
            .build(),
        wasi_tls_ctx: WasiTlsCtxBuilder::new().provider(provider).build(),
    };

    let engine = test_programs_artifacts::engine(|config| {
        config.async_support(true);
    });
    let mut store = Store::new(&engine, ctx);
    let component = Component::from_file(&engine, path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    let mut opts = LinkOptions::default();
    opts.tls(true);
    wasmtime_wasi_tls::add_to_linker(&mut linker, &mut opts, |h: &mut Ctx| {
        WasiTls::new(&h.wasi_tls_ctx, &mut h.table)
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

test_programs_artifacts::foreach_tls!(assert_test_exists);

#[tokio::test(flavor = "multi_thread")]
async fn tls_sample_application() -> Result<()> {
    run_test(test_programs_artifacts::TLS_SAMPLE_APPLICATION_COMPONENT).await
}
