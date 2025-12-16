#![cfg(feature = "p3")]

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Result, Store, format_err};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_tls::p3::{DefaultWasiTlsCtx, WasiTlsCtxView, WasiTlsView};

struct Ctx {
    table: ResourceTable,
    wasi_ctx: WasiCtx,
    wasi_tls_ctx: DefaultWasiTlsCtx,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiTlsView for Ctx {
    fn tls(&mut self) -> WasiTlsCtxView<'_> {
        WasiTlsCtxView {
            ctx: &mut self.wasi_tls_ctx,
            table: &mut self.table,
        }
    }
}

async fn run_test(path: &str) -> Result<()> {
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi_ctx: WasiCtx::builder()
            .inherit_stdout()
            .inherit_stderr()
            .inherit_network()
            .allow_ip_name_lookup(true)
            .build(),
        wasi_tls_ctx: DefaultWasiTlsCtx,
    };

    let engine = test_programs_artifacts::engine(|config| {
        config.async_support(true);
        config.wasm_component_model_async(true);
    });
    let mut store = Store::new(&engine, ctx);

    let mut linker = Linker::new(&engine);
    // TODO: Remove once test components are not built for `wasm32-wasip1`
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .context("failed to link `wasi:cli@0.2.x`")?;
    wasmtime_wasi::p3::add_to_linker(&mut linker).context("failed to link `wasi:cli@0.3.x`")?;
    wasmtime_wasi_tls::p3::add_to_linker(&mut linker)?;

    let component = Component::from_file(&engine, path)?;
    let command = Command::instantiate_async(&mut store, &component, &linker)
        .await
        .context("failed to instantiate `wasi:cli/command`")?;
    store
        .run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await)
        .await
        .context("failed to call `wasi:cli/run#run`")?
        .context("guest trapped")?
        .map_err(|()| format_err!("`wasi:cli/run#run` failed"))
}

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "just here to assert it exists")]
        use self::$name as _;
    };
}

test_programs_artifacts::foreach_p3_tls!(assert_test_exists);

#[tokio::test(flavor = "multi_thread")]
async fn p3_tls_sample_application() -> Result<()> {
    run_test(test_programs_artifacts::P3_TLS_SAMPLE_APPLICATION_COMPONENT).await
}
