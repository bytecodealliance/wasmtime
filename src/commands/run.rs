//! The module that implements the `wasmtime run` command.

#![cfg_attr(
    not(feature = "component-model"),
    allow(irrefutable_let_patterns, unreachable_patterns)
)]

use crate::common::{Profile, RunCommon, RunTarget};
use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use wasi_common::sync::{Dir, TcpListener, WasiCtxBuilder, ambient_authority};
use wasmtime::{
    Engine, Error, Func, Module, Result, Store, StoreLimits, Val, ValType, bail,
    error::Context as _, format_err,
};
use wasmtime_wasi::{WasiCtxView, WasiView};

#[cfg(feature = "wasi-config")]
use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
#[cfg(feature = "wasi-http")]
use wasmtime_wasi_http::{
    DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS, DEFAULT_OUTGOING_BODY_CHUNK_SIZE, WasiHttpCtx,
};
#[cfg(feature = "wasi-keyvalue")]
use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};
#[cfg(feature = "wasi-nn")]
use wasmtime_wasi_nn::wit::WasiNnView;
#[cfg(feature = "wasi-threads")]
use wasmtime_wasi_threads::WasiThreadsCtx;
#[cfg(feature = "wasi-tls")]
use wasmtime_wasi_tls::{WasiTls, WasiTlsCtx};

fn parse_preloads(s: &str) -> Result<(String, PathBuf)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("must contain exactly one equals character ('=')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

/// Runs a WebAssembly module
#[derive(Parser)]
pub struct RunCommand {
    #[command(flatten)]
    #[expect(missing_docs, reason = "don't want to mess with clap doc-strings")]
    pub run: RunCommon,

    /// The name of the function to run
    #[arg(long, value_name = "FUNCTION")]
    pub invoke: Option<String>,

    #[command(flatten)]
    #[expect(missing_docs, reason = "don't want to mess with clap doc-strings")]
    pub preloads: Preloads,

    /// Override the value of `argv[0]`, typically the name of the executable of
    /// the application being run.
    ///
    /// This can be useful to pass in situations where a CLI tool is being
    /// executed that dispatches its functionality on the value of `argv[0]`
    /// without needing to rename the original wasm binary.
    #[arg(long)]
    pub argv0: Option<String>,

    /// The WebAssembly module to run and arguments to pass to it.
    ///
    /// Arguments passed to the wasm module will be configured as WASI CLI
    /// arguments unless the `--invoke` CLI argument is passed in which case
    /// arguments will be interpreted as arguments to the function specified.
    #[arg(value_name = "WASM", trailing_var_arg = true, required = true)]
    pub module_and_args: Vec<OsString>,
}

#[expect(missing_docs, reason = "don't want to mess with clap doc-strings")]
#[derive(Parser, Default, Clone)]
pub struct Preloads {
    /// Load the given WebAssembly module before the main module
    #[arg(
        long = "preload",
        number_of_values = 1,
        value_name = "NAME=MODULE_PATH",
        value_parser = parse_preloads,
    )]
    modules: Vec<(String, PathBuf)>,
}

/// Dispatch between either a core or component linker.
#[expect(missing_docs, reason = "self-explanatory")]
pub enum CliLinker {
    Core(wasmtime::Linker<Host>),
    #[cfg(feature = "component-model")]
    Component(wasmtime::component::Linker<Host>),
}

/// Dispatch between either a core or component instance.
#[expect(missing_docs, reason = "self-explanatory")]
pub enum CliInstance {
    Core(wasmtime::Instance),
    #[cfg(feature = "component-model")]
    Component(wasmtime::component::Instance),
}

impl RunCommand {
    /// Executes the command.
    #[cfg(feature = "run")]
    pub fn execute(mut self) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .build()?;

