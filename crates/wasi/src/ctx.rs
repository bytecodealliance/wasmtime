use crate::{
    clocks::{
        host::{monotonic_clock, wall_clock},
        HostMonotonicClock, HostWallClock,
    },
    filesystem::{Dir, OpenMode},
    network::{SocketAddrCheck, SocketAddrUse},
    pipe, random,
    runtime::WasiExecutor,
    stdio,
    stdio::{StdinStream, StdoutStream},
    DirPerms, FilePerms,
};
use anyhow::Result;
use cap_rand::{Rng, RngCore, SeedableRng};
use cap_std::ambient_authority;
use std::path::Path;
use std::sync::Arc;
use std::{future::Future, pin::Pin};
use std::{mem, net::SocketAddr};
use wasmtime::component::ResourceTable;

/// Builder-style structure used to create a [`WasiCtx`].
///
/// This type is used to create a [`WasiCtx`] that is considered per-[`Store`]
/// state. The [`build`][WasiCtxBuilder::build] method is used to finish the
/// building process and produce a finalized [`WasiCtx`].
///
/// # Examples
///
/// ```
/// use wasmtime_wasi::{WasiCtxBuilder, WasiCtx};
///
/// let mut wasi = WasiCtxBuilder::new();
/// wasi.arg("./foo.wasm");
/// wasi.arg("--help");
/// wasi.env("FOO", "bar");
///
/// let wasi: WasiCtx = wasi.build();
/// ```
///
/// [`Store`]: wasmtime::Store
pub struct WasiCtxBuilder {
    stdin: Box<dyn StdinStream>,
    stdout: Box<dyn StdoutStream>,
    stderr: Box<dyn StdoutStream>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Dir, String)>,
    socket_addr_check: SocketAddrCheck,
    random: Box<dyn RngCore + Send>,
    insecure_random: Box<dyn RngCore + Send>,
    insecure_random_seed: u128,
    wall_clock: Box<dyn HostWallClock + Send>,
    monotonic_clock: Box<dyn HostMonotonicClock + Send>,
    allowed_network_uses: AllowedNetworkUses,
    built: bool,
}

impl WasiCtxBuilder {
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
    /// * `wasi:network/ip-name-lookup` is denied by default.
    ///
    /// These defaults can all be updated via the various builder configuration
    /// methods below.
    pub fn new() -> Self {
        // For the insecure random API, use `SmallRng`, which is fast. It's
        // also insecure, but that's the deal here.
        let insecure_random = Box::new(
            cap_rand::rngs::SmallRng::from_rng(cap_rand::thread_rng(cap_rand::ambient_authority()))
                .unwrap(),
        );

        // For the insecure random seed, use a `u128` generated from
        // `thread_rng()`, so that it's not guessable from the insecure_random
        // API.
        let insecure_random_seed =
            cap_rand::thread_rng(cap_rand::ambient_authority()).gen::<u128>();
        Self {
            stdin: Box::new(pipe::ClosedInputStream),
            stdout: Box::new(pipe::SinkOutputStream),
            stderr: Box::new(pipe::SinkOutputStream),
            env: Vec::new(),
            args: Vec::new(),
            preopens: Vec::new(),
            socket_addr_check: SocketAddrCheck::default(),
            random: random::thread_rng(),
            insecure_random,
            insecure_random_seed,
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
            allowed_network_uses: AllowedNetworkUses::default(),
            built: false,
        }
    }

    /// Provides a custom implementation of stdin to use.
    ///
    /// By default stdin is closed but an example of using the host's native
    /// stdin looks like:
    ///
    /// ```
    /// use wasmtime_wasi::{stdin, WasiCtxBuilder};
    ///
    /// let mut wasi = WasiCtxBuilder::new();
    /// wasi.stdin(stdin());
    /// ```
    ///
    /// Note that inheriting the process's stdin can also be done through
    /// [`inherit_stdin`](WasiCtxBuilder::inherit_stdin).
    pub fn stdin(&mut self, stdin: impl StdinStream + 'static) -> &mut Self {
        self.stdin = Box::new(stdin);
        self
    }

