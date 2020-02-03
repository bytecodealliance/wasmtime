//! The module that implements the `wasmtime wasm2obj` command.

use crate::{init_file_per_thread_logger, pick_compilation_strategy, CommonOptions};
use anyhow::{anyhow, bail, Context as _, Result};
use faerie::Artifact;
use std::{
    fmt::Write,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};
use structopt::{clap::AppSettings, StructOpt};
use target_lexicon::Triple;
use wasmtime::Strategy;
use wasmtime_debug::{emit_debugsections, read_debuginfo};
#[cfg(feature = "lightbeam")]
use wasmtime_environ::Lightbeam;
use wasmtime_environ::{
    cache_init, entity::EntityRef, settings, settings::Configurable, wasm::DefinedMemoryIndex,
    wasm::MemoryIndex, Compiler, Cranelift, ModuleEnvironment, ModuleMemoryOffset, ModuleVmctxInfo,
    Tunables, VMOffsets,
};
use wasmtime_jit::native;
use wasmtime_obj::emit_module;

/// The after help text for the `wasm2obj` command.
pub const WASM2OBJ_AFTER_HELP: &str = "The translation is dependent on the environment chosen.\n\
     The default is a dummy environment that produces placeholder values.";

fn parse_target(s: &str) -> Result<Triple> {
    Triple::from_str(&s).map_err(|e| anyhow!(e))
}

/// Translates a WebAssembly module to native object file
#[derive(StructOpt)]
#[structopt(
    name = "wasm2obj",
    version = env!("CARGO_PKG_VERSION"),
    setting = AppSettings::ColoredHelp,
    after_help = WASM2OBJ_AFTER_HELP,
)]
pub struct WasmToObjCommand {
    #[structopt(flatten)]
    common: CommonOptions,

    /// The path of the WebAssembly module to translate
    #[structopt(index = 1, value_name = "MODULE_PATH", parse(from_os_str))]
    module: PathBuf,

    /// The path of the output object file
    #[structopt(index = 2, value_name = "OUTPUT_PATH")]
    output: String,

    /// The target triple; default is the host triple
    #[structopt(long, value_name = "TARGET", parse(try_from_str = parse_target))]
    target: Option<Triple>,
}

impl WasmToObjCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        let log_config = if self.common.debug {
            pretty_env_logger::init();
            None
        } else {
            let prefix = "wasm2obj.dbg.";
            init_file_per_thread_logger(prefix);
            Some(prefix)
        };

        let errors = cache_init(
            !self.common.disable_cache,
            self.common.config.as_ref(),
            log_config,
        );

        if !errors.is_empty() {
            let mut message = String::new();
            writeln!(message, "Cache initialization failed. Errors:")?;
            for e in errors {
                writeln!(message, "  -> {}", e)?;
            }
            bail!(message);
        }

        self.handle_module()
    }

    fn handle_module(&self) -> Result<()> {
        let strategy = pick_compilation_strategy(self.common.cranelift, self.common.lightbeam)?;

        let data = wat::parse_file(&self.module).context("failed to parse module")?;

        let isa_builder = match self.target.as_ref() {
            Some(target) => native::lookup(target.clone())?,
            None => native::builder(),
        };
        let mut flag_builder = settings::builder();

        // There are two possible traps for division, and this way
        // we get the proper one if code traps.
        flag_builder.enable("avoid_div_traps").unwrap();

        if self.common.enable_simd {
            flag_builder.enable("enable_simd").unwrap();
        }

        if self.common.optimize {
            flag_builder.set("opt_level", "speed").unwrap();
        }

        let isa = isa_builder.finish(settings::Flags::new(flag_builder));

        let mut obj = Artifact::new(isa.triple().clone(), self.output.clone());

        // TODO: Expose the tunables as command-line flags.
        let tunables = Tunables::default();

        let (
            module,
            module_translation,
            lazy_function_body_inputs,
            lazy_data_initializers,
            target_config,
        ) = {
            let environ = ModuleEnvironment::new(isa.frontend_config(), tunables);

            let translation = environ
                .translate(&data)
                .context("failed to translate module")?;

            (
                translation.module,
                translation.module_translation.unwrap(),
                translation.function_body_inputs,
                translation.data_initializers,
                translation.target_config,
            )
        };

        // TODO: use the traps information
        let (compilation, relocations, address_transform, value_ranges, stack_slots, _traps) =
            match strategy {
                Strategy::Auto | Strategy::Cranelift => Cranelift::compile_module(
                    &module,
                    &module_translation,
                    lazy_function_body_inputs,
                    &*isa,
                    self.common.debug_info,
                ),
                #[cfg(feature = "lightbeam")]
                Strategy::Lightbeam => Lightbeam::compile_module(
                    &module,
                    &module_translation,
                    lazy_function_body_inputs,
                    &*isa,
                    self.common.debug_info,
                ),
                #[cfg(not(feature = "lightbeam"))]
                Strategy::Lightbeam => bail!("lightbeam support not enabled"),
                other => bail!("unsupported compilation strategy {:?}", other),
            }
            .context("failed to compile module")?;

        if compilation.is_empty() {
            bail!("no functions were found/compiled");
        }

        let module_vmctx_info = {
            let ofs = VMOffsets::new(target_config.pointer_bytes(), &module);
            ModuleVmctxInfo {
                memory_offset: if ofs.num_imported_memories > 0 {
                    ModuleMemoryOffset::Imported(ofs.vmctx_vmmemory_import(MemoryIndex::new(0)))
                } else if ofs.num_defined_memories > 0 {
                    ModuleMemoryOffset::Defined(
                        ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)),
                    )
                } else {
                    ModuleMemoryOffset::None
                },
                stack_slots,
            }
        };

        emit_module(
            &mut obj,
            &module,
            &compilation,
            &relocations,
            &lazy_data_initializers,
            &target_config,
        )
        .map_err(|e| anyhow!(e))
        .context("failed to emit module")?;

        if self.common.debug_info {
            let debug_data = read_debuginfo(&data);
            emit_debugsections(
                &mut obj,
                &module_vmctx_info,
                target_config,
                &debug_data,
                &address_transform,
                &value_ranges,
            )
            .context("failed to emit debug sections")?;
        }

        // FIXME: Make the format a parameter.
        let file = File::create(Path::new(&self.output)).context("failed to create object file")?;
        obj.write(file).context("failed to write object file")?;

        Ok(())
    }
}