        runtime.block_on(async {
            self.run.common.init_logging()?;

            let engine = self.new_engine()?;
            let main = self
                .run
                .load_module(&engine, self.module_and_args[0].as_ref())?;
            let (mut store, mut linker) = self.new_store_and_linker(&engine, &main)?;

            self.instantiate_and_run(&engine, &mut linker, &main, &mut store)
                .await?;
            Ok(())
        })
    }

    /// Creates a new `Engine` with the configuration for this command.
    pub fn new_engine(&mut self) -> Result<Engine> {
        let mut config = self.run.common.config(None)?;
        config.async_support(true);

        if self.run.common.wasm.timeout.is_some() {
            config.epoch_interruption(true);
        }
        match self.run.profile {
            Some(Profile::Native(s)) => {
                config.profiler(s);
            }
            Some(Profile::Guest { .. }) => {
                // Further configured down below as well.
                config.epoch_interruption(true);
            }
            None => {}
        }

        Engine::new(&config)
    }

    /// Populatse a new `Store` and `CliLinker` with the configuration in this
    /// command.
    ///
    /// The `engine` provided is used to for the store/linker and the `main`
    /// provided is the module/component that is going to be run.
    pub fn new_store_and_linker(
        &mut self,
        engine: &Engine,
        main: &RunTarget,
    ) -> Result<(Store<Host>, CliLinker)> {
        // Validate coredump-on-trap argument
        if let Some(path) = &self.run.common.debug.coredump {
            if path.contains("%") {
                bail!("the coredump-on-trap path does not support patterns yet.")
            }
        }

        let mut linker = match &main {
            RunTarget::Core(_) => CliLinker::Core(wasmtime::Linker::new(&engine)),
            #[cfg(feature = "component-model")]
            RunTarget::Component(_) => {
                CliLinker::Component(wasmtime::component::Linker::new(&engine))
            }
        };
        if let Some(enable) = self.run.common.wasm.unknown_exports_allow {
            match &mut linker {
                CliLinker::Core(l) => {
                    l.allow_unknown_exports(enable);
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(_) => {
                    bail!("--allow-unknown-exports not supported with components");
                }
            }
        }

        let host = Host {
            #[cfg(feature = "wasi-http")]
            wasi_http_outgoing_body_buffer_chunks: self
                .run
                .common
                .wasi
                .http_outgoing_body_buffer_chunks,
            #[cfg(feature = "wasi-http")]
            wasi_http_outgoing_body_chunk_size: self.run.common.wasi.http_outgoing_body_chunk_size,
            ..Default::default()
        };

        let mut store = Store::new(&engine, host);
        self.populate_with_wasi(&mut linker, &mut store, &main)?;

        store.data_mut().limits = self.run.store_limits();
        store.limiter(|t| &mut t.limits);

        // If fuel has been configured, we want to add the configured
        // fuel amount to this store.
        if let Some(fuel) = self.run.common.wasm.fuel {
            store.set_fuel(fuel)?;
        }

        Ok((store, linker))
    }

    /// Executes the `main` after instantiating it within `store`.
    ///
    /// This applies all configuration within `self`, such as timeouts and
    /// profiling, and performs the execution. The resulting instance is
    /// returned.
    pub async fn instantiate_and_run(
        &self,
        engine: &Engine,
        linker: &mut CliLinker,
        main: &RunTarget,
        store: &mut Store<Host>,
    ) -> Result<CliInstance> {
        let dur = self
            .run
            .common
            .wasm
            .timeout
            .unwrap_or(std::time::Duration::MAX);
        let result = tokio::time::timeout(dur, async {
            let mut profiled_modules: Vec<(String, Module)> = Vec::new();
            if let RunTarget::Core(m) = &main {
                profiled_modules.push(("".to_string(), m.clone()));
            }

            // Load the preload wasm modules.
            for (name, path) in self.preloads.modules.iter() {
                // Read the wasm module binary either as `*.wat` or a raw binary
                let preload_target = self.run.load_module(&engine, path)?;
                let preload_module = match preload_target {
                    RunTarget::Core(m) => m,
                    #[cfg(feature = "component-model")]
                    RunTarget::Component(_) => {
                        bail!("components cannot be loaded with `--preload`")
                    }
                };
                profiled_modules.push((name.to_string(), preload_module.clone()));

                // Add the module's functions to the linker.
                match linker {
                    #[cfg(feature = "cranelift")]
                    CliLinker::Core(linker) => {
                        linker
                            .module_async(&mut *store, name, &preload_module)
                            .await
                            .context(format!(
                                "failed to process preload `{}` at `{}`",
                                name,
                                path.display()
                            ))?;
                    }
                    #[cfg(not(feature = "cranelift"))]
                    CliLinker::Core(_) => {
                        bail!("support for --preload disabled at compile time");
                    }
                    #[cfg(feature = "component-model")]
                    CliLinker::Component(_) => {
                        bail!("--preload cannot be used with components");
                    }
                }
            }

            self.load_main_module(store, linker, &main, profiled_modules)
                .await
                .with_context(|| {
                    format!(
                        "failed to run main module `{}`",
                        self.module_and_args[0].to_string_lossy()
                    )
                })
        })
        .await;

        // Load the main wasm module.
        let instance = match result.unwrap_or_else(|elapsed| {
            Err(wasmtime::Error::from(wasmtime::Trap::Interrupt))
                .with_context(|| format!("timed out after {elapsed}"))
        }) {
            Ok(instance) => instance,
            Err(e) => {
                // Exit the process if Wasmtime understands the error;
                // otherwise, fall back on Rust's default error printing/return
                // code.
                if store.data().legacy_p1_ctx.is_some() {
                    return Err(wasi_common::maybe_exit_on_error(e));
                } else if store.data().wasip1_ctx.is_some() {
                    if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                        std::process::exit(exit.0);
                    }
                }
                if e.is::<wasmtime::Trap>() {
                    eprintln!("Error: {e:?}");
                    cfg_if::cfg_if! {
                        if #[cfg(unix)] {
                            std::process::exit(rustix::process::EXIT_SIGNALED_SIGABRT);
                        } else if #[cfg(windows)] {
                            // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
                            std::process::exit(3);
                        }
                    }
                }
                return Err(e);
            }
        };

        Ok(instance)
    }

    fn compute_argv(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();

        for (i, arg) in self.module_and_args.iter().enumerate() {
            // For argv[0], which is the program name. Only include the base
            // name of the main wasm module, to avoid leaking path information.
            let arg = if i == 0 {
                match &self.argv0 {
                    Some(s) => s.as_ref(),
                    None => Path::new(arg).components().next_back().unwrap().as_os_str(),
                }
            } else {
                arg.as_ref()
            };
            result.push(
                arg.to_str()
                    .ok_or_else(|| format_err!("failed to convert {arg:?} to utf-8"))?
                    .to_string(),
            );
        }

        Ok(result)
    }

    fn setup_epoch_handler(
        &self,
        store: &mut Store<Host>,
        main_target: &RunTarget,
        profiled_modules: Vec<(String, Module)>,
    ) -> Result<Box<dyn FnOnce(&mut Store<Host>)>> {
        if let Some(Profile::Guest { path, interval }) = &self.run.profile {
            #[cfg(feature = "profiling")]
            return Ok(self.setup_guest_profiler(
                store,
                main_target,
                profiled_modules,
                path,
                *interval,
            )?);
            #[cfg(not(feature = "profiling"))]
            {
                let _ = (profiled_modules, path, interval, main_target);
                bail!("support for profiling disabled at compile time");
            }
        }

        if let Some(timeout) = self.run.common.wasm.timeout {
            store.set_epoch_deadline(1);
            let engine = store.engine().clone();
            thread::spawn(move || {
                thread::sleep(timeout);
                engine.increment_epoch();
            });
        }

        Ok(Box::new(|_store| {}))
    }

    #[cfg(feature = "profiling")]
    fn setup_guest_profiler(
        &self,
        store: &mut Store<Host>,
        main_target: &RunTarget,
        profiled_modules: Vec<(String, Module)>,
        path: &str,
        interval: std::time::Duration,
    ) -> Result<Box<dyn FnOnce(&mut Store<Host>)>> {
        use wasmtime::{AsContext, GuestProfiler, StoreContext, StoreContextMut, UpdateDeadline};

        let module_name = self.module_and_args[0].to_str().unwrap_or("<main module>");
        store.data_mut().guest_profiler = match main_target {
            RunTarget::Core(_m) => Some(Arc::new(GuestProfiler::new(
                store.engine(),
                module_name,
                interval,
                profiled_modules,
            )?)),
            RunTarget::Component(component) => Some(Arc::new(GuestProfiler::new_component(
                store.engine(),
                module_name,
                interval,
                component.clone(),
                profiled_modules,
            )?)),
        };

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

        store.call_hook(|store, kind| {
            sample(store, |profiler, store| profiler.call_hook(store, kind));
            Ok(())
        });

        if let Some(timeout) = self.run.common.wasm.timeout {
            let mut timeout = (timeout.as_secs_f64() / interval.as_secs_f64()).ceil() as u64;
            assert!(timeout > 0);
            store.epoch_deadline_callback(move |store| {
                sample(store, |profiler, store| {
                    profiler.sample(store, std::time::Duration::ZERO)
                });
                timeout -= 1;
                if timeout == 0 {
                    bail!("timeout exceeded");
                }
                Ok(UpdateDeadline::Continue(1))
            });
        } else {
            store.epoch_deadline_callback(move |store| {
                sample(store, |profiler, store| {
                    profiler.sample(store, std::time::Duration::ZERO)
                });
                Ok(UpdateDeadline::Continue(1))
            });
        }

        store.set_epoch_deadline(1);
        let engine = store.engine().clone();
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                engine.increment_epoch();
            }
        });

        let path = path.to_string();
        Ok(Box::new(move |store| {
            let profiler = Arc::try_unwrap(store.data_mut().guest_profiler.take().unwrap())
                .expect("profiling doesn't support threads yet");
            if let Err(e) = std::fs::File::create(&path)
                .map_err(wasmtime::Error::new)
                .and_then(|output| profiler.finish(std::io::BufWriter::new(output)))
            {
                eprintln!("failed writing profile at {path}: {e:#}");
            } else {
                eprintln!();
                eprintln!("Profile written to: {path}");
                eprintln!("View this profile at https://profiler.firefox.com/.");
            }
        }))
    }

    async fn load_main_module(
        &self,
        store: &mut Store<Host>,
        linker: &mut CliLinker,
        main_target: &RunTarget,
        profiled_modules: Vec<(String, Module)>,
    ) -> Result<CliInstance> {
        // The main module might be allowed to have unknown imports, which
        // should be defined as traps:
        if self.run.common.wasm.unknown_imports_trap == Some(true) {
            match linker {
                CliLinker::Core(linker) => {
                    linker.define_unknown_imports_as_traps(main_target.unwrap_core())?;
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(linker) => {
                    linker.define_unknown_imports_as_traps(main_target.unwrap_component())?;
                }
            }
        }

        // ...or as default values.
        if self.run.common.wasm.unknown_imports_default == Some(true) {
            match linker {
                CliLinker::Core(linker) => {
                    linker.define_unknown_imports_as_default_values(
                        &mut *store,
                        main_target.unwrap_core(),
                    )?;
                }
                _ => bail!("cannot use `--default-values-unknown-imports` with components"),
            }
        }

        let finish_epoch_handler =
            self.setup_epoch_handler(store, main_target, profiled_modules)?;

        let result = match linker {
            CliLinker::Core(linker) => {
                let module = main_target.unwrap_core();
                let instance = linker
                    .instantiate_async(&mut *store, &module)
                    .await
                    .context(format!(
                        "failed to instantiate {:?}",
                        self.module_and_args[0]
                    ))?;

                // If `_initialize` is present, meaning a reactor, then invoke
                // the function.
                if let Some(func) = instance.get_func(&mut *store, "_initialize") {
                    func.typed::<(), ()>(&store)?
                        .call_async(&mut *store, ())
                        .await?;
                }

                // Look for the specific function provided or otherwise look for
                // "" or "_start" exports to run as a "main" function.
                let func = if let Some(name) = &self.invoke {
                    Some(
                        instance
                            .get_func(&mut *store, name)
                            .ok_or_else(|| format_err!("no func export named `{name}` found"))?,
                    )
                } else {
                    instance
                        .get_func(&mut *store, "")
                        .or_else(|| instance.get_func(&mut *store, "_start"))
                };

                if let Some(func) = func {
                    self.invoke_func(store, func).await?;
                }
                Ok(CliInstance::Core(instance))
            }
            #[cfg(feature = "component-model")]
            CliLinker::Component(linker) => {
                let component = main_target.unwrap_component();
                let result = if self.invoke.is_some() {
                    self.invoke_component(&mut *store, component, linker).await
                } else {
                    self.run_command_component(&mut *store, component, linker)
                        .await
                };
                result
                    .map(CliInstance::Component)
                    .map_err(|e| self.handle_core_dump(&mut *store, e))
            }
        };
        finish_epoch_handler(store);

        result
    }

    #[cfg(feature = "component-model")]
    async fn invoke_component(
        &self,
        store: &mut Store<Host>,
        component: &wasmtime::component::Component,
        linker: &mut wasmtime::component::Linker<Host>,
    ) -> Result<wasmtime::component::Instance> {
        use wasmtime::component::{
            Val,
            wasm_wave::{
                untyped::UntypedFuncCall,
                wasm::{DisplayFuncResults, WasmFunc},
            },
        };

        // Check if the invoke string is present
        let invoke: &String = self.invoke.as_ref().unwrap();

        let untyped_call = UntypedFuncCall::parse(invoke).with_context(|| {
                format!(
                    "Failed to parse invoke '{invoke}': See https://docs.wasmtime.dev/cli-options.html#run for syntax",
                )
        })?;

        let name = untyped_call.name();
        let matches =
            Self::search_component_funcs(store.engine(), component.component_type(), name);
        let (names, func_type) = match matches.len() {
            0 => bail!("No exported func named `{name}` in component."),
            1 => &matches[0],
            _ => bail!(
                "Multiple exports named `{name}`: {matches:?}. FIXME: support some way to disambiguate names"
            ),
        };

        let param_types = WasmFunc::params(func_type).collect::<Vec<_>>();
        let params = untyped_call
            .to_wasm_params(&param_types)
            .with_context(|| format!("while interpreting parameters in invoke \"{invoke}\""))?;

        let export = names
            .iter()
            .fold(None, |instance, name| {
                component.get_export_index(instance.as_ref(), name)
            })
            .expect("export has at least one name");

        let instance = linker.instantiate_async(&mut *store, component).await?;

        let func = instance
            .get_func(&mut *store, export)
            .expect("found export index");

        let mut results = vec![Val::Bool(false); func_type.results().len()];
        self.call_component_func(store, &params, func, &mut results)
            .await?;

        println!("{}", DisplayFuncResults(&results));
        Ok(instance)
    }

    #[cfg(feature = "component-model")]
    async fn call_component_func(
        &self,
        store: &mut Store<Host>,
        params: &[wasmtime::component::Val],
        func: wasmtime::component::Func,
        results: &mut Vec<wasmtime::component::Val>,
    ) -> Result<(), Error> {
        #[cfg(feature = "component-model-async")]
        if self.run.common.wasm.component_model_async.unwrap_or(false) {
            store
                .run_concurrent(async |store| {
                    let task = func.call_concurrent(store, params, results).await?;
                    task.block(store).await;
                    wasmtime::error::Ok(())
                })
                .await??;
            return Ok(());
        }

        func.call_async(&mut *store, &params, results).await?;
        func.post_return_async(&mut *store).await?;
        Ok(())
    }

    /// Execute the default behavior for components on the CLI, looking for
    /// `wasi:cli`-based commands and running their exported `run` function.
    #[cfg(feature = "component-model")]
    async fn run_command_component(
        &self,
        store: &mut Store<Host>,
        component: &wasmtime::component::Component,
        linker: &wasmtime::component::Linker<Host>,
    ) -> Result<wasmtime::component::Instance> {
        let instance = linker.instantiate_async(&mut *store, component).await?;

        let mut result = None;
        let _ = &mut result;

        // If WASIp3 is enabled at compile time, enabled at runtime, and found
        // in this component then use that to generate the result.
        #[cfg(feature = "component-model-async")]
        if self.run.common.wasi.p3.unwrap_or(crate::common::P3_DEFAULT) {
            if let Ok(command) = wasmtime_wasi::p3::bindings::Command::new(&mut *store, &instance) {
                result = Some(
                    store
                        .run_concurrent(async |store| {
                            let (result, task) = command.wasi_cli_run().call_run(store).await?;
                            task.block(store).await;
                            Ok(result)
                        })
                        .await?,
                );
            }
        }

        let result = match result {
            Some(result) => result,
            // If WASIp3 wasn't found then fall back to requiring WASIp2 and
            // this'll report an error if the right export doesn't exist.
            None => {
                wasmtime_wasi::p2::bindings::Command::new(&mut *store, &instance)?
                    .wasi_cli_run()
                    .call_run(&mut *store)
                    .await
            }
        };
        let wasm_result = result.context("failed to invoke `run` function")?;

        // Translate the `Result<(),()>` produced by wasm into a feigned
        // explicit exit here with status 1 if `Err(())` is returned.
        match wasm_result {
            Ok(()) => Ok(instance),
            Err(()) => Err(wasmtime_wasi::I32Exit(1).into()),
        }
    }

    #[cfg(feature = "component-model")]
    fn search_component_funcs(
        engine: &Engine,
        component: wasmtime::component::types::Component,
        name: &str,
    ) -> Vec<(Vec<String>, wasmtime::component::types::ComponentFunc)> {
        use wasmtime::component::types::ComponentItem as CItem;
        fn collect_exports(
            engine: &Engine,
            item: CItem,
            basename: Vec<String>,
        ) -> Vec<(Vec<String>, CItem)> {
            match item {
                CItem::Component(c) => c
                    .exports(engine)
                    .flat_map(move |(name, item)| {
                        let mut names = basename.clone();
                        names.push(name.to_string());
                        collect_exports(engine, item, names)
                    })
                    .collect::<Vec<_>>(),
                CItem::ComponentInstance(c) => c
                    .exports(engine)
                    .flat_map(move |(name, item)| {
                        let mut names = basename.clone();
                        names.push(name.to_string());
                        collect_exports(engine, item, names)
                    })
                    .collect::<Vec<_>>(),
                _ => vec![(basename, item)],
            }
        }

        collect_exports(engine, CItem::Component(component), Vec::new())
            .into_iter()
            .filter_map(|(names, item)| {
                let CItem::ComponentFunc(func) = item else {
                    return None;
                };
                let func_name = names.last().expect("at least one name");
                (func_name == name).then_some((names, func))
            })
            .collect()
    }

    async fn invoke_func(&self, store: &mut Store<Host>, func: Func) -> Result<()> {
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
                        bail!("not enough arguments for `{name}`")
                    } else {
                        bail!("not enough arguments for command default")
                    }
                }
            };
            let val = val
                .to_str()
                .ok_or_else(|| format_err!("argument is not valid utf-8: {val:?}"))?;
            values.push(match ty {
                // Supports both decimal and hexadecimal notation (with 0x prefix)
                ValType::I32 => Val::I32(if val.starts_with("0x") || val.starts_with("0X") {
                    i32::from_str_radix(&val[2..], 16)?
                } else {
                    val.parse::<i32>()?
                }),
                ValType::I64 => Val::I64(if val.starts_with("0x") || val.starts_with("0X") {
                    i64::from_str_radix(&val[2..], 16)?
                } else {
                    val.parse::<i64>()?
                }),
                ValType::F32 => Val::F32(val.parse::<f32>()?.to_bits()),
                ValType::F64 => Val::F64(val.parse::<f64>()?.to_bits()),
                t => bail!("unsupported argument type {t:?}"),
            });
        }

        // Invoke the function and then afterwards print all the results that came
        // out, if there are any.
        let mut results = vec![Val::null_func_ref(); ty.results().len()];
        let invoke_res = func
            .call_async(&mut *store, &values, &mut results)
            .await
            .with_context(|| {
                if let Some(name) = &self.invoke {
                    format!("failed to invoke `{name}`")
                } else {
                    format!("failed to invoke command default")
                }
            });

        if let Err(err) = invoke_res {
            return Err(self.handle_core_dump(&mut *store, err));
        }

        if !results.is_empty() {
            eprintln!(
                "warning: using `--invoke` with a function that returns values \
                 is experimental and may break in the future"
            );
        }

        for result in results {
            match result {
                Val::I32(i) => println!("{i}"),
                Val::I64(i) => println!("{i}"),
                Val::F32(f) => println!("{}", f32::from_bits(f)),
                Val::F64(f) => println!("{}", f64::from_bits(f)),
                Val::V128(i) => println!("{}", i.as_u128()),
                Val::ExternRef(None) => println!("<null externref>"),
                Val::ExternRef(Some(_)) => println!("<externref>"),
                Val::FuncRef(None) => println!("<null funcref>"),
                Val::FuncRef(Some(_)) => println!("<funcref>"),
                Val::AnyRef(None) => println!("<null anyref>"),
                Val::AnyRef(Some(_)) => println!("<anyref>"),
                Val::ExnRef(None) => println!("<null exnref>"),
                Val::ExnRef(Some(_)) => println!("<exnref>"),
                Val::ContRef(None) => println!("<null contref>"),
                Val::ContRef(Some(_)) => println!("<contref>"),
            }
        }

        Ok(())
    }

    #[cfg(feature = "coredump")]
    fn handle_core_dump(&self, store: &mut Store<Host>, err: Error) -> Error {
        let coredump_path = match &self.run.common.debug.coredump {
            Some(path) => path,
            None => return err,
        };
        if !err.is::<wasmtime::Trap>() {
            return err;
        }
        let source_name = self.module_and_args[0]
            .to_str()
            .unwrap_or_else(|| "unknown");

        if let Err(coredump_err) = write_core_dump(store, &err, &source_name, coredump_path) {
            eprintln!("warning: coredump failed to generate: {coredump_err}");
            err
        } else {
            err.context(format!("core dumped at {coredump_path}"))
        }
    }

    #[cfg(not(feature = "coredump"))]
    fn handle_core_dump(&self, _store: &mut Store<Host>, err: Error) -> Error {
        err
    }

    /// Populates the given `Linker` with WASI APIs.
    fn populate_with_wasi(
        &self,
        linker: &mut CliLinker,
        store: &mut Store<Host>,
        module: &RunTarget,
    ) -> Result<()> {
        self.run.validate_p3_option()?;
        let cli = self.run.validate_cli_enabled()?;

        if cli != Some(false) {
            match linker {
                CliLinker::Core(linker) => {
                    match (self.run.common.wasi.preview2, self.run.common.wasi.threads) {
                        // If preview2 is explicitly disabled, or if threads
                        // are enabled, then use the historical preview1
                        // implementation.
                        (Some(false), _) | (None, Some(true)) => {
                            wasi_common::tokio::add_to_linker(linker, |host| {
                                host.legacy_p1_ctx.as_mut().unwrap()
                            })?;
                            self.set_legacy_p1_ctx(store)?;
                        }
                        // If preview2 was explicitly requested, always use it.
                        // Otherwise use it so long as threads are disabled.
                        //
                        // Note that for now `p0` is currently
                        // default-enabled but this may turn into
                        // default-disabled in the future.
                        (Some(true), _) | (None, Some(false) | None) => {
                            if self.run.common.wasi.preview0 != Some(false) {
                                wasmtime_wasi::p0::add_to_linker_async(linker, |t| t.wasip1_ctx())?;
                            }
                            wasmtime_wasi::p1::add_to_linker_async(linker, |t| t.wasip1_ctx())?;
                            self.set_wasi_ctx(store)?;
                        }
                    }
                }
                #[cfg(feature = "component-model")]
                CliLinker::Component(linker) => {
                    self.run.add_wasmtime_wasi_to_linker(linker)?;
                    self.set_wasi_ctx(store)?;
                }
            }
        }

        if self.run.common.wasi.nn == Some(true) {
            #[cfg(not(feature = "wasi-nn"))]
            {
                bail!("Cannot enable wasi-nn when the binary is not compiled with this feature.");
            }
            #[cfg(all(feature = "wasi-nn", feature = "component-model"))]
            {
                let (backends, registry) = self.collect_preloaded_nn_graphs()?;
                match linker {
                    CliLinker::Core(linker) => {
                        wasmtime_wasi_nn::witx::add_to_linker(linker, |host| {
                            Arc::get_mut(host.wasi_nn_witx.as_mut().unwrap())
                                .expect("wasi-nn is not implemented with multi-threading support")
                        })?;
                        store.data_mut().wasi_nn_witx = Some(Arc::new(
                            wasmtime_wasi_nn::witx::WasiNnCtx::new(backends, registry),
                        ));
                    }
                    #[cfg(feature = "component-model")]
                    CliLinker::Component(linker) => {
                        wasmtime_wasi_nn::wit::add_to_linker(linker, |h: &mut Host| {
                            let ctx = h.wasip1_ctx.as_mut().expect("wasi is not configured");
                            let ctx = Arc::get_mut(ctx)
                                .expect("wasmtime_wasi is not compatible with threads")
                                .get_mut()
                                .unwrap();
                            let nn_ctx = Arc::get_mut(h.wasi_nn_wit.as_mut().unwrap())
                                .expect("wasi-nn is not implemented with multi-threading support");
                            WasiNnView::new(ctx.ctx().table, nn_ctx)
                        })?;
                        store.data_mut().wasi_nn_wit = Some(Arc::new(
                            wasmtime_wasi_nn::wit::WasiNnCtx::new(backends, registry),
                        ));
                    }
                }
            }
        }

        if self.run.common.wasi.config == Some(true) {
            #[cfg(not(feature = "wasi-config"))]
            {
                bail!(
                    "Cannot enable wasi-config when the binary is not compiled with this feature."
                );
            }
            #[cfg(all(feature = "wasi-config", feature = "component-model"))]
            {
                match linker {
                    CliLinker::Core(_) => {
                        bail!("Cannot enable wasi-config for core wasm modules");
                    }
                    CliLinker::Component(linker) => {
                        let vars = WasiConfigVariables::from_iter(
                            self.run
                                .common
                                .wasi
                                .config_var
                                .iter()
                                .map(|v| (v.key.clone(), v.value.clone())),
                        );

                        wasmtime_wasi_config::add_to_linker(linker, |h| {
                            WasiConfig::new(Arc::get_mut(h.wasi_config.as_mut().unwrap()).unwrap())
                        })?;
                        store.data_mut().wasi_config = Some(Arc::new(vars));
                    }
                }
            }
        }

        if self.run.common.wasi.keyvalue == Some(true) {
            #[cfg(not(feature = "wasi-keyvalue"))]
            {
                bail!(
                    "Cannot enable wasi-keyvalue when the binary is not compiled with this feature."
                );
            }
            #[cfg(all(feature = "wasi-keyvalue", feature = "component-model"))]
            {
                match linker {
                    CliLinker::Core(_) => {
                        bail!("Cannot enable wasi-keyvalue for core wasm modules");
                    }
                    CliLinker::Component(linker) => {
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

                        wasmtime_wasi_keyvalue::add_to_linker(linker, |h| {
                            let ctx = h.wasip1_ctx.as_mut().expect("wasip2 is not configured");
                            let ctx = Arc::get_mut(ctx).unwrap().get_mut().unwrap();
                            WasiKeyValue::new(
                                Arc::get_mut(h.wasi_keyvalue.as_mut().unwrap()).unwrap(),
                                ctx.ctx().table,
                            )
                        })?;
                        store.data_mut().wasi_keyvalue = Some(Arc::new(ctx));
                    }
                }
            }
        }

        if self.run.common.wasi.threads == Some(true) {
            #[cfg(not(feature = "wasi-threads"))]
            {
                // Silence the unused warning for `module` as it is only used in the
                // conditionally-compiled wasi-threads.
                let _ = &module;

                bail!(
                    "Cannot enable wasi-threads when the binary is not compiled with this feature."
                );
            }
            #[cfg(feature = "wasi-threads")]
            {
                let linker = match linker {
                    CliLinker::Core(linker) => linker,
                    _ => bail!("wasi-threads does not support components yet"),
                };
                let module = module.unwrap_core();
                wasmtime_wasi_threads::add_to_linker(linker, store, &module, |host| {
                    host.wasi_threads.as_ref().unwrap()
                })?;
                store.data_mut().wasi_threads = Some(Arc::new(WasiThreadsCtx::new(
                    module.clone(),
                    Arc::new(linker.clone()),
                )?));
            }
        }

        if self.run.common.wasi.http == Some(true) {
            #[cfg(not(all(feature = "wasi-http", feature = "component-model")))]
            {
                bail!("Cannot enable wasi-http when the binary is not compiled with this feature.");
            }
            #[cfg(all(feature = "wasi-http", feature = "component-model"))]
            {
                match linker {
                    CliLinker::Core(_) => {
                        bail!("Cannot enable wasi-http for core wasm modules");
                    }
                    CliLinker::Component(linker) => {
                        wasmtime_wasi_http::add_only_http_to_linker_sync(linker)?;
                        #[cfg(feature = "component-model-async")]
                        if self.run.common.wasi.p3.unwrap_or(crate::common::P3_DEFAULT) {
                            wasmtime_wasi_http::p3::add_to_linker(linker)?;
                        }
                    }
                }

                store.data_mut().wasi_http = Some(Arc::new(WasiHttpCtx::new()));
            }
        }

        if self.run.common.wasi.tls == Some(true) {
            #[cfg(all(not(all(feature = "wasi-tls", feature = "component-model"))))]
            {
                bail!("Cannot enable wasi-tls when the binary is not compiled with this feature.");
            }
            #[cfg(all(feature = "wasi-tls", feature = "component-model",))]
            {
                match linker {
                    CliLinker::Core(_) => {
                        bail!("Cannot enable wasi-tls for core wasm modules");
                    }
                    CliLinker::Component(linker) => {
                        let mut opts = wasmtime_wasi_tls::LinkOptions::default();
                        opts.tls(true);
                        wasmtime_wasi_tls::add_to_linker(linker, &mut opts, |h| {
                            let ctx = h.wasip1_ctx.as_mut().expect("wasi is not configured");
                            let ctx = Arc::get_mut(ctx).unwrap().get_mut().unwrap();
                            WasiTls::new(
                                Arc::get_mut(h.wasi_tls.as_mut().unwrap()).unwrap(),
                                ctx.ctx().table,
                            )
                        })?;

                        let ctx = wasmtime_wasi_tls::WasiTlsCtxBuilder::new().build();
                        store.data_mut().wasi_tls = Some(Arc::new(ctx));
                    }
                }
            }
        }

        Ok(())
    }

    fn set_legacy_p1_ctx(&self, store: &mut Store<Host>) -> Result<()> {
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdio().args(&self.compute_argv()?)?;

        if self.run.common.wasi.inherit_env == Some(true) {
            for (k, v) in std::env::vars() {
                builder.env(&k, &v)?;
            }
        }
        for (key, value) in self.run.vars.iter() {
            let value = match value {
                Some(value) => value.clone(),
                None => match std::env::var_os(key) {
                    Some(val) => val
                        .into_string()
                        .map_err(|_| format_err!("environment variable `{key}` not valid utf-8"))?,
                    None => {
                        // leave the env var un-set in the guest
                        continue;
                    }
                },
            };
            builder.env(key, &value)?;
        }

        let mut num_fd: usize = 3;

        if self.run.common.wasi.listenfd == Some(true) {
            num_fd = ctx_set_listenfd(num_fd, &mut builder)?;
        }

        for listener in self.run.compute_preopen_sockets()? {
            let listener = TcpListener::from_std(listener);
            builder.preopened_socket(num_fd as _, listener)?;
            num_fd += 1;
        }

        for (host, guest) in self.run.dirs.iter() {
            let dir = Dir::open_ambient_dir(host, ambient_authority())
                .with_context(|| format!("failed to open directory '{host}'"))?;
            builder.preopened_dir(dir, guest)?;
        }

        store.data_mut().legacy_p1_ctx = Some(builder.build());
        Ok(())
    }

    /// Note the naming here is subtle, but this is effectively setting up a
    /// `wasmtime_wasi::WasiCtx` structure.
    ///
    /// This is stored in `Host` as `WasiP1Ctx` which internally contains the
    /// `WasiCtx` and `ResourceTable` used for WASI implementations. Exactly
    /// which "p" for WASIpN is more a reference to
    /// `wasmtime-wasi`-vs-`wasi-common` here more than anything else.
    fn set_wasi_ctx(&self, store: &mut Store<Host>) -> Result<()> {
        let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
        builder.inherit_stdio().args(&self.compute_argv()?);
        self.run.configure_wasip2(&mut builder)?;
        let ctx = builder.build_p1();
        store.data_mut().wasip1_ctx = Some(Arc::new(Mutex::new(ctx)));
        Ok(())
    }

    #[cfg(feature = "wasi-nn")]
    fn collect_preloaded_nn_graphs(
        &self,
    ) -> Result<(Vec<wasmtime_wasi_nn::Backend>, wasmtime_wasi_nn::Registry)> {
        let graphs = self
            .run
            .common
            .wasi
            .nn_graph
            .iter()
            .map(|g| (g.format.clone(), g.dir.clone()))
            .collect::<Vec<_>>();
        wasmtime_wasi_nn::preload(&graphs)
    }
}

