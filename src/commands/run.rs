//! The module that implements the `wasmtime run` command.

use crate::{init_file_per_thread_logger, CommonOptions};
use anyhow::{bail, Context as _, Result};
use std::thread;
use std::time::Duration;
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    path::{Component, PathBuf},
    process,
};
use structopt::{clap::AppSettings, StructOpt};
use wasi_common::preopen_dir;
use wasmtime::{Engine, Instance, Linker, Module, Started, Store, Trap, Val, ValType};
use wasmtime_wasi::wasi_linker;

fn parse_module(s: &OsStr) -> Result<PathBuf, OsString> {
    // Do not accept wasmtime subcommand names as the module name
    match s.to_str() {
        Some("help") | Some("config") | Some("run") | Some("wasm2obj") | Some("wast") => {
            Err("module name cannot be the same as a subcommand".into())
        }
        _ => Ok(s.into()),
    }
}

fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
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

/// Runs a WebAssembly module
#[derive(StructOpt)]
#[structopt(name = "run", setting = AppSettings::TrailingVarArg)]
pub struct RunCommand {
    #[structopt(flatten)]
    common: CommonOptions,

    /// Grant access to the given host directory
    #[structopt(long = "dir", number_of_values = 1, value_name = "DIRECTORY")]
    dirs: Vec<String>,

    /// Pass an environment variable to the program
    #[structopt(long = "env", number_of_values = 1, value_name = "NAME=VAL", parse(try_from_str = parse_env_var))]
    vars: Vec<(String, String)>,

    /// The name of the function to run
    #[structopt(long, value_name = "FUNCTION")]
    invoke: Option<String>,

    /// Grant access to a guest directory mapped as a host directory
    #[structopt(long = "mapdir", number_of_values = 1, value_name = "GUEST_DIR::HOST_DIR", parse(try_from_str = parse_map_dirs))]
    map_dirs: Vec<(String, String)>,

    /// The path of the WebAssembly module to run
    #[structopt(
        index = 1,
        required = true,
        value_name = "WASM_MODULE",
        parse(try_from_os_str = parse_module),
    )]
    module: PathBuf,

    /// Load the given WebAssembly module before the main module
    #[structopt(
        long = "preload",
        number_of_values = 1,
        value_name = "NAME::MODULE_PATH",
        parse(try_from_str = parse_preloads)
    )]
    preloads: Vec<(String, PathBuf)>,

    /// Maximum execution time of wasm code before timing out (1, 2s, 100ms, etc)
    #[structopt(
        long = "wasm-timeout",
        value_name = "TIME",
        parse(try_from_str = parse_dur),
    )]
    wasm_timeout: Option<Duration>,

    // NOTE: this must come last for trailing varargs
    /// The arguments to pass to the module
    #[structopt(value_name = "ARGS")]
    module_args: Vec<String>,
}

