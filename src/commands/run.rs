//! The module that implements the `wasmtime run` command.

use crate::{init_file_per_thread_logger, CommonOptions};
use anyhow::{bail, Context as _, Result};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    path::{Component, Path, PathBuf},
    process,
};
use structopt::{clap::AppSettings, StructOpt};
use wasi_common::preopen_dir;
use wasmtime::{Engine, Instance, Module, Store, Trap, Val, ValType};
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi};

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
        value_name = "MODULE_PATH",
        parse(from_os_str)
    )]
    preloads: Vec<PathBuf>,

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

        let config = self.common.config()?;
        let engine = Engine::new(&config);
        let store = Store::new(&engine);

        // Make wasi available by default.
        let preopen_dirs = self.compute_preopen_dirs()?;
        let argv = self.compute_argv();

        let module_registry = ModuleRegistry::new(&store, &preopen_dirs, &argv, &self.vars)?;

        // Load the preload wasm modules.
        for preload in self.preloads.iter() {
            Self::instantiate_module(&store, &module_registry, preload)
                .with_context(|| format!("failed to process preload at `{}`", preload.display()))?;
        }

        // Load the main wasm module.
        match self
            .handle_module(&store, &module_registry)
            .with_context(|| format!("failed to run main module `{}`", self.module.display()))
        {
            Ok(()) => (),
            Err(e) => {
                // If the program exited because of a trap, return an error code
                // to the outside environment indicating a more severe problem
                // than a simple failure.
                if e.is::<Trap>() {
                    // Print the error message in the usual way.
                    eprintln!("Error: {:?}", e);

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

    fn instantiate_module(
        store: &Store,
        module_registry: &ModuleRegistry,
        path: &Path,
    ) -> Result<Instance> {
        // Read the wasm module binary either as `*.wat` or a raw binary
        let data = wat::parse_file(path)?;

        let module = Module::new(store, &data)?;

        // Resolve import using module_registry.
        let imports = module
            .imports()
            .map(|i| {
                let export = match i.module() {
                    "wasi_snapshot_preview1" => {
                        module_registry.wasi_snapshot_preview1.get_export(i.name())
                    }
                    "wasi_unstable" => module_registry.wasi_unstable.get_export(i.name()),
                    other => bail!("import module `{}` was not found", other),
                };
                match export {
                    Some(export) => Ok(export.clone().into()),
                    None => bail!(
                        "import `{}` was not found in module `{}`",
                        i.name(),
                        i.module()
                    ),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let instance = Instance::new(&module, &imports)
            .context(format!("failed to instantiate {:?}", path))?;

        Ok(instance)
    }

    fn handle_module(&self, store: &Store, module_registry: &ModuleRegistry) -> Result<()> {
        let instance = Self::instantiate_module(store, module_registry, &self.module)?;

        // If a function to invoke was given, invoke it.
        if let Some(name) = self.invoke.as_ref() {
            self.invoke_export(instance, name)?;
        } else if instance.exports().any(|export| export.name().is_empty()) {
            // Launch the default command export.
            self.invoke_export(instance, "")?;
        } else {
            // If the module doesn't have a default command export, launch the
            // _start function if one is present, as a compatibility measure.
            self.invoke_export(instance, "_start")?;
        }

        Ok(())
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
                Val::AnyRef(_) => println!("<anyref>"),
                Val::FuncRef(_) => println!("<anyref>"),
                Val::V128(i) => println!("{}", i),
            }
        }

        Ok(())
    }
}

struct ModuleRegistry {
    wasi_snapshot_preview1: Wasi,
    wasi_unstable: WasiSnapshot0,
}

impl ModuleRegistry {
    fn new(
        store: &Store,
        preopen_dirs: &[(String, File)],
        argv: &[String],
        vars: &[(String, String)],
    ) -> Result<ModuleRegistry> {
        let mut cx1 = wasi_common::WasiCtxBuilder::new();

        cx1.inherit_stdio().args(argv).envs(vars);

        for (name, file) in preopen_dirs {
            cx1.preopened_dir(file.try_clone()?, name);
        }

        let cx1 = cx1.build()?;

        let mut cx2 = wasi_common::old::snapshot_0::WasiCtxBuilder::new();

        cx2.inherit_stdio().args(argv).envs(vars);

        for (name, file) in preopen_dirs {
            cx2.preopened_dir(file.try_clone()?, name);
        }

        let cx2 = cx2.build()?;

        Ok(ModuleRegistry {
            wasi_snapshot_preview1: Wasi::new(store, cx1),
            wasi_unstable: WasiSnapshot0::new(store, cx2),
        })
    }
}
