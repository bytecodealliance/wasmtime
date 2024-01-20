use crate::common::{Profile, RunCommon, RunTarget};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use std::{
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use wasmtime::component::{InstancePre, Linker};
use wasmtime::{Engine, Store, StoreLimits};
use wasmtime_wasi::preview2::{self, StreamError, StreamResult, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::{
    bindings::http::types as http_types, body::HyperOutgoingBody, hyper_response_error,
    WasiHttpCtx, WasiHttpView,
};

#[cfg(feature = "wasi-nn")]
use wasmtime_wasi_nn::WasiNnCtx;

struct Host {
    table: wasmtime::component::ResourceTable,
    ctx: WasiCtx,
    http: WasiHttpCtx,

    limits: StoreLimits,

    #[cfg(feature = "wasi-nn")]
    nn: Option<WasiNnCtx>,
}

impl WasiView for Host {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for Host {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

const DEFAULT_ADDR: std::net::SocketAddr = std::net::SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
    8080,
);

/// Runs a WebAssembly module
#[derive(Parser, PartialEq)]
pub struct ServeCommand {
    #[command(flatten)]
    run: RunCommon,

    /// Socket address for the web server to bind to.
    #[arg(long = "addr", value_name = "SOCKADDR", default_value_t = DEFAULT_ADDR )]
    addr: std::net::SocketAddr,

    /// The WebAssembly component to run.
    #[arg(value_name = "WASM", required = true)]
    component: PathBuf,
}

impl ServeCommand {
    /// Start a server to run the given wasi-http proxy component
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;

        // We force cli errors before starting to listen for connections so tha we don't
        // accidentally delay them to the first request.
        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
            }
        }

        if let Some(Profile::Guest { .. }) = &self.run.profile {
            bail!("Cannot use the guest profiler with components");
        }

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
            }
        }

        if self.run.common.wasi.threads == Some(true) {
            bail!("wasi-threads does not support components yet")
        }

        // The serve command requires both wasi-http and the component model, so we enable those by
        // default here.
        if self.run.common.wasi.http.replace(true) == Some(false) {
            bail!("wasi-http is required for the serve command, and must not be disabled");
        }
        if self.run.common.wasm.component_model.replace(true) == Some(false) {
            bail!("components are required for the serve command, and must not be disabled");
        }

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

    fn new_store(&self, engine: &Engine, req_id: u64) -> Result<Store<Host>> {
        let mut builder = WasiCtxBuilder::new();

        builder.envs(&[("REQUEST_ID", req_id.to_string())]);

        builder.stdout(LogStream {
            prefix: format!("stdout [{req_id}] :: "),
            output: Output::Stdout,
        });

        builder.stderr(LogStream {
            prefix: format!("stderr [{req_id}] :: "),
            output: Output::Stderr,
        });

        let mut host = Host {
            table: wasmtime::component::ResourceTable::new(),
            ctx: builder.build(),
            http: WasiHttpCtx,

            limits: StoreLimits::default(),

            #[cfg(feature = "wasi-nn")]
            nn: None,
        };

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(feature = "wasi-nn")]
            {
                let graphs = self
                    .run
                    .common
                    .wasi
                    .nn_graph
                    .iter()
                    .map(|g| (g.format.clone(), g.dir.clone()))
                    .collect::<Vec<_>>();
                let (backends, registry) = wasmtime_wasi_nn::preload(&graphs)?;
                host.nn.replace(WasiNnCtx::new(backends, registry));
            }
        }

        let mut store = Store::new(engine, host);

        if self.run.common.wasm.timeout.is_some() {
            store.set_epoch_deadline(1);
        }

        store.data_mut().limits = self.run.store_limits();
        store.limiter(|t| &mut t.limits);

        // If fuel has been configured, we want to add the configured
        // fuel amount to this store.
        if let Some(fuel) = self.run.common.wasm.fuel {
            store.set_fuel(fuel)?;
        }

        Ok(store)
    }

    fn add_to_linker(&self, linker: &mut Linker<Host>) -> Result<()> {
        // Repurpose the `-Scommon` flag of `wasmtime run` for `wasmtime serve`
        // to serve as a signal to enable all WASI interfaces instead of just
        // those in the `proxy` world. If `-Scommon` is present then add all
        // `command` APIs which includes all "common" or base WASI APIs and then
        // additionally add in the required HTTP APIs.
        //
        // If `-Scommon` isn't passed then use the `proxy::add_to_linker`
        // bindings which adds just those interfaces that the proxy interface
        // uses.
        if self.run.common.wasi.common == Some(true) {
            preview2::command::add_to_linker(linker)?;
            wasmtime_wasi_http::proxy::add_only_http_to_linker(linker)?;
        } else {
            wasmtime_wasi_http::proxy::add_to_linker(linker)?;
        }

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("support for wasi-nn was disabled at compile time");
            }
            #[cfg(feature = "wasi-nn")]
            {
                wasmtime_wasi_nn::wit::ML::add_to_linker(linker, |host| host.nn.as_mut().unwrap())?;
            }
        }

        if self.run.common.wasi.threads == Some(true) {
            bail!("support for wasi-threads is not available with components");
        }

        if self.run.common.wasi.http == Some(false) {
            bail!("support for wasi-http must be enabled for `serve` subcommand");
        }

        Ok(())
    }

    async fn serve(mut self) -> Result<()> {
        use hyper::server::conn::http1;

        let mut config = self.run.common.config(None)?;
        config.wasm_component_model(true);
        config.async_support(true);

        if self.run.common.wasm.timeout.is_some() {
            config.epoch_interruption(true);
        }

        match self.run.profile {
            Some(Profile::Native(s)) => {
                config.profiler(s);
            }

            // We bail early in `execute` if the guest profiler is configured.
            Some(Profile::Guest { .. }) => unreachable!(),

            None => {}
        }

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        self.add_to_linker(&mut linker)?;

        let component = match self.run.load_module(&engine, &self.component)? {
            RunTarget::Core(_) => bail!("The serve command currently requires a component"),
            RunTarget::Component(c) => c,
        };

        let instance = linker.instantiate_pre(&component)?;

        let listener = tokio::net::TcpListener::bind(self.addr).await?;

        eprintln!("Serving HTTP on http://{}/", listener.local_addr()?);

        let _epoch_thread = if let Some(timeout) = self.run.common.wasm.timeout {
            Some(EpochThread::spawn(timeout, engine.clone()))
        } else {
            None
        };

        log::info!("Listening on {}", self.addr);

        let handler = ProxyHandler::new(self, engine, instance);

        loop {
            let (stream, _) = listener.accept().await?;
            let stream = TokioIo::new(stream);
            let h = handler.clone();
            tokio::task::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(stream, h)
                    .await
                {
                    eprintln!("error: {e:?}");
                }
            });
        }
    }
}

