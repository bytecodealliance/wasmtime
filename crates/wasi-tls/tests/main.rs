use anyhow::{Result, anyhow};
use wasmtime::{
    Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView, bindings::Command};
use wasmtime_wasi_tls::{LinkOptions, TlsProvider, WasiTls, WasiTlsCtx, WasiTlsCtxBuilder};

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

async fn run_test(provider: Box<dyn TlsProvider>, path: &str) -> Result<()> {
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

macro_rules! test_case {
    ($provider:ident, $name:ident) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() -> anyhow::Result<()> {
            super::$name(Box::new(wasmtime_wasi_tls::$provider::default())).await
        }
    };
}

#[cfg(feature = "rustls")]
mod rustls {
    macro_rules! rustls_test_case {
        ($name:ident) => {
            test_case!(RustlsProvider, $name);
        };
    }

    test_programs_artifacts::foreach_tls!(rustls_test_case);
}

#[cfg(feature = "nativetls")]
mod native_tls {
    macro_rules! native_tls_test_case {
        ($name:ident) => {
            test_case!(NativeTlsProvider, $name);
        };
    }

    test_programs_artifacts::foreach_tls!(native_tls_test_case);
}

async fn tls_sample_application(provider: Box<dyn TlsProvider>) -> Result<()> {
    run_test(
        provider,
        test_programs_artifacts::TLS_SAMPLE_APPLICATION_COMPONENT,
    )
    .await
}
