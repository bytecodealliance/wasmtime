use crate::cli::WasiCliCtx;
use crate::clocks::{HostMonotonicClock, HostWallClock, WasiClocksCtx};
use crate::random::WasiRandomCtx;
use crate::sockets::{SocketAddrCheck, SocketAddrUse, WasiSocketsCtx};
use cap_rand::RngCore;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

/// Builder-style structure used to create a WASI context.
///
/// This type is used to create a WASI context that is considered per-[`Store`]
/// state.
/// This is a low-level abstraction, users of this crate are expected to use it via
/// builders specific to WASI version used, for example,
/// [p2::WasiCtxBuilder](crate::p2::WasiCtxBuilder)
///
/// [`Store`]: wasmtime::Store
#[derive(Default)]
pub(crate) struct WasiCtxBuilder<I, O> {
    pub(crate) cli: WasiCliCtx<I, O>,
    pub(crate) clocks: WasiClocksCtx,
    pub(crate) random: WasiRandomCtx,
    pub(crate) sockets: WasiSocketsCtx,
    pub(crate) allow_blocking_current_thread: bool,
}

impl<I, O> WasiCtxBuilder<I, O> {
    /// Creates a builder for a new context with default parameters set.
    ///
    /// The current defaults are:
    ///
    /// * stdin is closed
    /// * stdout and stderr eat all input and it doesn't go anywhere
    /// * no env vars
    /// * no arguments
    /// * no preopens
    /// * clocks use the host implementation of wall/monotonic clocks
    /// * RNGs are all initialized with random state and suitable generator
    ///   quality to satisfy the requirements of WASI APIs.
    /// * TCP/UDP are allowed but all addresses are denied by default.
    /// * IP name lookup is denied by default.
    ///
    /// These defaults can all be updated via the various builder configuration
    /// methods below.
    pub(crate) fn new(stdin: I, stdout: O, stderr: O) -> Self {
        let cli = WasiCliCtx {
            environment: Vec::default(),
            arguments: Vec::default(),
            initial_cwd: None,
            stdin,
            stdout,
            stderr,
        };
        let clocks = WasiClocksCtx::default();
        let random = WasiRandomCtx::default();
        let sockets = WasiSocketsCtx::default();
        Self {
            cli,
            clocks,
            random,
            sockets,
            allow_blocking_current_thread: false,
        }
    }

    /// Provides a custom implementation of stdin to use.
    pub fn stdin(&mut self, stdin: I) -> &mut Self {
        self.cli.stdin = stdin;
        self
    }

    /// Same as [`stdin`](WasiCtxBuilder::stdin), but for stdout.
    pub fn stdout(&mut self, stdout: O) -> &mut Self {
        self.cli.stdout = stdout;
        self
    }

    /// Same as [`stdin`](WasiCtxBuilder::stdin), but for stderr.
    pub fn stderr(&mut self, stderr: O) -> &mut Self {
        self.cli.stderr = stderr;
        self
    }

    /// Configures whether or not blocking operations made through this
    /// `WasiCtx` are allowed to block the current thread.
    ///
    /// WASI is currently implemented on top of the Rust
    /// [Tokio](https://tokio.rs/) library. While most WASI APIs are
    /// non-blocking some are instead blocking from the perspective of
    /// WebAssembly. For example opening a file is a blocking operation with
    /// respect to WebAssembly but it's implemented as an asynchronous operation
    /// on the host. This is currently done with Tokio's
    /// [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
    ///
    /// When WebAssembly is used in a synchronous context, for example when
    /// [`Config::async_support`] is disabled, then this asynchronous operation
    /// is quickly turned back into a synchronous operation with a `block_on` in
    /// Rust. This switching back-and-forth between a blocking a non-blocking
    /// context can have overhead, and this option exists to help alleviate this
    /// overhead.
    ///
    /// This option indicates that for WASI functions that are blocking from the
    /// perspective of WebAssembly it's ok to block the native thread as well.
    /// This means that this back-and-forth between async and sync won't happen
    /// and instead blocking operations are performed on-thread (such as opening
    /// a file). This can improve the performance of WASI operations when async
    /// support is disabled.
    ///
    /// [`Config::async_support`]: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.async_support
    pub fn allow_blocking_current_thread(&mut self, enable: bool) -> &mut Self {
        self.allow_blocking_current_thread = enable;
        self
    }