    /// Same as [`stdin`](WasiCtxBuilder::stdin), but for stdout.
    pub fn stdout(&mut self, stdout: impl StdoutStream + 'static) -> &mut Self {
        self.stdout = Box::new(stdout);
        self
    }

    /// Same as [`stdin`](WasiCtxBuilder::stdin), but for stderr.
    pub fn stderr(&mut self, stderr: impl StdoutStream + 'static) -> &mut Self {
        self.stderr = Box::new(stderr);
        self
    }

    /// Configures this context's stdin stream to read the host process's
    /// stdin.
    ///
    /// Note that concurrent reads of stdin can produce surprising results so
    /// when using this it's typically best to have a single wasm instance in
    /// the process using this.
    pub fn inherit_stdin(&mut self) -> &mut Self {
        self.stdin(stdio::stdin())
    }

    /// Configures this context's stdout stream to write to the host process's
    /// stdout.
    ///
    /// Note that unlike [`inherit_stdin`](WasiCtxBuilder::inherit_stdin)
    /// multiple instances printing to stdout works well.
    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.stdout(stdio::stdout())
    }

    /// Configures this context's stderr stream to write to the host process's
    /// stderr.
    ///
    /// Note that unlike [`inherit_stdin`](WasiCtxBuilder::inherit_stdin)
    /// multiple instances printing to stderr works well.
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.stderr(stdio::stderr())
    }

    /// Configures all of stdin, stdout, and stderr to be inherited from the
    /// host process.
    ///
    /// See [`inherit_stdin`](WasiCtxBuilder::inherit_stdin) for some rationale
    /// on why this should only be done in situations of
    /// one-instance-per-process.
    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }

    /// Appends multiple environment variables at once for this builder.
    ///
    /// All environment variables are appended to the list of environment
    /// variables that this builder will configure.
    ///
    /// At this time environment variables are not deduplicated and if the same
    /// key is set twice then the guest will see two entries for the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime_wasi::{stdin, WasiCtxBuilder};
    ///
    /// let mut wasi = WasiCtxBuilder::new();
    /// wasi.envs(&[
    ///     ("FOO", "bar"),
    ///     ("HOME", "/somewhere"),
    /// ]);
    /// ```
    pub fn envs(&mut self, env: &[(impl AsRef<str>, impl AsRef<str>)]) -> &mut Self {
        self.env.extend(
            env.iter()
                .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned())),
        );
        self
    }

    /// Appends a single environment variable for this builder.
    ///
    /// At this time environment variables are not deduplicated and if the same
    /// key is set twice then the guest will see two entries for the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime_wasi::{stdin, WasiCtxBuilder};
    ///
    /// let mut wasi = WasiCtxBuilder::new();
    /// wasi.env("FOO", "bar");
    /// ```
    pub fn env(&mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> &mut Self {
        self.env
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
        self.args.extend(args.iter().map(|a| a.as_ref().to_owned()));
        self
    }

    /// Appends a single argument to get passed to wasm.
    pub fn arg(&mut self, arg: impl AsRef<str>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    /// Appends all host process arguments to the list of arguments to get
    /// passed to wasm.
    pub fn inherit_args(&mut self) -> &mut Self {
        self.args(&std::env::args().collect::<Vec<String>>())
    }

    /// Configures a "preopened directory" to be available to WebAssembly.
    ///
    /// By default WebAssembly does not have access to the filesystem because
    /// there are no preopened directories. All filesystem operations, such as
    /// opening a file, are done through a preexisting handle. This means that
    /// to provide WebAssembly access to a directory it must be configured
    /// through this API.
    ///
    /// WASI will also prevent access outside of files provided here. For
    /// example `..` can't be used to traverse up from the `host_path` provided here
    /// to the containing directory.
    ///
    /// * `host_path` - a path to a directory on the host to open and make
    ///   accessible to WebAssembly. Note that the name of this directory in the
    ///   guest is configured with `guest_path` below.
    /// * `guest_path` - the name of the preopened directory from WebAssembly's
    ///   perspective. Note that this does not need to match the host's name for
    ///   the directory.
    /// * `dir_perms` - this is the permissions that wasm will have to operate on
    ///   `guest_path`. This can be used, for example, to provide readonly access to a
    ///   directory.
    /// * `file_perms` - similar to `dir_perms` but corresponds to the maximum set
    ///   of permissions that can be used for any file in this directory.
    ///
    /// # Errors
    ///
    /// This method will return an error if `host_path` cannot be opened.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime_wasi::{WasiCtxBuilder, DirPerms, FilePerms};
    ///
    /// # fn main() {}
    /// # fn foo() -> wasmtime::Result<()> {
    /// let mut wasi = WasiCtxBuilder::new();
    ///
    /// // Make `./host-directory` available in the guest as `.`
    /// wasi.preopened_dir("./host-directory", ".", DirPerms::all(), FilePerms::all());
    ///
    /// // Make `./readonly` available in the guest as `./ro`
    /// wasi.preopened_dir("./readonly", "./ro", DirPerms::READ, FilePerms::READ);
    /// # Ok(())
    /// # }
    /// ```
    pub fn preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: impl AsRef<str>,
        dir_perms: DirPerms,
        file_perms: FilePerms,
    ) -> Result<&mut Self> {
        let dir = cap_std::fs::Dir::open_ambient_dir(host_path.as_ref(), ambient_authority())?;
        let mut open_mode = OpenMode::empty();
        if dir_perms.contains(DirPerms::READ) {
            open_mode |= OpenMode::READ;
        }
        if dir_perms.contains(DirPerms::MUTATE) {
            open_mode |= OpenMode::WRITE;
        }
        self.preopens.push((
            Dir::new(dir, dir_perms, file_perms, open_mode),
            guest_path.as_ref().to_owned(),
        ));
        Ok(self)
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
        self.random = Box::new(random);
        self
    }

    /// Configures the generator for `wasi:random/insecure`.
    ///
    /// The `insecure_random` generator provided will be used for all randomness
    /// requested by the `wasi:random/insecure` interface.
    pub fn insecure_random(&mut self, insecure_random: impl RngCore + Send + 'static) -> &mut Self {
        self.insecure_random = Box::new(insecure_random);
        self
    }

    /// Configures the seed to be returned from `wasi:random/insecure-seed` to
    /// the specified custom value.
    ///
    /// By default this number is randomly generated when a builder is created.
    pub fn insecure_random_seed(&mut self, insecure_random_seed: u128) -> &mut Self {
        self.insecure_random_seed = insecure_random_seed;
        self
    }

    /// Configures `wasi:clocks/wall-clock` to use the `clock` specified.
    ///
    /// By default the host's wall clock is used.
    pub fn wall_clock(&mut self, clock: impl HostWallClock + 'static) -> &mut Self {
        self.wall_clock = Box::new(clock);
        self
    }

    /// Configures `wasi:clocks/monotonic-clock` to use the `clock` specified.
    ///
    /// By default the host's monotonic clock is used.
    pub fn monotonic_clock(&mut self, clock: impl HostMonotonicClock + 'static) -> &mut Self {
        self.monotonic_clock = Box::new(clock);
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
        self.socket_addr_check = SocketAddrCheck(Arc::new(check));
        self
    }

    /// Allow usage of `wasi:sockets/ip-name-lookup`
    ///
    /// By default this is disabled.
    pub fn allow_ip_name_lookup(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.ip_name_lookup = enable;
        self
    }

    /// Allow usage of UDP.
    ///
    /// This is enabled by default, but can be disabled if UDP should be blanket
    /// disabled.
    pub fn allow_udp(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.udp = enable;
        self
    }

    /// Allow usage of TCP
    ///
    /// This is enabled by default, but can be disabled if TCP should be blanket
    /// disabled.
    pub fn allow_tcp(&mut self, enable: bool) -> &mut Self {
        self.allowed_network_uses.tcp = enable;
        self
    }

    /// Uses the configured context so far to construct the final [`WasiCtx`].
    ///
    /// Note that each `WasiCtxBuilder` can only be used to "build" once, and
    /// calling this method twice will panic.
    ///
    /// # Panics
    ///
    /// Panics if this method is called twice. Each [`WasiCtxBuilder`] can be
    /// used to create only a single [`WasiCtx`]. Repeated usage of this method
    /// is not allowed and should use a second builder instead.
    pub fn build<E>(&mut self) -> WasiCtx<E> {
        assert!(!self.built);

        let Self {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            socket_addr_check,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
            allowed_network_uses,
            built: _,
        } = mem::replace(self, Self::new());
        self.built = true;

        WasiCtx {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            socket_addr_check,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
            allowed_network_uses,
            _executor: std::marker::PhantomData,
        }
    }

    /// Builds a WASIp1 context instead of a [`WasiCtx`].
    ///
    /// This method is the same as [`build`](WasiCtxBuilder::build) but it
    /// creates a [`WasiP1Ctx`] instead. This is intended for use with the
    /// [`preview1`] module of this crate
    ///
    /// [`WasiP1Ctx`]: crate::preview1::WasiP1Ctx
    /// [`preview1`]: crate::preview1
    ///
    /// # Panics
    ///
    /// Panics if this method is called twice. Each [`WasiCtxBuilder`] can be
    /// used to create only a single [`WasiCtx`] or [`WasiP1Ctx`]. Repeated
    /// usage of this method is not allowed and should use a second builder
    /// instead.
    #[cfg(feature = "preview1")]
    pub fn build_p1<E>(&mut self) -> crate::preview1::WasiP1Ctx<E> {
        let wasi = self.build();
        crate::preview1::WasiP1Ctx::new(wasi)
    }
}