struct EpochThread {
    shutdown: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EpochThread {
    fn spawn(timeout: std::time::Duration, engine: Engine) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let handle = {
            let shutdown = Arc::clone(&shutdown);
            let handle = std::thread::spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    std::thread::sleep(timeout);
                    engine.increment_epoch();
                }
            });
            Some(handle)
        };

        EpochThread { shutdown, handle }
    }
}

impl Drop for EpochThread {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.shutdown.store(true, Ordering::Relaxed);
            handle.join().unwrap();
        }
    }
}

struct ProxyHandlerInner {
    cmd: ServeCommand,
    engine: Engine,
    instance_pre: InstancePre<Host>,
    next_id: AtomicU64,
}

impl ProxyHandlerInner {
    fn next_req_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Clone)]
struct ProxyHandler(Arc<ProxyHandlerInner>);

impl ProxyHandler {
    fn new(cmd: ServeCommand, engine: Engine, instance_pre: InstancePre<Host>) -> Self {
        Self(Arc::new(ProxyHandlerInner {
            cmd,
            engine,
            instance_pre,
            next_id: AtomicU64::from(0),
        }))
    }
}

type Request = hyper::Request<hyper::body::Incoming>;

impl hyper::service::Service<Request> for ProxyHandler {
    type Response = hyper::Response<HyperOutgoingBody>;
    type Error = anyhow::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response>> + Send>>;

