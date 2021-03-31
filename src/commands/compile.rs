//! The module that implements the `wasmtime wast` command.

use crate::CommonOptions;
use anyhow::{anyhow, bail, Context, Result};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use structopt::{
    clap::{AppSettings, ArgGroup},
    StructOpt,
};
use target_lexicon::Triple;
use wasmtime::{Config, Engine, Module};

/// Compiles a WebAssembly module.
#[derive(StructOpt)]
#[structopt(
    name = "compile",
    version = env!("CARGO_PKG_VERSION"),
    setting = AppSettings::ColoredHelp,
    group = ArgGroup::with_name("x64").multiple(true),
    group = ArgGroup::with_name("preset-x64"),
    group = ArgGroup::with_name("aarch64").multiple(true).conflicts_with_all(&["x64", "preset-x64"]),
    group = ArgGroup::with_name("preset-aarch64").conflicts_with_all(&["x64", "preset-x64"]),
    after_help = "By default, no CPU flags will be enabled for the compilation.\n\
                  \n\
                  Use the various preset and CPU flag options for the environment being targeted.\n\
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
                  wasmtime compile --target x86_64-unknown-linux --skylake foo.wasm\n"
)]
pub struct CompileCommand {
    #[structopt(flatten)]
    common: CommonOptions,

    /// Enable support for interrupting WebAssembly code.
    #[structopt(long)]
    interruptable: bool,

    /// Enable SSE3 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    sse3: bool,

    /// Enable SSSE3 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    ssse3: bool,

    /// Enable SSE41 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    sse41: bool,

    /// Enable SSE42 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    sse42: bool,

    /// Enable AVX support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    avx: bool,

    /// Enable AVX2 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    avx2: bool,

    /// Enable AVX512DQ support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    avx512dq: bool,

    /// Enable AVX512VL support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    avx512vl: bool,

    /// Enable AVX512F support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    avx512f: bool,

    /// Enable POPCNT support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    popcnt: bool,

    /// Enable BMI1 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    bmi1: bool,

    /// Enable BMI2 support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    bmi2: bool,

    /// Enable LZCNT support (for x86-64 targets).
    #[structopt(long, group = "x64")]
    lzcnt: bool,

    /// Enable LSE support (for aarch64 targets).
    #[structopt(long, group = "aarch64")]
    lse: bool,

    /// Enable Nehalem preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    nehalem: bool,

    /// Enable Haswell preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    haswell: bool,

    /// Enable Broadwell preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    broadwell: bool,

    /// Enable Skylake preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    skylake: bool,

    /// Enable Cannonlake preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    cannonlake: bool,

    /// Enable Icelake preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    icelake: bool,

    /// Enable Zen preset (for x86-64 targets).
    #[structopt(long, group = "x64", group = "preset-x64")]
    znver1: bool,

    /// The target triple; default is the host triple
    #[structopt(long, value_name = "TARGET")]
    target: Option<String>,

    /// The path of the output compiled module; defaults to <MODULE>.cwasm
    #[structopt(short = "o", long, value_name = "OUTPUT", parse(from_os_str))]
    output: Option<PathBuf>,

    /// The path of the WebAssembly to compile
    #[structopt(index = 1, value_name = "MODULE", parse(from_os_str))]
    module: PathBuf,
}