/// A trait which provides access to internal WASI state.
///
/// This trait is the basis of implementation of all traits in this crate. All
/// traits are implemented like:
///
/// ```
/// # trait WasiView {}
/// # mod bindings { pub mod wasi { pub trait Host {} } }
/// impl<T: WasiView> bindings::wasi::Host for T {
///     // ...
/// }
/// ```
///
/// For a [`Store<T>`](wasmtime::Store) this trait will be implemented
/// for the `T`. This also corresponds to the `T` in
/// [`Linker<T>`](wasmtime::component::Linker).
///
/// # Example
///
/// ```
/// use wasmtime_wasi::{WasiCtx, ResourceTable, WasiView, Tokio, WasiCtxBuilder};
///
/// struct MyState {
///     ctx: WasiCtx,
///     table: ResourceTable,
/// }
///
/// impl WasiView for MyState {
///     type Executor = Tokio;
///     fn ctx(&mut self) -> &mut WasiCtx<Tokio> { &mut self.ctx }
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
///
/// impl MyState {
///     fn new() -> MyState {
///         let mut wasi = WasiCtxBuilder::new();
///         wasi.arg("./foo.wasm");
///         wasi.arg("--help");
///         wasi.env("FOO", "bar");
///
///         MyState {
///             ctx: wasi.build(),
///             table: ResourceTable::new(),
///         }
///     }
/// }
/// ```
pub trait WasiView: Send {
    type Executor: WasiExecutor;
    /// Yields mutable access to the internal resource management that this
    /// context contains.
    ///
    /// Embedders can add custom resources to this table as well to give
    /// resources to wasm as well.
    fn table(&mut self) -> &mut ResourceTable;

