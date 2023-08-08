//! The module that implements the `wasmtime run` command.

use anyhow::{anyhow, bail, Context as _, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wasmtime::{
    AsContextMut, Engine, Func, GuestProfiler, Linker, Module, Store, StoreLimits,
    StoreLimitsBuilder, UpdateDeadline, Val, ValType,
};
use wasmtime_cli_flags::{CommonOptions, WasiModules};
use wasmtime_wasi::maybe_exit_on_error;
use wasmtime_wasi::sync::{ambient_authority, Dir, TcpListener, WasiCtxBuilder};

#[cfg(feature = "wasi-nn")]
use wasmtime_wasi_nn::WasiNnCtx;

#[cfg(feature = "wasi-threads")]
use wasmtime_wasi_threads::WasiThreadsCtx;

#[cfg(feature = "wasi-http")]
use wasmtime_wasi_http::WasiHttp;

fn parse_env_var(s: &str) -> Result<(String, Option<String>)> {
    let mut parts = s.splitn(2, '=');
    Ok((
        parts.next().unwrap().to_string(),
        parts.next().map(|s| s.to_string()),
    ))
}

fn parse_map_dirs(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 2 {
        bail!("must contain exactly one double colon ('::')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

fn parse_dur(s: &str) -> Result<Duration> {
    // assume an integer without a unit specified is a number of seconds ...
    if let Ok(val) = s.parse() {
        return Ok(Duration::from_secs(val));
    }
    // ... otherwise try to parse it with units such as `3s` or `300ms`
    let dur = humantime::parse_duration(s)?;
    Ok(dur)
}

fn parse_preloads(s: &str) -> Result<(String, PathBuf)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("must contain exactly one equals character ('=')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

fn parse_profile(s: &str) -> Result<Profile> {
    let parts = s.split(',').collect::<Vec<_>>();
    match &parts[..] {
        ["perfmap"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::PerfMap)),
        ["jitdump"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::JitDump)),
        ["vtune"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::VTune)),
        ["guest"] => Ok(Profile::Guest {
            path: "wasmtime-guest-profile.json".to_string(),
            interval: Duration::from_millis(10),
        }),
        ["guest", path] => Ok(Profile::Guest {
            path: path.to_string(),
            interval: Duration::from_millis(10),
        }),
        ["guest", path, dur] => Ok(Profile::Guest {
            path: path.to_string(),
            interval: parse_dur(dur)?,
        }),
        _ => bail!("unknown profiling strategy: {s}"),
    }
}

static AFTER_HELP: Lazy<String> = Lazy::new(|| crate::FLAG_EXPLANATIONS.to_string());

/// Runs a WebAssembly module
#[derive(Parser)]
#[structopt(name = "run", after_help = AFTER_HELP.as_str())]
pub struct RunCommand {
    #[clap(flatten)]
    common: CommonOptions,

    /// Allow unknown exports when running commands.
    #[clap(long = "allow-unknown-exports")]
    allow_unknown_exports: bool,

    /// Allow the main module to import unknown functions, using an
    /// implementation that immediately traps, when running commands.
    #[clap(long = "trap-unknown-imports")]
    trap_unknown_imports: bool,

    /// Allow the main module to import unknown functions, using an
    /// implementation that returns default values, when running commands.
    #[clap(long = "default-values-unknown-imports")]
    default_values_unknown_imports: bool,

    /// Allow executing precompiled WebAssembly modules as `*.cwasm` files.
    ///
    /// Note that this option is not safe to pass if the module being passed in
    /// is arbitrary user input. Only `wasmtime`-precompiled modules generated
    /// via the `wasmtime compile` command or equivalent should be passed as an
    /// argument with this option specified.
    #[clap(long = "allow-precompiled")]
    allow_precompiled: bool,

    /// Inherit environment variables and file descriptors following the
    /// systemd listen fd specification (UNIX only)
    #[clap(long = "listenfd")]
    listenfd: bool,

    /// Grant access to the given TCP listen socket
    #[clap(
        long = "tcplisten",
        number_of_values = 1,
        value_name = "SOCKET ADDRESS"
    )]
    tcplisten: Vec<String>,

    /// Grant access to the given host directory
    #[clap(long = "dir", number_of_values = 1, value_name = "DIRECTORY")]
    dirs: Vec<String>,

    /// Pass an environment variable to the program.
    ///
    /// The `--env FOO=BAR` form will set the environment variable named `FOO`
    /// to the value `BAR` for the guest program using WASI. The `--env FOO`
    /// form will set the environment variable named `FOO` to the same value it
    /// has in the calling process for the guest, or in other words it will
    /// cause the environment variable `FOO` to be inherited.
    #[clap(long = "env", number_of_values = 1, value_name = "NAME[=VAL]", value_parser = parse_env_var)]
    vars: Vec<(String, Option<String>)>,

    /// The name of the function to run
    #[clap(long, value_name = "FUNCTION")]
    invoke: Option<String>,

    /// Grant access to a guest directory mapped as a host directory
    #[clap(long = "mapdir", number_of_values = 1, value_name = "GUEST_DIR::HOST_DIR", value_parser = parse_map_dirs)]
    map_dirs: Vec<(String, String)>,

    /// Load the given WebAssembly module before the main module
    #[clap(
        long = "preload",
        number_of_values = 1,
        value_name = "NAME=MODULE_PATH",
        value_parser = parse_preloads,
    )]
    preloads: Vec<(String, PathBuf)>,

    /// Maximum execution time of wasm code before timing out (1, 2s, 100ms, etc)
    #[clap(
        long = "wasm-timeout",
        value_name = "TIME",
        value_parser = parse_dur,
    )]
    wasm_timeout: Option<Duration>,

    /// Profiling strategy (valid options are: perfmap, jitdump, vtune, guest)
    ///
    /// The perfmap, jitdump, and vtune profiling strategies integrate Wasmtime
    /// with external profilers such as `perf`. The guest profiling strategy
    /// enables in-process sampling and will write the captured profile to
    /// `wasmtime-guest-profile.json` by default which can be viewed at
    /// https://profiler.firefox.com/.
    ///
    /// The `guest` option can be additionally configured as:
    ///
    ///     --profile=guest[,path[,interval]]
    ///
    /// where `path` is where to write the profile and `interval` is the
    /// duration between samples. When used with `--wasm-timeout` the timeout
    /// will be rounded up to the nearest multiple of this interval.
    #[clap(
        long,
        value_name = "STRATEGY",
        value_parser = parse_profile,
    )]
    profile: Option<Profile>,

    /// Enable coredump generation after a WebAssembly trap.
    #[clap(long = "coredump-on-trap", value_name = "PATH")]
    coredump_on_trap: Option<String>,

    /// Maximum size, in bytes, that a linear memory is allowed to reach.
    ///
    /// Growth beyond this limit will cause `memory.grow` instructions in
    /// WebAssembly modules to return -1 and fail.
    #[clap(long, value_name = "BYTES")]
    max_memory_size: Option<usize>,

    /// Maximum size, in table elements, that a table is allowed to reach.
    #[clap(long)]
    max_table_elements: Option<u32>,

    /// Maximum number of WebAssembly instances allowed to be created.
    #[clap(long)]
    max_instances: Option<usize>,

    /// Maximum number of WebAssembly tables allowed to be created.
    #[clap(long)]
    max_tables: Option<usize>,

    /// Maximum number of WebAssembly linear memories allowed to be created.
    #[clap(long)]
    max_memories: Option<usize>,

    /// Force a trap to be raised on `memory.grow` and `table.grow` failure
    /// instead of returning -1 from these instructions.
    ///
    /// This is not necessarily a spec-compliant option to enable but can be
    /// useful for tracking down a backtrace of what is requesting so much
    /// memory, for example.
    #[clap(long)]
    trap_on_grow_failure: bool,

    /// The WebAssembly module to run and arguments to pass to it.
    ///
    /// Arguments passed to the wasm module will be configured as WASI CLI
    /// arguments unless the `--invoke` CLI argument is passed in which case
    /// arguments will be interpreted as arguments to the function specified.
    #[clap(value_name = "WASM", trailing_var_arg = true, required = true)]
    module_and_args: Vec<PathBuf>,
}

