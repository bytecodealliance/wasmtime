use anyhow::Result;
use host::{command, command::wasi::Command, proxy, proxy::wasi::Proxy, WasiCtx};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};

use clap::Parser;

/// Simple program to run components with host WASI support.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filesystem path of a component
    component: String,

    /// Command-line arguments
    args: Vec<String>,

    /// Name of the world to load it in.
    #[arg(long, default_value_t = String::from("command"))]
    world: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let input = args.component;

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &input)?;
    let mut linker = Linker::new(&engine);

    if args.world == "command" {
        run_command(&mut linker, &engine, &component, &args.args).await?;
    } else if args.world == "proxy" {
        run_proxy(&mut linker, &engine, &component, &args.args).await?;
    }

    Ok(())
}

async fn run_command(
    linker: &mut Linker<WasiCtx>,
    engine: &Engine,
    component: &Component,
    args: &[String],
) -> anyhow::Result<()> {
    command::add_to_linker(linker, |x| x)?;

    let mut argv: Vec<&str> = vec!["wasm"];
    argv.extend(args.iter().map(String::as_str));

    let mut store = Store::new(
        engine,
        WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_network()
            .args(&argv)
            .build(),
    );

    let (wasi, _instance) = Command::instantiate_async(&mut store, component, linker).await?;

    let result: Result<(), ()> = wasi.call_main(&mut store).await?;

    if result.is_err() {
        anyhow::bail!("command returned with failing exit status");
    }

    Ok(())
}

async fn run_proxy(
    linker: &mut Linker<WasiCtx>,
    engine: &Engine,
    component: &Component,
    args: &[String],
) -> anyhow::Result<()> {
    proxy::add_to_linker(linker, |x| x)?;

    let mut argv: Vec<&str> = vec!["wasm"];
    argv.extend(args.iter().map(String::as_str));

    let mut store = Store::new(
        engine,
        WasiCtxBuilder::new().inherit_stdio().args(&argv).build(),
    );

    let (wasi, _instance) = Proxy::instantiate_async(&mut store, component, linker).await?;

    // TODO: do something
    let _ = wasi;
    let result: Result<(), ()> = Ok(());

    if result.is_err() {
        anyhow::bail!("command returned with failing exit status");
    }

    Ok(())
}
