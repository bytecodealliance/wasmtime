use crate::common::{Profile, RunCommon, RunTarget};
use anyhow::{Result, bail};
use bytes::Bytes;
use clap::Parser;
use http::{Response, StatusCode};
use http_body_util::BodyExt as _;
use http_body_util::combinators::BoxBody;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::io::{self, AsyncWrite};
use tokio::sync::Notify;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store, StoreLimits, UpdateDeadline};
use wasmtime_wasi::p2::{StreamError, StreamResult};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::bindings as p2;
#[cfg(feature = "component-model-async")]
use wasmtime_wasi_http::handler::Task;
use wasmtime_wasi_http::handler::{HandlerState, ProxyHandler, ProxyPre};
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::{
    DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS, DEFAULT_OUTGOING_BODY_CHUNK_SIZE, WasiHttpCtx,
    WasiHttpView,
};

#[cfg(feature = "wasi-config")]
use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
#[cfg(feature = "wasi-keyvalue")]
use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};
#[cfg(feature = "wasi-nn")]
use wasmtime_wasi_nn::wit::WasiNnCtx;

struct Host {
    table: wasmtime::component::ResourceTable,
    ctx: WasiCtx,
    http: WasiHttpCtx,
    http_outgoing_body_buffer_chunks: Option<usize>,
    http_outgoing_body_chunk_size: Option<usize>,

    #[cfg(feature = "component-model-async")]
    p3_http: crate::common::DefaultP3Ctx,

    limits: StoreLimits,

    #[cfg(feature = "wasi-nn")]
    nn: Option<WasiNnCtx>,

    #[cfg(feature = "wasi-config")]
    wasi_config: Option<WasiConfigVariables>,

    #[cfg(feature = "wasi-keyvalue")]
    wasi_keyvalue: Option<WasiKeyValueCtx>,

    #[cfg(feature = "profiling")]
    guest_profiler: Option<Arc<wasmtime::GuestProfiler>>,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Host {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        self.http_outgoing_body_buffer_chunks
            .unwrap_or_else(|| DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS)
    }

    fn outgoing_body_chunk_size(&mut self) -> usize {
        self.http_outgoing_body_chunk_size
            .unwrap_or_else(|| DEFAULT_OUTGOING_BODY_CHUNK_SIZE)
    }
}

#[cfg(feature = "component-model-async")]
impl wasmtime_wasi_http::p3::WasiHttpView for Host {
    fn http(&mut self) -> wasmtime_wasi_http::p3::WasiHttpCtxView<'_> {
        wasmtime_wasi_http::p3::WasiHttpCtxView {
            table: &mut self.table,
            ctx: &mut self.p3_http,
        }
    }
}

const DEFAULT_ADDR: std::net::SocketAddr = std::net::SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
    8080,
);

/// Runs a WebAssembly module
#[derive(Parser)]
pub struct ServeCommand {
    #[command(flatten)]
    run: RunCommon,

    /// Socket address for the web server to bind to.
    #[arg(long , value_name = "SOCKADDR", default_value_t = DEFAULT_ADDR)]
    addr: SocketAddr,

    /// Socket address where, when connected to, will initiate a graceful
    /// shutdown.
    ///
    /// Note that graceful shutdown is also supported on ctrl-c.
    #[arg(long, value_name = "SOCKADDR")]
    shutdown_addr: Option<SocketAddr>,

    /// Disable log prefixes of wasi-http handlers.
    /// if unspecified, logs will be prefixed with 'stdout|stderr [{req_id}] :: '
    #[arg(long)]
    no_logging_prefix: bool,

    /// The WebAssembly component to run.
    #[arg(value_name = "WASM", required = true)]
    component: PathBuf,
}

impl ServeCommand {
    /// Start a server to run the given wasi-http proxy component
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;

