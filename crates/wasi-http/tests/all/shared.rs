use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Result, Store};
use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{
    WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpNamedView, WasiHttpView,
};

pub struct NamedImportsData {
    wasi: WasiCtx,
    http: WasiHttpCtx,
    table: ResourceTable,
    a: NamedImportsHooks,
    b: NamedImportsHooks,
}

struct NamedImportsHooks(&'static str);

impl WasiView for NamedImportsData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for NamedImportsData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: Default::default(),
        }
    }
}

impl WasiHttpNamedView for NamedImportsData {
    fn http(&mut self, id: NamedId) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: match id.0 {
                0 => &mut self.a,
                1 => &mut self.b,
                _ => panic!("unexpected id: {}", id.0),
            },
        }
    }
}

impl WasiHttpHooks for NamedImportsHooks {
    fn is_forbidden_header(&mut self, name: &http::HeaderName) -> bool {
        name == self.0
    }
}

pub async fn run_named_imports_test<T>(
    component: &str,
    add_only_http_to_linker: fn(&mut Linker<NamedImportsData>) -> Result<()>,
    add_named_to_linker: fn(
        &mut Linker<NamedImportsData>,
        &Component,
        fn(T, &str) -> Result<NamedId>,
    ) -> Result<()>,
    run: impl AsyncFnOnce(
        &mut Store<NamedImportsData>,
        &Component,
        &Linker<NamedImportsData>,
    ) -> Result<()>,
) -> Result<()> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_component_model_implements(true);
    });
    let component = Component::from_file(&engine, component)?;
    let mut store = Store::new(
        &engine,
        NamedImportsData {
            wasi: WasiCtx::builder().inherit_stdio().build(),
            table: ResourceTable::new(),
            http: Default::default(),
            a: NamedImportsHooks("a"),
            b: NamedImportsHooks("b"),
        },
    );
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    add_only_http_to_linker(&mut linker)?;
    add_named_to_linker(&mut linker, &component, |_, name| {
        Ok(match name {
            "a" => NamedId(0),
            "b" => NamedId(1),
            _ => panic!("unexpected name: {}", name),
        })
    })?;
    run(&mut store, &component, &linker).await
}