    /// Yields mutable access to the configuration used for this context.
    ///
    /// The returned type is created through [`WasiCtxBuilder`].
    fn ctx(&mut self) -> &mut WasiCtx<Self::Executor>;
}

impl<T: ?Sized + WasiView> WasiView for &mut T {
    type Executor = T::Executor;
    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }
    fn ctx(&mut self) -> &mut WasiCtx<Self::Executor> {
        T::ctx(self)
    }
}

impl<T: ?Sized + WasiView> WasiView for Box<T> {
    type Executor = T::Executor;
    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }
    fn ctx(&mut self) -> &mut WasiCtx<Self::Executor> {
        T::ctx(self)
    }
}

/// A small newtype wrapper which serves as the basis for implementations of
/// `Host` WASI traits in this crate.
///
/// This type is used as the basis for the implementation of all `Host` traits
/// generated by `bindgen!` for WASI interfaces. This is used automatically with
/// [`add_to_linker_sync`](crate::add_to_linker_sync) and
/// [`add_to_linker_async`](crate::add_to_linker_async).
///
/// This type is otherwise provided if you're calling the `add_to_linker`
/// functions generated by `bindgen!` from the [`bindings`
/// module](crate::bindings). In this situation you'll want to create a value of
/// this type in the closures added to a `Linker`.
#[repr(transparent)]
pub struct WasiImpl<T>(pub T);