        // We force cli errors before starting to listen for connections so then
        // we don't accidentally delay them to the first request.

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
            }
        }

        if self.run.common.wasi.threads == Some(true) {
            bail!("wasi-threads does not support components yet")
        }

        // The serve command requires both wasi-http and the component model, so
        // we enable those by default here.
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

        runtime.block_on(self.serve())?;

        Ok(())
    }

    fn new_store(&self, engine: &Engine, req_id: Option<u64>) -> Result<Store<Host>> {
        let mut builder = WasiCtxBuilder::new();
        self.run.configure_wasip2(&mut builder)?;

        if let Some(req_id) = req_id {
            builder.env("REQUEST_ID", req_id.to_string());
        }

        let stdout_prefix: String;
        let stderr_prefix: String;
        match req_id {
            Some(req_id) if !self.no_logging_prefix => {
                stdout_prefix = format!("stdout [{req_id}] :: ");
                stderr_prefix = format!("stderr [{req_id}] :: ");
            }
            _ => {
                stdout_prefix = "".to_string();
                stderr_prefix = "".to_string();
            }
        }
        builder.stdout(LogStream::new(stdout_prefix, Output::Stdout));
        builder.stderr(LogStream::new(stderr_prefix, Output::Stderr));

        let mut host = Host {
            table: wasmtime::component::ResourceTable::new(),
            ctx: builder.build(),
            http: WasiHttpCtx::new(),
            http_outgoing_body_buffer_chunks: self.run.common.wasi.http_outgoing_body_buffer_chunks,
            http_outgoing_body_chunk_size: self.run.common.wasi.http_outgoing_body_chunk_size,

            limits: StoreLimits::default(),

            #[cfg(feature = "wasi-nn")]
            nn: None,
            #[cfg(feature = "wasi-config")]
            wasi_config: None,
            #[cfg(feature = "wasi-keyvalue")]
            wasi_keyvalue: None,
            #[cfg(feature = "profiling")]
            guest_profiler: None,
            #[cfg(feature = "component-model-async")]
            p3_http: crate::common::DefaultP3Ctx,
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

        if self.run.common.wasi.config == Some(true) {
            #[cfg(feature = "wasi-config")]
            {
                let vars = WasiConfigVariables::from_iter(
                    self.run
                        .common
                        .wasi
                        .config_var
                        .iter()
                        .map(|v| (v.key.clone(), v.value.clone())),
                );
                host.wasi_config.replace(vars);
            }
        }

        if self.run.common.wasi.keyvalue == Some(true) {
            #[cfg(feature = "wasi-keyvalue")]
            {
                let ctx = WasiKeyValueCtxBuilder::new()
                    .in_memory_data(
                        self.run
                            .common
                            .wasi
                            .keyvalue_in_memory_data
                            .iter()
                            .map(|v| (v.key.clone(), v.value.clone())),
                    )
                    .build();
                host.wasi_keyvalue.replace(ctx);
            }
        }

        let mut store = Store::new(engine, host);

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
        self.run.validate_p3_option()?;
        let cli = self.run.validate_cli_enabled()?;

        // Repurpose the `-Scli` flag of `wasmtime run` for `wasmtime serve`
        // to serve as a signal to enable all WASI interfaces instead of just
        // those in the `proxy` world. If `-Scli` is present then add all
        // `command` APIs and then additionally add in the required HTTP APIs.
        //
        // If `-Scli` isn't passed then use the `add_to_linker_async`
        // bindings which adds just those interfaces that the proxy interface
        // uses.
        if cli == Some(true) {
            self.run.add_wasmtime_wasi_to_linker(linker)?;
            wasmtime_wasi_http::add_only_http_to_linker_async(linker)?;
            #[cfg(feature = "component-model-async")]
            if self.run.common.wasi.p3.unwrap_or(crate::common::P3_DEFAULT) {
                wasmtime_wasi_http::p3::add_to_linker(linker)?;
            }
        } else {
            wasmtime_wasi_http::add_to_linker_async(linker)?;
            #[cfg(feature = "component-model-async")]
            if self.run.common.wasi.p3.unwrap_or(crate::common::P3_DEFAULT) {
                wasmtime_wasi_http::p3::add_to_linker(linker)?;
                wasmtime_wasi::p3::clocks::add_to_linker(linker)?;
                wasmtime_wasi::p3::random::add_to_linker(linker)?;
                wasmtime_wasi::p3::cli::add_to_linker(linker)?;
            }
        }

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("support for wasi-nn was disabled at compile time");
            }
            #[cfg(feature = "wasi-nn")]
            {
                wasmtime_wasi_nn::wit::add_to_linker(linker, |h: &mut Host| {
                    let ctx = h.nn.as_mut().unwrap();
                    wasmtime_wasi_nn::wit::WasiNnView::new(&mut h.table, ctx)
                })?;
            }
        }

        if self.run.common.wasi.config == Some(true) {
            #[cfg(not(feature = "wasi-config"))]
            {
                bail!("support for wasi-config was disabled at compile time");
            }
            #[cfg(feature = "wasi-config")]
            {
                wasmtime_wasi_config::add_to_linker(linker, |h| {
                    WasiConfig::from(h.wasi_config.as_ref().unwrap())
                })?;
            }
        }

        if self.run.common.wasi.keyvalue == Some(true) {
            #[cfg(not(feature = "wasi-keyvalue"))]
            {
                bail!("support for wasi-keyvalue was disabled at compile time");
            }
            #[cfg(feature = "wasi-keyvalue")]
            {
                wasmtime_wasi_keyvalue::add_to_linker(linker, |h: &mut Host| {
                    WasiKeyValue::new(h.wasi_keyvalue.as_ref().unwrap(), &mut h.table)
                })?;
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

        let mut config = self
            .run
            .common
            .config(use_pooling_allocator_by_default().unwrap_or(None))?;
        config.wasm_component_model(true);
        config.async_support(true);

        if self.run.common.wasm.timeout.is_some() {
            config.epoch_interruption(true);
        }

        match self.run.profile {
            Some(Profile::Native(s)) => {
                config.profiler(s);
            }
            Some(Profile::Guest { .. }) => {
                config.epoch_interruption(true);
            }
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
        #[cfg(feature = "component-model-async")]
        let instance = match wasmtime_wasi_http::p3::bindings::ProxyPre::new(instance.clone()) {
            Ok(pre) => ProxyPre::P3(pre),
            Err(_) => ProxyPre::P2(p2::ProxyPre::new(instance)?),
        };
        #[cfg(not(feature = "component-model-async"))]
        let instance = ProxyPre::P2(p2::ProxyPre::new(instance)?);

        // Spawn background task(s) waiting for graceful shutdown signals. This
        // always listens for ctrl-c but additionally can listen for a TCP
        // connection to the specified address.
        let shutdown = Arc::new(GracefulShutdown::default());
        tokio::task::spawn({
            let shutdown = shutdown.clone();
            async move {
                tokio::signal::ctrl_c().await.unwrap();
                shutdown.requested.notify_one();
            }
        });
        if let Some(addr) = self.shutdown_addr {
            let listener = tokio::net::TcpListener::bind(addr).await?;
            eprintln!(
                "Listening for shutdown on tcp://{}/",
                listener.local_addr()?
            );
            let shutdown = shutdown.clone();
            tokio::task::spawn(async move {
                let _ = listener.accept().await;
                shutdown.requested.notify_one();
            });
        }

        let socket = match &self.addr {
            SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4()?,
            SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6()?,
        };
        // Conditionally enable `SO_REUSEADDR` depending on the current
        // platform. On Unix we want this to be able to rebind an address in
        // the `TIME_WAIT` state which can happen then a server is killed with
        // active TCP connections and then restarted. On Windows though if
        // `SO_REUSEADDR` is specified then it enables multiple applications to
        // bind the port at the same time which is not something we want. Hence
        // this is conditionally set based on the platform (and deviates from
        // Tokio's default from always-on).
        socket.set_reuseaddr(!cfg!(windows))?;
        socket.bind(self.addr)?;
        let listener = socket.listen(100)?;

        eprintln!("Serving HTTP on http://{}/", listener.local_addr()?);

        log::info!("Listening on {}", self.addr);

        let epoch_interval = if let Some(Profile::Guest { interval, .. }) = self.run.profile {
            Some(interval)
        } else if let Some(t) = self.run.common.wasm.timeout {
            Some(EPOCH_INTERRUPT_PERIOD.min(t))
        } else {
            None
        };
        let _epoch_thread = epoch_interval.map(|t| EpochThread::spawn(t, engine.clone()));

        let handler = ProxyHandler::new(HostHandlerState { cmd: self, engine }, instance);

        loop {
            // Wait for a socket, but also "race" against shutdown to break out
            // of this loop. Once the graceful shutdown signal is received then
            // this loop exits immediately.
            let (stream, _) = tokio::select! {
                _ = shutdown.requested.notified() => break,
                v = listener.accept() => v?,
            };
            let comp = component.clone();

            // The Nagle algorithm can impose a significant latency penalty
            // (e.g. 40ms on Linux) on guests which write small, intermittent
            // response body chunks (e.g. SSE streams).  Here we disable that
            // algorithm and rely on the guest to buffer if appropriate to avoid
            // TCP fragmentation.
            stream.set_nodelay(true)?;

            let stream = TokioIo::new(stream);
            let h = handler.clone();
            let shutdown_guard = shutdown.clone().increment();
            tokio::task::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(
                        stream,
                        hyper::service::service_fn(move |req| {
                            let comp = comp.clone();
                            let h = h.clone();
                            async move {
                                use http_body_util::{BodyExt, Full};
                                match handle_request(h, req, comp).await {
                                    Ok(r) => Ok::<_, Infallible>(r),
                                    Err(e) => {
                                        eprintln!("error: {e:?}");
                                        let error_html = "\
<!doctype html>
<html>
<head>
    <title>500 Internal Server Error</title>
</head>
<body>
    <center>
        <h1>500 Internal Server Error</h1>
        <hr>
        wasmtime
    </center>
</body>
</html>";
                                        Ok(Response::builder()
                                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                                            .header("Content-Type", "text/html; charset=UTF-8")
                                            .body(
                                                Full::new(bytes::Bytes::from(error_html))
                                                    .map_err(|_| unreachable!())
                                                    .boxed(),
                                            )
                                            .unwrap())
                                    }
                                }
                            }
                        }),
                    )
                    .await
                {
                    eprintln!("error: {e:?}");
                }
                drop(shutdown_guard);
            });
        }

        // Upon exiting the loop we'll no longer process any more incoming
        // connections but there may still be outstanding connections
        // processing in child tasks. If there are wait for those to complete
        // before shutting down completely. Also enable short-circuiting this
        // wait with a second ctrl-c signal.
        if shutdown.close() {
            return Ok(());
        }
        eprintln!("Waiting for child tasks to exit, ctrl-c again to quit sooner...");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = shutdown.complete.notified() => {}
        }

        Ok(())
    }
}

