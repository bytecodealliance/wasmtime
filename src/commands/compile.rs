//! The module that implements the `wasmtime compile` command.

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use wasmtime::{CodeBuilder, CodeHint, Engine};
use wasmtime_cli_flags::CommonOptions;

const AFTER_HELP: &str =
    "By default, no CPU features or presets will be enabled for the compilation.\n\
        \n\
        Usage examples:\n\
        \n\
        Compiling a WebAssembly module for the current platform:\n\
        \n  \
        wasmtime compile example.wasm
        \n\
        Specifying the output file:\n\
        \n  \
        wasmtime compile -o output.cwasm input.wasm\n\
        \n\
        Compiling for a specific platform (Linux) and CPU preset (Skylake):\n\
        \n  \
        wasmtime compile --target x86_64-unknown-linux -Ccranelift-skylake foo.wasm\n";

/// Compiles a WebAssembly module.
#[derive(Parser, PartialEq)]
#[command(
    version,
    after_help = AFTER_HELP,
)]
pub struct CompileCommand {
    #[command(flatten)]
    #[allow(missing_docs)]
    pub common: CommonOptions,

    /// The target triple; default is the host triple
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// The path of the output compiled module; defaults to `<MODULE>.cwasm`
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// The directory path to write clif files into, one clif file per wasm function.
    #[arg(long = "emit-clif", value_name = "PATH")]
    pub emit_clif: Option<PathBuf>,

    /// The path of the WebAssembly to compile
    #[arg(index = 1, value_name = "MODULE")]
    pub module: PathBuf,
}

impl CompileCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.common.init_logging()?;

        let mut config = self.common.config(self.target.as_deref(), None)?;

        if let Some(path) = self.emit_clif {
            if !path.exists() {
                std::fs::create_dir(&path)?;
            }

            if !path.is_dir() {
                bail!(
                    "the path passed for '--emit-clif' ({}) must be a directory",
                    path.display()
                );
            }

            config.emit_clif(&path);
        }

        let engine = Engine::new(&config)?;

        if self.module.file_name().is_none() {
            bail!(
                "'{}' is not a valid input module path",
                self.module.display()
            );
        }

        let mut code = CodeBuilder::new(&engine);
        code.wasm_binary_or_text_file(&self.module)?;

        let output = self.output.take().unwrap_or_else(|| {
            let mut output: PathBuf = self.module.file_name().unwrap().into();
            output.set_extension("cwasm");
            output
        });

        let output_bytes = match code.hint() {
            #[cfg(feature = "component-model")]
            Some(CodeHint::Component) => code.compile_component_serialized()?,
            #[cfg(not(feature = "component-model"))]
            Some(CodeHint::Component) => {
                bail!("component model support was disabled at compile time")
            }
            Some(CodeHint::Module) | None => code.compile_module_serialized()?,
        };
        fs::write(&output, output_bytes)
            .with_context(|| format!("failed to write output: {}", output.display()))?;

        Ok(())
    }
}

#[cfg(all(test, not(miri)))]
mod test {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use wasmtime::{Instance, Module, Store};

    #[test]
    fn test_successful_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all(
            "(module (func (export \"f\") (param i32) (result i32) local.get 0))".as_bytes(),
        )?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        let command = CompileCommand::try_parse_from(vec![
            "compile",
            "-Dlogging=n",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        command.execute()?;

        let engine = Engine::default();
        let contents = std::fs::read(output_path)?;
        let module = unsafe { Module::deserialize(&engine, contents)? };
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let f = instance.get_typed_func::<i32, i32>(&mut store, "f")?;
        assert_eq!(f.call(&mut store, 1234).unwrap(), 1234);

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_x64_flags_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all("(module)".as_bytes())?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        // Set all the x64 flags to make sure they work
        let command = CompileCommand::try_parse_from(vec![
            "compile",
            "-Dlogging=n",
            "-Ccranelift-has-sse3",
            "-Ccranelift-has-ssse3",
            "-Ccranelift-has-sse41",
            "-Ccranelift-has-sse42",
            "-Ccranelift-has-avx",
            "-Ccranelift-has-avx2",
            "-Ccranelift-has-fma",
            "-Ccranelift-has-avx512dq",
            "-Ccranelift-has-avx512vl",
            "-Ccranelift-has-avx512f",
            "-Ccranelift-has-popcnt",
            "-Ccranelift-has-bmi1",
            "-Ccranelift-has-bmi2",
            "-Ccranelift-has-lzcnt",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        command.execute()?;

        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_aarch64_flags_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all("(module)".as_bytes())?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        // Set all the aarch64 flags to make sure they work
        let command = CompileCommand::try_parse_from(vec![
            "compile",
            "-Dlogging=n",
            "-Ccranelift-has-lse",
            "-Ccranelift-has-pauth",
            "-Ccranelift-has-fp16",
            "-Ccranelift-sign-return-address",
            "-Ccranelift-sign-return-address-all",
            "-Ccranelift-sign-return-address-with-bkey",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        command.execute()?;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_unsupported_flags_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all("(module)".as_bytes())?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        // aarch64 flags should not be supported
        let command = CompileCommand::try_parse_from(vec![
            "compile",
            "-Dlogging=n",
            "-Ccranelift-has-lse",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        assert_eq!(
            command.execute().unwrap_err().to_string(),
            "No existing setting named 'has_lse'"
        );

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_x64_presets_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all("(module)".as_bytes())?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        for preset in &[
            "nehalem",
            "haswell",
            "broadwell",
            "skylake",
            "cannonlake",
            "icelake",
            "znver1",
        ] {
            let flag = format!("-Ccranelift-{preset}");
            let command = CompileCommand::try_parse_from(vec![
                "compile",
                "-Dlogging=n",
                flag.as_str(),
                "-o",
                output_path.to_str().unwrap(),
                input_path.to_str().unwrap(),
            ])?;

            command.execute()?;
        }

        Ok(())
    }
}