/// The `T` in `Store<T>` for what the CLI is running.
///
/// This structures has a number of contexts used for various WASI proposals.
/// Note that all of them are optional meaning that they're `None` by default
/// and enabled with various CLI flags (some CLI flags are on-by-default). Note
/// additionally that this structure is `Clone` to implement the `wasi-threads`
/// proposal. Many WASI proposals are not compatible with `wasi-threads` so to
/// model this `Arc` and `Arc<Mutex<T>>` is used for many configurations. If a
/// WASI proposal is inherently threadsafe it's protected with just an `Arc` to
/// share its configuration across many threads.
///
/// If mutation is required then `Mutex` is used. Note though that the mutex is
/// not actually locked as access always goes through `Arc::get_mut` which
/// effectively asserts that there's only one thread. In short much of this is
/// not compatible with `wasi-threads`.
#[derive(Default, Clone)]
pub struct Host {
    // Legacy wasip1 context using `wasi_common`, not set unless opted-in-to
    // with the CLI.
    legacy_p1_ctx: Option<wasi_common::WasiCtx>,

    // Context for both WASIp1 and WASIp2 (and beyond) for the `wasmtime_wasi`
    // crate. This has both `wasmtime_wasi::WasiCtx` as well as a
    // `ResourceTable` internally to be used.
    //
    // The Mutex is only needed to satisfy the Sync constraint but we never
    // actually perform any locking on it as we use Mutex::get_mut for every
    // access.
    wasip1_ctx: Option<Arc<Mutex<wasmtime_wasi::p1::WasiP1Ctx>>>,