    /// Appends multiple environment variables at once for this builder.
    ///
    /// All environment variables are appended to the list of environment
    /// variables that this builder will configure.
    ///
    /// At this time environment variables are not deduplicated and if the same
    /// key is set twice then the guest will see two entries for the same key.
    pub fn envs(&mut self, env: &[(impl AsRef<str>, impl AsRef<str>)]) -> &mut Self {
        self.cli.environment.extend(
            env.iter()
                .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned())),
        );
        self
    }

    /// Appends a single environment variable for this builder.
    ///
    /// At this time environment variables are not deduplicated and if the same
    /// key is set twice then the guest will see two entries for the same key.
    pub fn env(&mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> &mut Self {
        self.cli
            .environment
            .push((k.as_ref().to_owned(), v.as_ref().to_owned()));
        self
    }

    /// Configures all environment variables to be inherited from the calling
    /// process into this configuration.
    ///
    /// This will use [`envs`](WasiCtxBuilder::envs) to append all host-defined
    /// environment variables.
    pub fn inherit_env(&mut self) -> &mut Self {
        self.envs(&std::env::vars().collect::<Vec<(String, String)>>())
    }

    /// Appends a list of arguments to the argument array to pass to wasm.
    pub fn args(&mut self, args: &[impl AsRef<str>]) -> &mut Self {
        self.cli
            .arguments
            .extend(args.iter().map(|a| a.as_ref().to_owned()));
        self
    }

    /// Appends a single argument to get passed to wasm.
    pub fn arg(&mut self, arg: impl AsRef<str>) -> &mut Self {
        self.cli.arguments.push(arg.as_ref().to_owned());
        self
    }

    /// Appends all host process arguments to the list of arguments to get
    /// passed to wasm.
    pub fn inherit_args(&mut self) -> &mut Self {
        self.args(&std::env::args().collect::<Vec<String>>())
    }

    /// Set the generator for the `wasi:random/random` number generator to the
    /// custom generator specified.
    ///
    /// Note that contexts have a default RNG configured which is a suitable
    /// generator for WASI and is configured with a random seed per-context.
    ///
    /// Guest code may rely on this random number generator to produce fresh
    /// unpredictable random data in order to maintain its security invariants,
    /// and ideally should use the insecure random API otherwise, so using any
    /// prerecorded or otherwise predictable data may compromise security.
    pub fn secure_random(&mut self, random: impl RngCore + Send + 'static) -> &mut Self {
        self.random.random = Box::new(random);
        self
    }

    /// Configures the generator for `wasi:random/insecure`.
    ///
    /// The `insecure_random` generator provided will be used for all randomness
    /// requested by the `wasi:random/insecure` interface.
    pub fn insecure_random(&mut self, insecure_random: impl RngCore + Send + 'static) -> &mut Self {
        self.random.insecure_random = Box::new(insecure_random);
        self
    }

    /// Configures the seed to be returned from `wasi:random/insecure-seed` to
    /// the specified custom value.
    ///
    /// By default this number is randomly generated when a builder is created.
    pub fn insecure_random_seed(&mut self, insecure_random_seed: u128) -> &mut Self {
        self.random.insecure_random_seed = insecure_random_seed;
        self
    }

    /// Configures `wasi:clocks/wall-clock` to use the `clock` specified.
    ///
    /// By default the host's wall clock is used.
    pub fn wall_clock(&mut self, clock: impl HostWallClock + 'static) -> &mut Self {
        self.clocks.wall_clock = Box::new(clock);
        self
    }

    /// Configures `wasi:clocks/monotonic-clock` to use the `clock` specified.
    ///
    /// By default the host's monotonic clock is used.
    pub fn monotonic_clock(&mut self, clock: impl HostMonotonicClock + 'static) -> &mut Self {
        self.clocks.monotonic_clock = Box::new(clock);
        self
    }

    /// Allow all network addresses accessible to the host.
    ///
    /// This method will inherit all network addresses meaning that any address
    /// can be bound by the guest or connected to by the guest using any
    /// protocol.
    ///
    /// See also [`WasiCtxBuilder::socket_addr_check`].
    pub fn inherit_network(&mut self) -> &mut Self {
        self.socket_addr_check(|_, _| Box::pin(async { true }))
    }

    /// A check that will be called for each socket address that is used.
    ///
    /// Returning `true` will permit socket connections to the `SocketAddr`,
    /// while returning `false` will reject the connection.
    pub fn socket_addr_check<F>(&mut self, check: F) -> &mut Self
    where
        F: Fn(SocketAddr, SocketAddrUse) -> Pin<Box<dyn Future<Output = bool> + Send + Sync>>
            + Send
            + Sync
            + 'static,
    {
        self.sockets.socket_addr_check = SocketAddrCheck(Arc::new(check));
        self
    }

    /// Allow usage of `wasi:sockets/ip-name-lookup`
    ///
    /// By default this is disabled.
    pub fn allow_ip_name_lookup(&mut self, enable: bool) -> &mut Self {
        self.sockets.allowed_network_uses.ip_name_lookup = enable;
        self
    }

    /// Allow usage of UDP.
    ///
    /// This is enabled by default, but can be disabled if UDP should be blanket
    /// disabled.
    pub fn allow_udp(&mut self, enable: bool) -> &mut Self {
        self.sockets.allowed_network_uses.udp = enable;
        self
    }

    /// Allow usage of TCP
    ///
    /// This is enabled by default, but can be disabled if TCP should be blanket
    /// disabled.
    pub fn allow_tcp(&mut self, enable: bool) -> &mut Self {
        self.sockets.allowed_network_uses.tcp = enable;
        self
    }
}