struct HostHandlerState {
    cmd: ServeCommand,
    engine: Engine,
}

impl HandlerState for HostHandlerState {
    type StoreData = Host;

    fn new_store(&self) -> Result<Store<Host>> {
        self.cmd.new_store(&self.engine, None)
    }
}

/// Helper structure to manage graceful shutdown int he accept loop above.
#[derive(Default)]
struct GracefulShutdown {
    /// Async notification that shutdown has been requested.
    requested: Notify,
    /// Async notification that shutdown has completed, signaled when
    /// `notify_when_done` is `true` and `active_tasks` reaches 0.
    complete: Notify,
    /// Internal state related to what's in progress when shutdown is requested.
    state: Mutex<GracefulShutdownState>,
}

#[derive(Default)]
struct GracefulShutdownState {
    active_tasks: u32,
    notify_when_done: bool,
}

impl GracefulShutdown {
    /// Increments the number of active tasks and returns a guard indicating
    fn increment(self: Arc<Self>) -> impl Drop {
        struct Guard(Arc<GracefulShutdown>);

        let mut state = self.state.lock().unwrap();
        assert!(!state.notify_when_done);
        state.active_tasks += 1;
        drop(state);

        return Guard(self);

        impl Drop for Guard {
            fn drop(&mut self) {
                let mut state = self.0.state.lock().unwrap();
                state.active_tasks -= 1;
                if state.notify_when_done && state.active_tasks == 0 {
                    self.0.complete.notify_one();
                }
            }
        }
    }