    #[cfg(feature = "wasi-nn")]
    wasi_nn_wit: Option<Arc<wasmtime_wasi_nn::wit::WasiNnCtx>>,
    #[cfg(feature = "wasi-nn")]
    wasi_nn_witx: Option<Arc<wasmtime_wasi_nn::witx::WasiNnCtx>>,

    #[cfg(feature = "wasi-threads")]
    wasi_threads: Option<Arc<WasiThreadsCtx<Host>>>,
    #[cfg(feature = "wasi-http")]
    wasi_http: Option<Arc<WasiHttpCtx>>,
    #[cfg(feature = "wasi-http")]
    wasi_http_outgoing_body_buffer_chunks: Option<usize>,
    #[cfg(feature = "wasi-http")]
    wasi_http_outgoing_body_chunk_size: Option<usize>,
    #[cfg(all(feature = "wasi-http", feature = "component-model-async"))]
    p3_http: crate::common::DefaultP3Ctx,
    limits: StoreLimits,
    #[cfg(feature = "profiling")]
    guest_profiler: Option<Arc<wasmtime::GuestProfiler>>,

    #[cfg(feature = "wasi-config")]
    wasi_config: Option<Arc<WasiConfigVariables>>,
    #[cfg(feature = "wasi-keyvalue")]
    wasi_keyvalue: Option<Arc<WasiKeyValueCtx>>,
    #[cfg(feature = "wasi-tls")]
    wasi_tls: Option<Arc<WasiTlsCtx>>,
}

