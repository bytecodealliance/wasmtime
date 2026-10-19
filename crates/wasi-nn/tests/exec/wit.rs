use super::PREOPENED_DIR_NAME;
use crate::check::artifacts_dir;
use std::path::Path;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Result, Store, format_err};
use wasmtime_wasi::p2::bindings::sync::Command;
use wasmtime_wasi::{FsPerms, NamedId, WasiCtx, WasiCtxView};
use wasmtime_wasi_nn::backend::{BackendError, BackendGraph};
use wasmtime_wasi_nn::wit::{Interface, WasiNnNamedView, WasiNnView};
use wasmtime_wasi_nn::{
    Backend, ExecutionContext, Graph, GraphRegistry, InMemoryRegistry, wit::WasiNnCtx,
};

/// Run a wasi-nn test program. This is modeled after
/// `crates/wasi/tests/all/main.rs` but still uses the older p1 API for
/// file reads.
pub fn run(path: &str, backend: Backend, preload_model: bool) -> Result<()> {
    let path = Path::new(path);
    let engine = Engine::new(&Config::new())?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi_nn::wit::add_to_linker(&mut linker, |c: &mut Ctx| {
        WasiNnView::new(&mut c.table, &mut c.wasi_nn)
    })?;
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    let module = Component::from_file(&engine, path)?;
    let mut store = Store::new(&engine, Ctx::new(&artifacts_dir(), preload_model, backend)?);
    let command = Command::instantiate(&mut store, &module, &linker)?;
    let result = command.wasi_cli_run().call_run(&mut store)?;
    result.map_err(|_| format_err!("failed to run command"))
}

/// The host state for running wasi-nn component tests.
struct Ctx {
    wasi: WasiCtx,
    wasi_nn: WasiNnCtx,
    /// Contexts for named imports, indexed by their `NamedId`.
    named_wasi_nn: Vec<WasiNnCtx>,
    table: ResourceTable,
}

impl Ctx {
    fn new(preopen_dir: &Path, preload_model: bool, mut backend: Backend) -> Result<Self> {
        let mut builder = WasiCtx::builder();
        builder.inherit_stdio().preopened_dir(
            preopen_dir,
            PREOPENED_DIR_NAME,
            FsPerms::ReadOnly,
        )?;
        let wasi = builder.build();

        let mut registry = InMemoryRegistry::new();
        let mobilenet_dir = artifacts_dir();
        if preload_model {
            registry.load((backend).as_dir_loadable().unwrap(), &mobilenet_dir)?;
        }
        let wasi_nn = WasiNnCtx::new([backend], registry.into());

        let table = ResourceTable::new();

        Ok(Self {
            wasi,
            wasi_nn,
            named_wasi_nn: Vec::new(),
            table,
        })
    }
}

impl wasmtime_wasi::WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiNnNamedView for Ctx {
    fn nn(&mut self, id: NamedId) -> WasiNnView<'_> {
        WasiNnView::new(&mut self.table, &mut self.named_wasi_nn[id.0])
    }
}

/// Run a test program which imports `wasi:nn/graph` under the labels `a` and
/// `b`, where each label gets a registry with only a graph of the same name.
pub fn run_named_imports(path: &str) -> Result<()> {
    struct Registry(&'static str, Graph);

    impl GraphRegistry for Registry {
        fn get(&self, name: &str) -> Option<&Graph> {
            (name == self.0).then_some(&self.1)
        }
        fn get_mut(&mut self, name: &str) -> Option<&mut Graph> {
            (name == self.0).then_some(&mut self.1)
        }
    }

    struct UnusedGraph;

    impl BackendGraph for UnusedGraph {
        fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
            unreachable!()
        }
    }

    let mut config = Config::new();
    config.wasm_component_model_implements(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, path)?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    wasmtime_wasi_nn::wit::add_to_linker(&mut linker, |c: &mut Ctx| {
        WasiNnView::new(&mut c.table, &mut c.wasi_nn)
    })?;
    wasmtime_wasi_nn::wit::add_named_to_linker(&mut linker, &component, |i, name| {
        assert_eq!(i, Interface::NnGraph);
        Ok(match name {
            "a" => NamedId(0),
            "b" => NamedId(1),
            _ => panic!("unexpected name: {name}"),
        })
    })?;

    let named_ctx = |name| {
        let graph = Graph::from(Box::new(UnusedGraph) as Box<dyn BackendGraph>);
        WasiNnCtx::new([], Registry(name, graph).into())
    };
    let ctx = Ctx {
        wasi: WasiCtx::builder().inherit_stdio().build(),
        wasi_nn: WasiNnCtx::new([], InMemoryRegistry::new().into()),
        named_wasi_nn: vec![named_ctx("a"), named_ctx("b")],
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, ctx);
    let command = Command::instantiate(&mut store, &component, &linker)?;
    let result = command.wasi_cli_run().call_run(&mut store)?;
    result.map_err(|_| format_err!("failed to run command"))
}