    fn call(&self, req: Request) -> Self::Future {
        use http_body_util::BodyExt;

        let ProxyHandler(inner) = self.clone();

        let (sender, receiver) = tokio::sync::oneshot::channel();

        // TODO: need to track the join handle, but don't want to block the response on it
        tokio::task::spawn(async move {
            let req_id = inner.next_req_id();
            let (mut parts, body) = req.into_parts();

            parts.uri = {
                let uri_parts = parts.uri.into_parts();

                let scheme = uri_parts.scheme.unwrap_or(http::uri::Scheme::HTTP);

                let host = if let Some(val) = parts.headers.get(hyper::header::HOST) {
                    std::str::from_utf8(val.as_bytes())
                        .map_err(|_| http_types::ErrorCode::HttpRequestUriInvalid)?
                } else {
                    uri_parts
                        .authority
                        .as_ref()
                        .ok_or(http_types::ErrorCode::HttpRequestUriInvalid)?
                        .host()
                };

                let path_with_query = uri_parts
                    .path_and_query
                    .ok_or(http_types::ErrorCode::HttpRequestUriInvalid)?;

                hyper::Uri::builder()
                    .scheme(scheme)
                    .authority(host)
                    .path_and_query(path_with_query)
                    .build()
                    .map_err(|_| http_types::ErrorCode::HttpRequestUriInvalid)?
            };

            let req = hyper::Request::from_parts(parts, body.map_err(hyper_response_error).boxed());

            log::info!(
                "Request {req_id} handling {} to {}",
                req.method(),
                req.uri()
            );

            let mut store = inner.cmd.new_store(&inner.engine, req_id)?;

            let req = store.data_mut().new_incoming_request(req)?;
            let out = store.data_mut().new_response_outparam(sender)?;

            let (proxy, _inst) =
                wasmtime_wasi_http::proxy::Proxy::instantiate_pre(&mut store, &inner.instance_pre)
                    .await?;

            if let Err(e) = proxy
                .wasi_http_incoming_handler()
                .call_handle(store, req, out)
                .await
            {
                log::error!("[{req_id}] :: {:#?}", e);
                return Err(e);
            }

            Ok(())
        });

        Box::pin(async move {
            match receiver.await {
                Ok(Ok(resp)) => Ok(resp),
                Ok(Err(e)) => Err(e.into()),
                Err(_) => bail!("guest never invoked `response-outparam::set` method"),
            }
        })
    }
}

#[derive(Clone)]
enum Output {
    Stdout,
    Stderr,
}

impl Output {
    fn write_all(&self, buf: &[u8]) -> anyhow::Result<()> {
        use std::io::Write;

        match self {
            Output::Stdout => std::io::stdout().write_all(buf),
            Output::Stderr => std::io::stderr().write_all(buf),
        }
        .map_err(|e| anyhow!(e))
    }
}

#[derive(Clone)]
struct LogStream {
    prefix: String,
    output: Output,
}

impl preview2::StdoutStream for LogStream {
    fn stream(&self) -> Box<dyn preview2::HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        use std::io::IsTerminal;

        match &self.output {
            Output::Stdout => std::io::stdout().is_terminal(),
            Output::Stderr => std::io::stderr().is_terminal(),
        }
    }
}

impl preview2::HostOutputStream for LogStream {
    fn write(&mut self, bytes: bytes::Bytes) -> StreamResult<()> {
        let mut msg = Vec::new();

        for line in bytes.split(|c| *c == b'\n') {
            if !line.is_empty() {
                msg.extend_from_slice(&self.prefix.as_bytes());
                msg.extend_from_slice(line);
                msg.push(b'\n');
            }
        }

        self.output
            .write_all(&msg)
            .map_err(StreamError::LastOperationFailed)
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

#[async_trait::async_trait]
impl preview2::Subscribe for LogStream {
    async fn ready(&mut self) {}
}
