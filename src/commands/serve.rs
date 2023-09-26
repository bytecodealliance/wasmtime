use anyhow::Result;
use clap::Parser;
use std::{path::PathBuf, pin::Pin, sync::Arc};
use wasmtime::component::{Component, InstancePre, Linker};
use wasmtime::{Engine, Store};
use wasmtime_cli_flags::CommonOptions;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{body::HyperOutgoingBody, WasiHttpCtx, WasiHttpView};

struct Host {
    table: Table,
    ctx: WasiCtx,
    http: WasiHttpCtx,
}

impl Host {
    fn new() -> Result<Self> {
        let mut table = Table::new();
        let ctx = WasiCtxBuilder::new().build(&mut table)?;
        Ok(Host {
            table,
            ctx,
            http: WasiHttpCtx,
        })
    }
}

impl WasiView for Host {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for Host {
    fn table(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

/// Runs a WebAssembly module
#[derive(Parser)]
#[structopt(name = "run")]
pub struct ServeCommand {
    #[clap(flatten)]
    common: CommonOptions,

    /// The WebAssembly component to run.
    #[clap(value_name = "WASM", required = true)]
    component: PathBuf,
}

impl ServeCommand {
    /// Start a server to run the given wasi-http proxy component
    pub fn execute(mut self) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()?;

        runtime.block_on(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    Ok::<_, anyhow::Error>(())
                }

                res = self.serve() => {
                    res
                }
            }
        })?;

        Ok(())
    }

    fn add_to_linker(&self, linker: &mut Linker<Host>) -> Result<()> {
        wasmtime_wasi::preview2::bindings::filesystem::types::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::filesystem::preopens::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::environment::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::exit::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::terminal_input::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::terminal_output::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::terminal_stdin::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::terminal_stdout::add_to_linker(linker, |a| a)?;
        wasmtime_wasi::preview2::bindings::cli::terminal_stderr::add_to_linker(linker, |a| a)?;
        wasmtime_wasi_http::proxy::add_to_linker(linker)?;
        Ok(())
    }

    async fn serve(&mut self) -> Result<()> {
        use hyper::server::conn::http1;

        let mut config = self.common.config(None)?;
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Arc::new(Engine::new(&config)?);
        let mut linker = Linker::new(&engine);

        self.add_to_linker(&mut linker)?;

        let component = Component::from_file(&engine, &self.component)?;

        let instance = Arc::new(linker.instantiate_pre(&component)?);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let engine = Arc::clone(&engine);
            let instance = Arc::clone(&instance);
            tokio::task::spawn(async move {
                let handler = ProxyHandler::new(engine, instance);
                if let Err(e) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(stream, handler)
                    .await
                {
                    eprintln!("error: {e:?}");
                }

                Ok::<_, anyhow::Error>(())
            });
        }
    }
}

#[derive(Clone)]
struct ProxyHandler {
    engine: Arc<Engine>,
    instance_pre: Arc<InstancePre<Host>>,
}

impl ProxyHandler {
    fn new(engine: Arc<Engine>, instance_pre: Arc<InstancePre<Host>>) -> Self {
        Self {
            engine,
            instance_pre,
        }
    }
}

type Request = hyper::Request<hyper::body::Incoming>;

impl hyper::service::Service<Request> for ProxyHandler {
    type Response = hyper::Response<HyperOutgoingBody>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response>> + Send>>;

    fn call(&mut self, req: Request) -> Self::Future {
        use http_body_util::BodyExt;

        let handler = self.clone();

        Box::pin(async move {
            let host = Host::new()?;
            let mut store = Store::new(&handler.engine, host);

            let req = store.data_mut().new_incoming_request(
                req.map(|body| body.map_err(|e| anyhow::anyhow!(e)).boxed()),
            )?;

            let (sender, receiver) = tokio::sync::oneshot::channel();
            let out = store.data_mut().new_response_outparam(sender)?;

            let (proxy, _inst) = wasmtime_wasi_http::proxy::Proxy::instantiate_pre(
                &mut store,
                &handler.instance_pre,
            )
            .await?;

            // TODO: need to track the join handle, but don't want to block the response on it
            tokio::task::spawn(async move {
                proxy
                    .wasi_http_incoming_handler()
                    .call_handle(store, req, out)
                    .await?;

                Ok::<_, anyhow::Error>(())
            });

            let resp = receiver.await.unwrap()?;
            Ok(resp)
        })
    }
}