impl<T: WasiView> WasiView for WasiImpl<T> {
    type Executor = T::Executor;
    fn table(&mut self) -> &mut ResourceTable {
        T::table(&mut self.0)
    }
    fn ctx(&mut self) -> &mut WasiCtx<Self::Executor> {
        T::ctx(&mut self.0)
    }
}

/// Per-[`Store`] state which holds state necessary to implement WASI from this
/// crate.
///
/// This structure is created through [`WasiCtxBuilder`] and is stored within
/// the `T` of [`Store<T>`][`Store`]. Access to the structure is provided
/// through the [`WasiView`] trait as an implementation on `T`.
///
/// Note that this structure itself does not have any accessors, it's here for
/// internal use within the `wasmtime-wasi` crate's implementation of
/// bindgen-generated traits.
///
/// [`Store`]: wasmtime::Store
pub struct WasiCtx<E> {
    pub(crate) random: Box<dyn RngCore + Send>,
    pub(crate) insecure_random: Box<dyn RngCore + Send>,
    pub(crate) insecure_random_seed: u128,
    pub(crate) wall_clock: Box<dyn HostWallClock + Send>,
    pub(crate) monotonic_clock: Box<dyn HostMonotonicClock + Send>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) args: Vec<String>,
    pub(crate) preopens: Vec<(Dir, String)>,
    pub(crate) stdin: Box<dyn StdinStream>,
    pub(crate) stdout: Box<dyn StdoutStream>,
    pub(crate) stderr: Box<dyn StdoutStream>,
    pub(crate) socket_addr_check: SocketAddrCheck,
    pub(crate) allowed_network_uses: AllowedNetworkUses,
    pub(crate) _executor: std::marker::PhantomData<E>,
}

impl<E> WasiCtx<E> {
    /// Convenience function for calling [`WasiCtxBuilder::new`].
    pub fn builder() -> WasiCtxBuilder {
        WasiCtxBuilder::new()
    }
}

pub struct AllowedNetworkUses {
    pub ip_name_lookup: bool,
    pub udp: bool,
    pub tcp: bool,
}

impl Default for AllowedNetworkUses {
    fn default() -> Self {
        Self {
            ip_name_lookup: false,
            udp: true,
            tcp: true,
        }
    }
}

impl AllowedNetworkUses {
    pub(crate) fn check_allowed_udp(&self) -> std::io::Result<()> {
        if !self.udp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "UDP is not allowed",
            ));
        }

        Ok(())
    }

    pub(crate) fn check_allowed_tcp(&self) -> std::io::Result<()> {
        if !self.tcp {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "TCP is not allowed",
            ));
        }

        Ok(())
    }
}
