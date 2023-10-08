#![cfg(all(feature = "test_programs", not(skip_wasi_sockets_tests)))]
use cap_std::ambient_authority;
use wasmtime::component::Linker;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{self, command::Command, Table, WasiCtx, WasiCtxBuilder, WasiView};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_component(&str) -> Component
include!(concat!(
    env!("OUT_DIR"),
    "/wasi_sockets_tests_components.rs"
));

struct SocketsCtx {
    table: Table,
    wasi: WasiCtx,
}

impl WasiView for SocketsCtx {
    fn table(&self) -> &Table {
        &self.table
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

async fn run(name: &str) -> anyhow::Result<()> {
    let component = get_component(name);
    let mut linker = Linker::new(&ENGINE);

    preview2::command::add_to_linker(&mut linker)?;

    // Create our wasi context.
    let table = Table::new();
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_network(ambient_authority())
        .allow_ip_name_lookup(true)
        .arg(name)
        .build();

    let mut store = Store::new(&ENGINE, SocketsCtx { table, wasi });

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    command
        .wasi_cli_run()
        .call_run(&mut store)
        .await?
        .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn tcp_sample_application() {
    run("tcp_sample_application").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn tcp_bind() {
    run("tcp_bind").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn tcp_connect() {
    run("tcp_connect").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn tcp_states() {
    run("tcp_states").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn tcp_sockopts() {
    run("tcp_sockopts").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn udp_sample_application() {
    run("udp_sample_application").await.unwrap();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn ip_name_lookup() {
    run("ip_name_lookup").await.unwrap();
}