#[derive(Clone)]
enum Profile {
    Native(wasmtime::ProfilingStrategy),
    Guest { path: String, interval: Duration },
}

impl RunCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        self.common.init_logging();

        let mut config = self.common.config(None)?;

        if self.wasm_timeout.is_some() {
            config.epoch_interruption(true);
        }
        match self.profile {
            Some(Profile::Native(s)) => {
                config.profiler(s);
            }
            Some(Profile::Guest { .. }) => {
                // Further configured down below as well.
                config.epoch_interruption(true);
            }
            None => {}
        }

        let engine = Engine::new(&config)?;

        let preopen_sockets = self.compute_preopen_sockets()?;

        // Validate coredump-on-trap argument
        if let Some(coredump_path) = self.coredump_on_trap.as_ref() {
            if coredump_path.contains("%") {
                bail!("the coredump-on-trap path does not support patterns yet.")
            }
        }

        // Make wasi available by default.
        let preopen_dirs = self.compute_preopen_dirs()?;
        let argv = self.compute_argv()?;

        let mut linker = Linker::new(&engine);
        linker.allow_unknown_exports(self.allow_unknown_exports);

        // Read the wasm module binary either as `*.wat` or a raw binary.
        let module = self.load_module(linker.engine(), &self.module_and_args[0])?;
        let mut modules = vec![(String::new(), module.clone())];

        let host = Host::default();
        let mut store = Store::new(&engine, host);
        populate_with_wasi(
            &mut linker,
            &mut store,
            module.clone(),
            preopen_dirs,
            &argv,
            &self.vars,
            &self.common.wasi_modules.unwrap_or(WasiModules::default()),
            self.listenfd,
            preopen_sockets,
        )?;

        let mut limits = StoreLimitsBuilder::new();
        if let Some(max) = self.max_memory_size {
            limits = limits.memory_size(max);
        }
        if let Some(max) = self.max_table_elements {
            limits = limits.table_elements(max);
        }
        if let Some(max) = self.max_instances {
            limits = limits.instances(max);
        }
        if let Some(max) = self.max_tables {
            limits = limits.tables(max);
        }
        if let Some(max) = self.max_memories {
            limits = limits.memories(max);
        }
        store.data_mut().limits = limits
            .trap_on_grow_failure(self.trap_on_grow_failure)
            .build();
        store.limiter(|t| &mut t.limits);

        // If fuel has been configured, we want to add the configured
        // fuel amount to this store.
        if let Some(fuel) = self.common.fuel {
            store.add_fuel(fuel)?;
        }

        // Load the preload wasm modules.
        for (name, path) in self.preloads.iter() {
            // Read the wasm module binary either as `*.wat` or a raw binary
            let module = self.load_module(&engine, path)?;
            modules.push((name.clone(), module.clone()));

            // Add the module's functions to the linker.
            linker.module(&mut store, name, &module).context(format!(
                "failed to process preload `{}` at `{}`",
                name,
                path.display()
            ))?;
        }

        // Load the main wasm module.
        match self
            .load_main_module(&mut store, &mut linker, module, modules, &argv[0])
            .with_context(|| {
                format!(
                    "failed to run main module `{}`",
                    self.module_and_args[0].display()
                )
            }) {
            Ok(()) => (),
            Err(e) => {
                // Exit the process if Wasmtime understands the error;
                // otherwise, fall back on Rust's default error printing/return
                // code.
                return Err(maybe_exit_on_error(e));
            }
        }

        Ok(())
    }

    fn compute_preopen_dirs(&self) -> Result<Vec<(String, Dir)>> {
        let mut preopen_dirs = Vec::new();

        for dir in self.dirs.iter() {
            preopen_dirs.push((
                dir.clone(),
                Dir::open_ambient_dir(dir, ambient_authority())
                    .with_context(|| format!("failed to open directory '{}'", dir))?,
            ));
        }

        for (guest, host) in self.map_dirs.iter() {
            preopen_dirs.push((
                guest.clone(),
                Dir::open_ambient_dir(host, ambient_authority())
                    .with_context(|| format!("failed to open directory '{}'", host))?,
            ));
        }

        Ok(preopen_dirs)
    }

    fn compute_preopen_sockets(&self) -> Result<Vec<TcpListener>> {
        let mut listeners = vec![];

        for address in &self.tcplisten {
            let stdlistener = std::net::TcpListener::bind(address)
                .with_context(|| format!("failed to bind to address '{}'", address))?;

            let _ = stdlistener.set_nonblocking(true)?;

            listeners.push(TcpListener::from_std(stdlistener))
        }
        Ok(listeners)
    }

    fn compute_argv(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();

        for (i, arg) in self.module_and_args.iter().enumerate() {
            // For argv[0], which is the program name. Only include the base
            // name of the main wasm module, to avoid leaking path information.
            let arg = if i == 0 {
                arg.components().next_back().unwrap().as_os_str()
            } else {
                arg.as_ref()
            };
            result.push(
                arg.to_str()
                    .ok_or_else(|| anyhow!("failed to convert {arg:?} to utf-8"))?
                    .to_string(),
            );
        }

        Ok(result)
    }

    fn setup_epoch_handler(
        &self,
        store: &mut Store<Host>,
        module_name: &str,
        modules: Vec<(String, Module)>,
    ) -> Box<dyn FnOnce(&mut Store<Host>)> {
        if let Some(Profile::Guest { path, interval }) = &self.profile {
            let interval = *interval;
            store.data_mut().guest_profiler =
                Some(Arc::new(GuestProfiler::new(module_name, interval, modules)));

            fn sample(mut store: impl AsContextMut<Data = Host>) {
                let mut profiler = store
                    .as_context_mut()
                    .data_mut()
                    .guest_profiler
                    .take()
                    .unwrap();
                Arc::get_mut(&mut profiler)
                    .expect("profiling doesn't support threads yet")
                    .sample(&store);
                store.as_context_mut().data_mut().guest_profiler = Some(profiler);
            }

            if let Some(timeout) = self.wasm_timeout {
                let mut timeout = (timeout.as_secs_f64() / interval.as_secs_f64()).ceil() as u64;
                assert!(timeout > 0);
                store.epoch_deadline_callback(move |mut store| {
                    sample(&mut store);
                    timeout -= 1;
                    if timeout == 0 {
                        bail!("timeout exceeded");
                    }
                    Ok(UpdateDeadline::Continue(1))
                });
            } else {
                store.epoch_deadline_callback(move |mut store| {
                    sample(&mut store);
                    Ok(UpdateDeadline::Continue(1))
                });
            }

            store.set_epoch_deadline(1);
            let engine = store.engine().clone();
            thread::spawn(move || loop {
                thread::sleep(interval);
                engine.increment_epoch();
            });

            let path = path.clone();
            return Box::new(move |store| {
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
        }

        if let Some(timeout) = self.wasm_timeout {
            store.set_epoch_deadline(1);
            let engine = store.engine().clone();
            thread::spawn(move || {
                thread::sleep(timeout);
                engine.increment_epoch();
            });
        }

        Box::new(|_store| {})
    }

    fn load_main_module(
        &self,
        store: &mut Store<Host>,
        linker: &mut Linker<Host>,
        module: Module,
        modules: Vec<(String, Module)>,
        module_name: &str,
    ) -> Result<()> {
        // The main module might be allowed to have unknown imports, which
        // should be defined as traps:
        if self.trap_unknown_imports {
            linker.define_unknown_imports_as_traps(&module)?;
        }

        // ...or as default values.
        if self.default_values_unknown_imports {
            linker.define_unknown_imports_as_default_values(&module)?;
        }

        // Use "" as a default module name.
        linker.module(&mut *store, "", &module).context(format!(
            "failed to instantiate {:?}",
            self.module_and_args[0]
        ))?;

        // If a function to invoke was given, invoke it.
        let func = if let Some(name) = &self.invoke {
            self.find_export(store, linker, name)?
        } else {
            linker.get_default(&mut *store, "")?
        };

        // Finish all lookups before starting any epoch timers.
        let finish_epoch_handler = self.setup_epoch_handler(store, module_name, modules);
        let result = self.invoke_func(store, func);
        finish_epoch_handler(store);
        result
    }

    fn find_export(
        &self,
        store: &mut Store<Host>,
        linker: &Linker<Host>,
        name: &str,
    ) -> Result<Func> {
        let func = match linker
            .get(&mut *store, "", name)
            .ok_or_else(|| anyhow!("no export named `{}` found", name))?
            .into_func()
        {
            Some(func) => func,
            None => bail!("export of `{}` wasn't a function", name),
        };
        Ok(func)
    }

    fn invoke_func(&self, store: &mut Store<Host>, func: Func) -> Result<()> {
        let ty = func.ty(&store);
        if ty.params().len() > 0 {
            eprintln!(
                "warning: using `--invoke` with a function that takes arguments \
                 is experimental and may break in the future"
            );
        }
        let mut args = self.module_and_args.iter().skip(1);
        let mut values = Vec::new();
        for ty in ty.params() {
            let val = match args.next() {
                Some(s) => s,
                None => {
                    if let Some(name) = &self.invoke {
                        bail!("not enough arguments for `{}`", name)
                    } else {
                        bail!("not enough arguments for command default")
                    }
                }
            };
            let val = val
                .to_str()
                .ok_or_else(|| anyhow!("argument is not valid utf-8: {val:?}"))?;
            values.push(match ty {
                // TODO: integer parsing here should handle hexadecimal notation
                // like `0x0...`, but the Rust standard library currently only
                // parses base-10 representations.
                ValType::I32 => Val::I32(val.parse()?),
                ValType::I64 => Val::I64(val.parse()?),
                ValType::F32 => Val::F32(val.parse()?),
                ValType::F64 => Val::F64(val.parse()?),
                t => bail!("unsupported argument type {:?}", t),
            });
        }

        // Invoke the function and then afterwards print all the results that came
        // out, if there are any.
        let mut results = vec![Val::null(); ty.results().len()];
        let invoke_res = func.call(store, &values, &mut results).with_context(|| {
            if let Some(name) = &self.invoke {
                format!("failed to invoke `{}`", name)
            } else {
                format!("failed to invoke command default")
            }
        });

        if let Err(err) = invoke_res {
            let err = if err.is::<wasmtime::Trap>() {
                if let Some(coredump_path) = self.coredump_on_trap.as_ref() {
                    let source_name = self.module_and_args[0]
                        .to_str()
                        .unwrap_or_else(|| "unknown");

                    if let Err(coredump_err) = generate_coredump(&err, &source_name, coredump_path)
                    {
                        eprintln!("warning: coredump failed to generate: {}", coredump_err);
                        err
                    } else {
                        err.context(format!("core dumped at {}", coredump_path))
                    }
                } else {
                    err
                }
            } else {
                err
            };
            return Err(err);
        }

        if !results.is_empty() {
            eprintln!(
                "warning: using `--invoke` with a function that returns values \
                 is experimental and may break in the future"
            );
        }

        for result in results {
            match result {
                Val::I32(i) => println!("{}", i),
                Val::I64(i) => println!("{}", i),
                Val::F32(f) => println!("{}", f32::from_bits(f)),
                Val::F64(f) => println!("{}", f64::from_bits(f)),
                Val::ExternRef(_) => println!("<externref>"),
                Val::FuncRef(_) => println!("<funcref>"),
                Val::V128(i) => println!("{}", i),
            }
        }

        Ok(())
    }

    fn load_module(&self, engine: &Engine, path: &Path) -> Result<Module> {
        let path = match path.to_str() {
            #[cfg(unix)]
            Some("-") => "/dev/stdin".as_ref(),
            _ => path,
        };

        if self.allow_precompiled {
            unsafe { Module::from_trusted_file(engine, path) }
        } else {
            Module::from_file(engine, path)
                .context("if you're trying to run a precompiled module, pass --allow-precompiled")
        }
    }
}

#[derive(Default, Clone)]
struct Host {
    wasi: Option<wasmtime_wasi::WasiCtx>,
    #[cfg(feature = "wasi-nn")]
    wasi_nn: Option<Arc<WasiNnCtx>>,
    #[cfg(feature = "wasi-threads")]
    wasi_threads: Option<Arc<WasiThreadsCtx<Host>>>,
    #[cfg(feature = "wasi-http")]
    wasi_http: Option<WasiHttp>,
    limits: StoreLimits,
    guest_profiler: Option<Arc<GuestProfiler>>,
}

/// Populates the given `Linker` with WASI APIs.
fn populate_with_wasi(
    linker: &mut Linker<Host>,
    store: &mut Store<Host>,
    module: Module,
    preopen_dirs: Vec<(String, Dir)>,
    argv: &[String],
    vars: &[(String, Option<String>)],
    wasi_modules: &WasiModules,
    listenfd: bool,
    mut tcplisten: Vec<TcpListener>,
) -> Result<()> {
    if wasi_modules.wasi_common {
        wasmtime_wasi::add_to_linker(linker, |host| host.wasi.as_mut().unwrap())?;

        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdio().args(argv)?;

        for (key, value) in vars {
            let value = match value {
                Some(value) => value.clone(),
                None => std::env::var(key)
                    .map_err(|_| anyhow!("environment varialbe `{key}` not found"))?,
            };
            builder.env(key, &value)?;
        }

        let mut num_fd: usize = 3;

        if listenfd {
            num_fd = ctx_set_listenfd(num_fd, &mut builder)?;
        }

        for listener in tcplisten.drain(..) {
            builder.preopened_socket(num_fd as _, listener)?;
            num_fd += 1;
        }

        for (name, dir) in preopen_dirs.into_iter() {
            builder.preopened_dir(dir, name)?;
        }

        store.data_mut().wasi = Some(builder.build());
    }

    if wasi_modules.wasi_nn {
        #[cfg(not(feature = "wasi-nn"))]
        {
            bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
        }
        #[cfg(feature = "wasi-nn")]
        {
            wasmtime_wasi_nn::add_to_linker(linker, |host| {
                // This WASI proposal is currently not protected against
                // concurrent access--i.e., when wasi-threads is actively
                // spawning new threads, we cannot (yet) safely allow access and
                // fail if more than one thread has `Arc`-references to the
                // context. Once this proposal is updated (as wasi-common has
                // been) to allow concurrent access, this `Arc::get_mut`
                // limitation can be removed.
                Arc::get_mut(host.wasi_nn.as_mut().unwrap())
                    .expect("wasi-nn is not implemented with multi-threading support")
            })?;
            store.data_mut().wasi_nn = Some(Arc::new(WasiNnCtx::new()?));
        }
    }

    if wasi_modules.wasi_threads {
        #[cfg(not(feature = "wasi-threads"))]
        {
            // Silence the unused warning for `module` as it is only used in the
            // conditionally-compiled wasi-threads.
            drop(&module);

            bail!("Cannot enable wasi-threads when the binary is not compiled with this feature.");
        }
        #[cfg(feature = "wasi-threads")]
        {
            wasmtime_wasi_threads::add_to_linker(linker, store, &module, |host| {
                host.wasi_threads.as_ref().unwrap()
            })?;
            store.data_mut().wasi_threads = Some(Arc::new(WasiThreadsCtx::new(
                module,
                Arc::new(linker.clone()),
            )?));
        }
    }

    if wasi_modules.wasi_http {
        #[cfg(not(feature = "wasi-http"))]
        {
            bail!("Cannot enable wasi-http when the binary is not compiled with this feature.");
        }
        #[cfg(feature = "wasi-http")]
        {
            let w_http = WasiHttp::new();
            wasmtime_wasi_http::add_to_linker(linker, |host: &mut Host| {
                host.wasi_http.as_mut().unwrap()
            })?;
            store.data_mut().wasi_http = Some(w_http);
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn ctx_set_listenfd(num_fd: usize, _builder: &mut WasiCtxBuilder) -> Result<usize> {
    Ok(num_fd)
}

#[cfg(unix)]
fn ctx_set_listenfd(mut num_fd: usize, builder: &mut WasiCtxBuilder) -> Result<usize> {
    use listenfd::ListenFd;

    for env in ["LISTEN_FDS", "LISTEN_FDNAMES"] {
        if let Ok(val) = std::env::var(env) {
            builder.env(env, &val)?;
        }
    }

    let mut listenfd = ListenFd::from_env();

    for i in 0..listenfd.len() {
        if let Some(stdlistener) = listenfd.take_tcp_listener(i)? {
            let _ = stdlistener.set_nonblocking(true)?;
            let listener = TcpListener::from_std(stdlistener);
            builder.preopened_socket((3 + i) as _, listener)?;
            num_fd = 3 + i;
        }
    }

    Ok(num_fd)
}

fn generate_coredump(err: &anyhow::Error, source_name: &str, coredump_path: &str) -> Result<()> {
    let bt = err
        .downcast_ref::<wasmtime::WasmBacktrace>()
        .ok_or_else(|| anyhow!("no wasm backtrace found to generate coredump with"))?;

    let coredump = wasm_encoder::CoreDumpSection::new(source_name);
    let mut stacksection = wasm_encoder::CoreDumpStackSection::new("main");
    for f in bt.frames() {
        // We don't have the information at this point to map frames to
        // individual instances of a module, so we won't be able to create the
        // "frame ∈ instance ∈ module" hierarchy described in the core dump spec
        // until we move core dump generation into the runtime. So for now
        // instanceidx will be 0 for all frames
        let instanceidx = 0;
        stacksection.frame(
            instanceidx,
            f.func_index(),
            u32::try_from(f.func_offset().unwrap_or(0)).unwrap(),
            // We don't currently have access to locals/stack values
            [],
            [],
        );
    }
    let mut module = wasm_encoder::Module::new();
    module.section(&coredump);
    module.section(&stacksection);

    let mut f = File::create(coredump_path)
        .context(format!("failed to create file at `{}`", coredump_path))?;
    f.write_all(module.as_slice())
        .with_context(|| format!("failed to write coredump file at `{}`", coredump_path))?;
    Ok(())
}