impl RunCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        if self.common.log_to_files {
            let prefix = "wasmtime.dbg.";
            init_file_per_thread_logger(prefix);
        } else {
            pretty_env_logger::init();
        }

        let mut config = self.common.config()?;
        if self.wasm_timeout.is_some() {
            config.interruptable(true);
        }
        let engine = Engine::new(&config);
        let store = Store::new(&engine);

        // Make wasi available by default.
        let preopen_dirs = self.compute_preopen_dirs()?;
        let argv = self.compute_argv();

        let mut linker = wasi_linker(&store, &preopen_dirs, &argv, &self.vars)?;

        // Load the preload wasm modules.
        for (name, path) in self.preloads.iter() {
            // Read the wasm module binary either as `*.wat` or a raw binary
            let module = Module::from_file(linker.store(), path)?;
            let started = linker
                .instantiate(&module)
                .and_then(|new_instance| new_instance.start(&[]))
                .context(format!("failed to instantiate {:?}", path))?;

            // If it was a command, don't register it.
            let instance = match started {
                Started::Command(_) => continue,
                Started::Reactor(instance) => instance,
            };

            linker.instance(name, &instance).with_context(|| {
                format!(
                    "failed to process preload `{}` at `{}`",
                    name,
                    path.display()
                )
            })?;
        }

        // Load the main wasm module.
        match self
            .load_main_module(&mut linker)
            .with_context(|| format!("failed to run main module `{}`", self.module.display()))
        {
            Ok(()) => (),
            Err(e) => {
                // If the program exited because of a non-zero exit status, print
                // a message and exit.
                if let Some(trap) = e.downcast_ref::<Trap>() {
                    // Print the error message in the usual way.
                    eprintln!("Error: {:?}", e);

                    if let Some(status) = trap.i32_exit_status() {
                        // On Windows, exit status 3 indicates an abort (see below),
                        // so return 1 indicating a non-zero status to avoid ambiguity.
                        if cfg!(windows) && status >= 3 {
                            process::exit(1);
                        }
                        process::exit(status);
                    }

                    // If the program exited because of a trap, return an error code
                    // to the outside environment indicating a more severe problem
                    // than a simple failure.
                    if cfg!(unix) {
                        // On Unix, return the error code of an abort.
                        process::exit(128 + libc::SIGABRT);
                    } else if cfg!(windows) {
                        // On Windows, return 3.
                        // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
                        process::exit(3);
                    }
                }
                return Err(e);
            }
        }

        Ok(())
    }

    fn compute_preopen_dirs(&self) -> Result<Vec<(String, File)>> {
        let mut preopen_dirs = Vec::new();

        for dir in self.dirs.iter() {
            preopen_dirs.push((
                dir.clone(),
                preopen_dir(dir).with_context(|| format!("failed to open directory '{}'", dir))?,
            ));
        }

        for (guest, host) in self.map_dirs.iter() {
            preopen_dirs.push((
                guest.clone(),
                preopen_dir(host)
                    .with_context(|| format!("failed to open directory '{}'", host))?,
            ));
        }

        Ok(preopen_dirs)
    }

    fn compute_argv(&self) -> Vec<String> {
        let mut result = Vec::new();

        // Add argv[0], which is the program name. Only include the base name of the
        // main wasm module, to avoid leaking path information.
        result.push(
            self.module
                .components()
                .next_back()
                .map(Component::as_os_str)
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_owned(),
        );

        // Add the remaining arguments.
        for arg in self.module_args.iter() {
            result.push(arg.clone());
        }

        result
    }

    fn load_main_module(&self, linker: &mut Linker) -> Result<()> {
        if let Some(timeout) = self.wasm_timeout {
            let handle = linker.store().interrupt_handle()?;
            thread::spawn(move || {
                thread::sleep(timeout);
                handle.interrupt();
            });
        }

        // Read the wasm module binary either as `*.wat` or a raw binary
        let module = Module::from_file(linker.store(), &self.module)?;
        let started = linker
            .instantiate(&module)
            .and_then(|new_instance| new_instance.start(&[]))
            .context(format!("failed to instantiate {:?}", self.module))?;

        // If a function to invoke was given, invoke it.
        if let Some(name) = self.invoke.as_ref() {
            match started {
                Started::Command(_) => {
                    bail!("Cannot invoke exports on a command after it has executed")
                }
                Started::Reactor(instance) => self.invoke_export(instance, name),
            }
        } else {
            Ok(())
        }
    }

    fn invoke_export(&self, instance: Instance, name: &str) -> Result<()> {
        let func = if let Some(export) = instance.get_export(name) {
            if let Some(func) = export.into_func() {
                func
            } else {
                bail!("export of `{}` wasn't a function", name)
            }
        } else {
            bail!("failed to find export of `{}` in module", name)
        };
        let ty = func.ty();
        if ty.params().len() > 0 {
            eprintln!(
                "warning: using `--invoke` with a function that takes arguments \
                 is experimental and may break in the future"
            );
        }
        let mut args = self.module_args.iter();
        let mut values = Vec::new();
        for ty in ty.params() {
            let val = match args.next() {
                Some(s) => s,
                None => bail!("not enough arguments for `{}`", name),
            };
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
        let results = func
            .call(&values)
            .with_context(|| format!("failed to invoke `{}`", name))?;
        if !results.is_empty() {
            eprintln!(
                "warning: using `--invoke` with a function that returns values \
                 is experimental and may break in the future"
            );
        }

        for result in results.into_vec() {
            match result {
                Val::I32(i) => println!("{}", i),
                Val::I64(i) => println!("{}", i),
                Val::F32(f) => println!("{}", f),
                Val::F64(f) => println!("{}", f),
                Val::ExternRef(_) => println!("<externref>"),
                Val::FuncRef(_) => println!("<externref>"),
                Val::V128(i) => println!("{}", i),
            }
        }

        Ok(())
    }
}
