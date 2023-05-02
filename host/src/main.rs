use anyhow::{Context, Result};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{wasi, Table, WasiCtx, WasiView};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi_sockets::{WasiSocketsCtx, WasiSocketsView};
use wasmtime_wasi_sockets_sync::WasiSocketsCtxBuilder;

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

    #[arg(
        long = "mapdir",
        number_of_values = 1,
        value_name = "GUEST_DIR::HOST_DIR",
        value_parser = parse_map_dir
    )]
    map_dirs: Vec<(String, String)>,
}

fn parse_map_dir(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "failed parsing map dir: must contain exactly one double colon `::`, got {s:?}"
        )
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let input = args.component;

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &input)?;

    let mut argv: Vec<&str> = vec!["wasm"];
    argv.extend(args.args.iter().map(String::as_str));

    let mut builder = WasiCtxBuilder::new().inherit_stdio().args(&argv);

    for (guest, host) in args.map_dirs {
        let dir = cap_std::fs::Dir::open_ambient_dir(&host, cap_std::ambient_authority())
            .context(format!("opening directory {host:?}"))?;
        builder = builder.preopened_dir(dir, &guest);
    }

    let mut table = Table::new();
    let wasi = builder.build(&mut table)?;

    if args.world == "command" {
        struct CommandCtx {
            table: Table,
            wasi: WasiCtx,
            sockets: WasiSocketsCtx,
        }
        impl WasiView for CommandCtx {
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
        let sockets = WasiSocketsCtxBuilder::new()
            .inherit_network(cap_std::ambient_authority())
            .build();
        impl WasiSocketsView for CommandCtx {
            fn table(&self) -> &Table {
                &self.table
            }
            fn table_mut(&mut self) -> &mut Table {
                &mut self.table
            }
            fn ctx(&self) -> &WasiSocketsCtx {
                &self.sockets
            }
            fn ctx_mut(&mut self) -> &mut WasiSocketsCtx {
                &mut self.sockets
            }
        }

        let mut linker = Linker::new(&engine);
        wasi::command::add_to_linker(&mut linker)?;
        wasmtime_wasi_sockets::add_to_linker(&mut linker)?;
        let mut store = Store::new(
            &engine,
            CommandCtx {
                table,
                wasi,
                sockets,
            },
        );

        let (wasi, _instance) =
            wasi::command::Command::instantiate_async(&mut store, &component, &linker).await?;

        let result: Result<(), ()> = wasi.call_main(&mut store).await?;

        if result.is_err() {
            anyhow::bail!("command returned with failing exit status");
        }

        Ok(())
    } else if args.world == "proxy" {
        struct ProxyCtx {
            table: Table,
            wasi: WasiCtx,
        }
        impl WasiView for ProxyCtx {
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

        let mut linker = Linker::new(&engine);
        wasi::proxy::add_to_linker(&mut linker)?;

        let mut store = Store::new(&engine, ProxyCtx { table, wasi });

        let (wasi, _instance) =
            wasi::proxy::Proxy::instantiate_async(&mut store, &component, &linker).await?;

        // TODO: do something
        let _ = wasi;
        let result: Result<(), ()> = Ok(());

        if result.is_err() {
            anyhow::bail!("proxy returned with failing exit status");
        }

        Ok(())
    } else {
        anyhow::bail!("no such world {}", args.world)
    }
}