impl CompileCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.common.init_logging();

        let target = self
            .target
            .take()
            .unwrap_or_else(|| Triple::host().to_string());

        let mut config = self.common.config(Some(&target))?;
        config.interruptable(self.interruptable);

        self.set_flags(&mut config, &target)?;

        let engine = Engine::new(&config)?;

        if self.module.file_name().is_none() {
            bail!(
                "'{}' is not a valid input module path",
                self.module.display()
            );
        }

        let input = fs::read(&self.module).with_context(|| "failed to read input file")?;

        let output = self.output.take().unwrap_or_else(|| {
            let mut output: PathBuf = self.module.file_name().unwrap().into();
            output.set_extension("cwasm");
            output
        });

        let mut writer = BufWriter::new(File::create(&output)?);
        Module::compile(&engine, &input, &mut writer)?;

        Ok(())
    }

    fn set_flags(&self, c: &mut Config, target: &str) -> Result<()> {
        use std::str::FromStr;

        macro_rules! set_flag {
            ($config:expr, $arch:expr, $flag:expr, $name:literal, $display:literal) => {
                if $flag {
                    unsafe {
                        $config.cranelift_flag_enable($name).map_err(|_| {
                            anyhow!("{} is not supported for architecture '{}'", $display, $arch)
                        })?;
                    }
                }
            };
        }

        let arch = Triple::from_str(target).unwrap().architecture;

        set_flag!(c, arch, self.sse3, "has_sse3", "SSE3");
        set_flag!(c, arch, self.ssse3, "has_ssse3", "SSSE3");
        set_flag!(c, arch, self.sse41, "has_sse41", "SSE41");
        set_flag!(c, arch, self.sse42, "has_sse42", "SSE42");
        set_flag!(c, arch, self.avx, "has_avx", "AVX");
        set_flag!(c, arch, self.avx2, "has_avx2", "AVX2");
        set_flag!(c, arch, self.avx512dq, "has_avx512dq", "AVX512DQ");
        set_flag!(c, arch, self.avx512vl, "has_avx512vl", "AVX512VL");
        set_flag!(c, arch, self.avx512f, "has_avx512f", "AVX512F");
        set_flag!(c, arch, self.popcnt, "has_popcnt", "POPCNT");
        set_flag!(c, arch, self.bmi1, "has_bmi1", "BMI1");
        set_flag!(c, arch, self.bmi2, "has_bmi2", "BMI2");
        set_flag!(c, arch, self.lzcnt, "has_lzcnt", "LZCNT");
        set_flag!(c, arch, self.lse, "has_lse", "LSE");
        set_flag!(c, arch, self.nehalem, "nehalem", "Nehalem preset");
        set_flag!(c, arch, self.haswell, "haswell", "Haswell preset");
        set_flag!(c, arch, self.broadwell, "broadwell", "Broadwell preset");
        set_flag!(c, arch, self.skylake, "skylake", "Skylake preset");
        set_flag!(c, arch, self.cannonlake, "cannonlake", "Cannonlake preset");
        set_flag!(c, arch, self.icelake, "icelake", "Icelake preset");
        set_flag!(c, arch, self.znver1, "znver1", "Zen preset");

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use wasmtime::{Instance, Store};

    #[test]
    fn test_successful_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all(
            "(module (func (export \"f\") (param i32) (result i32) local.get 0))".as_bytes(),
        )?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        let command = CompileCommand::from_iter_safe(vec![
            "compile",
            "--disable-logging",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        command.execute()?;

        let engine = Engine::default();
        let module = Module::from_file(&engine, output_path)?;
        let store = Store::new(&engine);
        let instance = Instance::new(&store, &module, &[])?;
        let f = instance.get_typed_func::<i32, i32>("f")?;
        assert_eq!(f.call(1234).unwrap(), 1234);

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
        let command = CompileCommand::from_iter_safe(vec![
            "compile",
            "--disable-logging",
            "--sse3",
            "--ssse3",
            "--sse41",
            "--sse42",
            "--avx",
            "--avx2",
            "--avx512dq",
            "--avx512vl",
            "--avx512f",
            "--popcnt",
            "--bmi1",
            "--bmi2",
            "--lzcnt",
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
        let command = CompileCommand::from_iter_safe(vec![
            "compile",
            "--disable-logging",
            "--lse",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ])?;

        command.execute()?;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_incompatible_flags_compile() -> Result<()> {
        let (mut input, input_path) = NamedTempFile::new()?.into_parts();
        input.write_all("(module)".as_bytes())?;
        drop(input);

        let output_path = NamedTempFile::new()?.into_temp_path();

        // x64 and aarch64 flags should conflict
        match CompileCommand::from_iter_safe(vec![
            "compile",
            "--disable-logging",
            "--sse3",
            "--lse",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ]) {
            Ok(_) => unreachable!(),
            Err(e) => {
                assert!(e
                    .to_string()
                    .contains("cannot be used with one or more of the other specified arguments"));
            }
        }

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
            "--nehalem",
            "--haswell",
            "--broadwell",
            "--skylake",
            "--cannonlake",
            "--icelake",
            "--znver1",
        ] {
            let command = CompileCommand::from_iter_safe(vec![
                "compile",
                "--disable-logging",
                preset,
                "-o",
                output_path.to_str().unwrap(),
                input_path.to_str().unwrap(),
            ])?;

            command.execute()?;
        }

        // Two presets should conflict
        match CompileCommand::from_iter_safe(vec![
            "compile",
            "--disable-logging",
            "--broadwell",
            "--cannonlake",
            "-o",
            output_path.to_str().unwrap(),
            input_path.to_str().unwrap(),
        ]) {
            Ok(_) => unreachable!(),
            Err(e) => {
                assert!(e
                    .to_string()
                    .contains("cannot be used with one or more of the other specified arguments"));
            }
        }

        Ok(())
    }
}