    /// Flags this state as done spawning tasks and returns whether there are no
    /// more child tasks remaining.
    fn close(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.notify_when_done = true;
        state.active_tasks == 0
    }
}

/// When executing with a timeout enabled, this is how frequently epoch
/// interrupts will be executed to check for timeouts. If guest profiling
/// is enabled, the guest epoch period will be used.
const EPOCH_INTERRUPT_PERIOD: Duration = Duration::from_millis(50);

struct EpochThread {
    shutdown: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EpochThread {
    fn spawn(interval: std::time::Duration, engine: Engine) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let handle = {
            let shutdown = Arc::clone(&shutdown);
            let handle = std::thread::spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    std::thread::sleep(interval);
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

type WriteProfile = Box<dyn FnOnce(&mut Store<Host>) + Send>;

fn setup_epoch_handler(
    cmd: &ServeCommand,
    store: &mut Store<Host>,
    component: Component,
) -> Result<WriteProfile> {
    // Profiling Enabled
    if let Some(Profile::Guest { interval, path }) = &cmd.run.profile {
        #[cfg(feature = "profiling")]
        return setup_guest_profiler(cmd, store, path.clone(), *interval, component.clone());
        #[cfg(not(feature = "profiling"))]
        {
            let _ = (path, interval);
            bail!("support for profiling disabled at compile time!");
        }
    }

    // Profiling disabled but there's a global request timeout
    if cmd.run.common.wasm.timeout.is_some() {
        store.epoch_deadline_async_yield_and_update(1);
    }

    Ok(Box::new(|_store| {}))
}

#[cfg(feature = "profiling")]
fn setup_guest_profiler(
    cmd: &ServeCommand,
    store: &mut Store<Host>,
    path: String,
    interval: Duration,
    component: Component,
) -> Result<WriteProfile> {
    use wasmtime::{AsContext, GuestProfiler, StoreContext, StoreContextMut};

    let module_name = "<main>";

    store.data_mut().guest_profiler = Some(Arc::new(GuestProfiler::new_component(
        module_name,
        interval,
        component,
        std::iter::empty(),
    )));

    fn sample(
        mut store: StoreContextMut<Host>,
        f: impl FnOnce(&mut GuestProfiler, StoreContext<Host>),
    ) {
        let mut profiler = store.data_mut().guest_profiler.take().unwrap();
        f(
            Arc::get_mut(&mut profiler).expect("profiling doesn't support threads yet"),
            store.as_context(),
        );
        store.data_mut().guest_profiler = Some(profiler);
    }

    // Hostcall entry/exit, etc.
    store.call_hook(|store, kind| {
        sample(store, |profiler, store| profiler.call_hook(store, kind));
        Ok(())
    });

    let start = Instant::now();
    let timeout = cmd.run.common.wasm.timeout;
    store.epoch_deadline_callback(move |store| {
        sample(store, |profiler, store| {
            profiler.sample(store, std::time::Duration::ZERO)
        });

        // Originally epoch counting was used here; this is problematic in
        // a lot of cases due to there being a lot of time (e.g. in hostcalls)
        // when we are not expected to get sample hits.
        if let Some(timeout) = timeout {
            if start.elapsed() > timeout {
                bail!("Timeout expired");
            }
        }

        Ok(UpdateDeadline::Continue(1))
    });

    store.set_epoch_deadline(1);

    let write_profile = Box::new(move |store: &mut Store<Host>| {
        let profiler = Arc::try_unwrap(store.data_mut().guest_profiler.take().unwrap())
            .expect("profiling doesn't support threads yet");
        if let Err(e) = std::fs::File::create(&path)
            .map_err(anyhow::Error::new)
            .and_then(|output| profiler.finish(std::io::BufWriter::new(output)))
        {
            eprintln!("failed writing profile at {path}: {e:#}");
        } else {
            eprintln!();
            eprintln!("Profile written to: {path}");
            eprintln!("View this profile at https://profiler.firefox.com/.");
        }
    });

    Ok(write_profile)
}

type Request = hyper::Request<hyper::body::Incoming>;

async fn handle_request(
    handler: ProxyHandler<HostHandlerState, Host>,
    req: Request,
    component: Component,
) -> Result<hyper::Response<BoxBody<Bytes, anyhow::Error>>> {
    let req_id = handler.next_req_id();

    log::info!(
        "Request {req_id} handling {} to {}",
        req.method(),
        req.uri()
    );

    match &handler.instance_pre() {
        ProxyPre::P2(pre) => {
            let mut store = handler
                .state()
                .cmd
                .new_store(&handler.state().engine, Some(req_id))?;

            let write_profile =
                setup_epoch_handler(&handler.state().cmd, &mut store, component.clone())?;
            let timeout = handler
                .state()
                .cmd
                .run
                .common
                .wasm
                .timeout
                .unwrap_or(Duration::MAX);

            let proxy = pre.instantiate_async(&mut store).await?;
            let req = store
                .data_mut()
                .new_incoming_request(p2::http::types::Scheme::Http, req)?;
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let out = store.data_mut().new_response_outparam(sender)?;
            let task = tokio::task::spawn(async move {
                let result = tokio::time::timeout(
                    timeout,
                    proxy
                        .wasi_http_incoming_handler()
                        .call_handle(&mut store, req, out),
                )
                .await
                .unwrap_or_else(|_| bail!("guest timed out"));
                if let Err(e) = result {
                    log::error!("[{req_id}] :: {e:?}");
                    return Err(e);
                }

                write_profile(&mut store);

                Ok(())
            });

            let result = match receiver.await {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => bail!(e),
                Err(_) => {
                    // An error in the receiver (`RecvError`) only indicates that the
                    // task exited before a response was sent (i.e., the sender was
                    // dropped); it does not describe the underlying cause of failure.
                    // Instead we retrieve and propagate the error from inside the task
                    // which should more clearly tell the user what went wrong. Note
                    // that we assume the task has already exited at this point so the
                    // `await` should resolve immediately.
                    let e = match task.await {
                        Ok(Ok(())) => {
                            bail!("guest never invoked `response-outparam::set` method")
                        }
                        Ok(Err(e)) => e,
                        Err(e) => e.into(),
                    };
                    bail!(e.context("guest never invoked `response-outparam::set` method"))
                }
            };

            Ok(result.map(|body| body.map_err(|e| e.into()).boxed()))
        }
        #[cfg(feature = "component-model-async")]
        ProxyPre::P3(..) => {
            use wasmtime_wasi_http::p3::bindings::http::types::{ErrorCode, Request};

            let (tx, rx) = tokio::sync::oneshot::channel();

            handler.push(Task::new(
                Box::new(move |store, proxy| {
                    Box::pin(async move {
                        let (req, body) = req.into_parts();
                        let body = body.map_err(ErrorCode::from_hyper_request_error);
                        let req = http::Request::from_parts(req, body);
                        let (request, request_io_result) = Request::from_http(req);
                        let (res, task) = proxy.handle(store, request).await??;
                        let res =
                            store.with(|mut store| res.into_http(&mut store, request_io_result))?;
                        _ = tx.send(res);

                        // Wait for the task to finish.
                        task.block(store).await;
                        anyhow::Ok(())
                    })
                }),
                req_id,
            ));
            Ok(rx.await?.map(|body| body.map_err(|err| err.into()).boxed()))
        }
    }
}

#[derive(Clone)]
enum Output {
    Stdout,
    Stderr,
}

impl Output {
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        use std::io::Write;

        match self {
            Output::Stdout => std::io::stdout().write_all(buf),
            Output::Stderr => std::io::stderr().write_all(buf),
        }
    }
}

#[derive(Clone)]
struct LogStream {
    output: Output,
    state: Arc<LogStreamState>,
}

struct LogStreamState {
    prefix: String,
    needs_prefix_on_next_write: AtomicBool,
}

impl LogStream {
    fn new(prefix: String, output: Output) -> LogStream {
        LogStream {
            output,
            state: Arc::new(LogStreamState {
                prefix,
                needs_prefix_on_next_write: AtomicBool::new(true),
            }),
        }
    }

    fn write_all(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            if self
                .state
                .needs_prefix_on_next_write
                .load(Ordering::Relaxed)
            {
                self.output.write_all(self.state.prefix.as_bytes())?;
                self.state
                    .needs_prefix_on_next_write
                    .store(false, Ordering::Relaxed);
            }
            match bytes.iter().position(|b| *b == b'\n') {
                Some(i) => {
                    let (a, b) = bytes.split_at(i + 1);
                    bytes = b;
                    self.output.write_all(a)?;
                    self.state
                        .needs_prefix_on_next_write
                        .store(true, Ordering::Relaxed);
                }
                None => {
                    self.output.write_all(bytes)?;
                    break;
                }
            }
        }

        Ok(())
    }
}

impl wasmtime_wasi::cli::StdoutStream for LogStream {
    fn p2_stream(&self) -> Box<dyn wasmtime_wasi::p2::OutputStream> {
        Box::new(self.clone())
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl wasmtime_wasi::cli::IsTerminal for LogStream {
    fn is_terminal(&self) -> bool {
        match &self.output {
            Output::Stdout => std::io::stdout().is_terminal(),
            Output::Stderr => std::io::stderr().is_terminal(),
        }
    }
}

impl wasmtime_wasi::p2::OutputStream for LogStream {
    fn write(&mut self, bytes: bytes::Bytes) -> StreamResult<()> {
        self.write_all(&bytes)
            .map_err(|e| StreamError::LastOperationFailed(e.into()))?;
        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::Pollable for LogStream {
    async fn ready(&mut self) {}
}

impl AsyncWrite for LogStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.write_all(buf).map(|_| buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// The pooling allocator is tailor made for the `wasmtime serve` use case, so
/// try to use it when we can. The main cost of the pooling allocator, however,
/// is the virtual memory required to run it. Not all systems support the same
/// amount of virtual memory, for example some aarch64 and riscv64 configuration
/// only support 39 bits of virtual address space.
///
/// The pooling allocator, by default, will request 1000 linear memories each
/// sized at 6G per linear memory. This is 6T of virtual memory which ends up
/// being about 42 bits of the address space. This exceeds the 39 bit limit of
/// some systems, so there the pooling allocator will fail by default.
///
/// This function attempts to dynamically determine the hint for the pooling
/// allocator. This returns `Some(true)` if the pooling allocator should be used
/// by default, or `None` or an error otherwise.
///
/// The method for testing this is to allocate a 0-sized 64-bit linear memory
/// with a maximum size that's N bits large where we force all memories to be
/// static. This should attempt to acquire N bits of the virtual address space.
/// If successful that should mean that the pooling allocator is OK to use, but
/// if it fails then the pooling allocator is not used and the normal mmap-based
/// implementation is used instead.
fn use_pooling_allocator_by_default() -> Result<Option<bool>> {
    use wasmtime::{Config, Memory, MemoryType};
    const BITS_TO_TEST: u32 = 42;
    let mut config = Config::new();
    config.wasm_memory64(true);
    config.memory_reservation(1 << BITS_TO_TEST);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    // NB: the maximum size is in wasm pages to take out the 16-bits of wasm
    // page size here from the maximum size.
    let ty = MemoryType::new64(0, Some(1 << (BITS_TO_TEST - 16)));
    if Memory::new(&mut store, ty).is_ok() {
        Ok(Some(true))
    } else {
        Ok(None)
    }
}