impl Host {
    fn wasip1_ctx(&mut self) -> &mut wasmtime_wasi::p1::WasiP1Ctx {
        unwrap_singlethread_context(&mut self.wasip1_ctx)
    }
}

fn unwrap_singlethread_context<T>(ctx: &mut Option<Arc<Mutex<T>>>) -> &mut T {
    let ctx = ctx.as_mut().expect("context not configured");
    Arc::get_mut(ctx)
        .expect("context is not compatible with threads")
        .get_mut()
        .unwrap()
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiView::ctx(self.wasip1_ctx())
    }
}

#[cfg(feature = "wasi-http")]
impl wasmtime_wasi_http::types::WasiHttpView for Host {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        let ctx = self.wasi_http.as_mut().unwrap();
        Arc::get_mut(ctx).expect("wasmtime_wasi is not compatible with threads")
    }

    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        WasiView::ctx(self).table
    }

    fn outgoing_body_buffer_chunks(&mut self) -> usize {
        self.wasi_http_outgoing_body_buffer_chunks
            .unwrap_or_else(|| DEFAULT_OUTGOING_BODY_BUFFER_CHUNKS)
    }

    fn outgoing_body_chunk_size(&mut self) -> usize {
        self.wasi_http_outgoing_body_chunk_size
            .unwrap_or_else(|| DEFAULT_OUTGOING_BODY_CHUNK_SIZE)
    }
}

