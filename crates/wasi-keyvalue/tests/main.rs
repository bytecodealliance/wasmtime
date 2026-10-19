use test_programs_artifacts::{
    KEYVALUE_MAIN_COMPONENT, KEYVALUE_NAMED_IMPORTS_COMPONENT, foreach_keyvalue,
};
use wasmtime::{
    Result, Store,
    component::{Component, Linker, ResourceTable},
    format_err,
};
use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiView, p2::bindings::Command};
use wasmtime_wasi_keyvalue::{
    Interface, WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder, WasiKeyValueNamedView,
};

struct Ctx {
    table: ResourceTable,
    wasi_ctx: WasiCtx,
    wasi_keyvalue_ctx: WasiKeyValueCtx,
    /// Contexts for imports labelled `a` and `b`, in that order.
    named_keyvalue_ctxs: Vec<WasiKeyValueCtx>,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiKeyValueNamedView for Ctx {
    fn keyvalue(&mut self, id: NamedId) -> WasiKeyValue<'_> {
        WasiKeyValue::new(&self.named_keyvalue_ctxs[id.0], &mut self.table)
    }
}

async fn run_wasi(path: &str, ctx: Ctx) -> Result<()> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_component_model_implements(true);
    });
    let mut store = Store::new(&engine, ctx);
    let component = Component::from_file(&engine, path)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_keyvalue::add_to_linker(&mut linker, |h: &mut Ctx| {
        WasiKeyValue::new(&h.wasi_keyvalue_ctx, &mut h.table)
    })?;
    wasmtime_wasi_keyvalue::add_named_to_linker(&mut linker, &component, |i, name| {
        assert_eq!(i, Interface::KeyvalueStore);
        Ok(match name {
            "a" => NamedId(0),
            "b" => NamedId(1),
            _ => panic!("unexpected name: {name}"),
        })
    })?;

    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    command
        .wasi_cli_run()
        .call_run(&mut store)
        .await?
        .map_err(|()| format_err!("command returned with failing exit status"))
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
            wasi_ctx: WasiCtx::builder().inherit_stderr().build(),
            wasi_keyvalue_ctx: WasiKeyValueCtxBuilder::new()
                .in_memory_data([("atomics_key", "5")])
                .build(),
            named_keyvalue_ctxs: Vec::new(),
        },
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn keyvalue_named_imports() -> Result<()> {
    let ctx = |name| {
        WasiKeyValueCtxBuilder::new()
            .in_memory_data([("name", name)])
            .build()
    };
    run_wasi(
        KEYVALUE_NAMED_IMPORTS_COMPONENT,
        Ctx {
            table: ResourceTable::new(),
            wasi_ctx: WasiCtx::builder().inherit_stderr().build(),
            wasi_keyvalue_ctx: ctx("default"),
            named_keyvalue_ctxs: vec![ctx("a"), ctx("b")],
        },
    )
    .await
}