#[cfg(all(feature = "wasi-http", feature = "component-model-async"))]
impl wasmtime_wasi_http::p3::WasiHttpView for Host {
    fn http(&mut self) -> wasmtime_wasi_http::p3::WasiHttpCtxView<'_> {
        wasmtime_wasi_http::p3::WasiHttpCtxView {
            table: WasiView::ctx(unwrap_singlethread_context(&mut self.wasip1_ctx)).table,
            ctx: &mut self.p3_http,
        }
    }
}

fn ctx_set_listenfd(mut num_fd: usize, builder: &mut WasiCtxBuilder) -> Result<usize> {
    let _ = &mut num_fd;
    let _ = &mut *builder;

    #[cfg(all(unix, feature = "run"))]
    {
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
    }

    Ok(num_fd)
}

#[cfg(feature = "coredump")]
fn write_core_dump(
    store: &mut Store<Host>,
    err: &wasmtime::Error,
    name: &str,
    path: &str,
) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let core_dump = err
        .downcast_ref::<wasmtime::WasmCoreDump>()
        .expect("should have been configured to capture core dumps");

    let core_dump = core_dump.serialize(store, name);

    let mut core_dump_file =
        File::create(path).context(format!("failed to create file at `{path}`"))?;
    core_dump_file
        .write_all(&core_dump)
        .with_context(|| format!("failed to write core dump file at `{path}`"))?;
    Ok(())
}
