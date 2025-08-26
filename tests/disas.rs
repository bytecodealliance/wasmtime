//! A filetest-lookalike test suite using Cranelift tooling but built on
//! Wasmtime's code generator.
//!
//! This test will read the `tests/disas/*` directory and interpret all files in
//! that directory as a test. Each test must be in the wasm text format and
//! start with directives that look like:
//!
//! ```wasm
//! ;;! target = "x86_64"
//! ;;! compile = true
//!
//! (module
//!     ;; ...
//! )
//! ```
//!
//! Tests must configure a `target` and then can optionally specify a kind of
//! test:
//!
//! * No specifier - the output CLIF from translation is inspected.
//! * `optimize = true` - CLIF is emitted, then optimized, then inspected.
//! * `compile = true` - backends are run to produce machine code and that's inspected.
//!
//! Tests may also have a `flags` directive which are CLI flags to Wasmtime
//! itself:
//!
//! ```wasm
//! ;;! target = "x86_64"
//! ;;! flags = "-O opt-level=s"
//!
//! (module
//!     ;; ...
//! )
//! ```
//!
//! Flags are parsed by the `wasmtime_cli_flags` crate to build a `Config`.
//!
//! Configuration of tests is prefixed with `;;!` comments and must be present
//! at the start of the file. These comments are then parsed as TOML and
//! deserialized into `TestConfig` in this crate.

use anyhow::{Context, Result, bail};
use clap::Parser;
use cranelift_codegen::ir::Function;
use libtest_mimic::{Arguments, Trial};
use serde_derive::Deserialize;
use similar::TextDiff;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tempfile::TempDir;
use wasmtime::{Engine, OptLevel, Strategy};
use wasmtime_cli_flags::CommonOptions;

fn main() -> Result<()> {
    if cfg!(miri) || cfg!(asan) {
        return Ok(());
    }

    // There's not a ton of use in emulating these tests on other architectures
    // since they only exercise architecture-independent code of compiling to
    // multiple architectures. Additionally CI seems to occasionally deadlock or
    // get stuck in these tests when using QEMU, and it's not entirely clear
    // why. Finally QEMU-emulating these tests is relatively slow and without
    // much benefit from emulation it's hard to justify this. In the end disable
    // this test suite when QEMU is enabled.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }

    let _ = env_logger::try_init();

    let mut tests = Vec::new();
    find_tests("./tests/disas".as_ref(), &mut tests)?;

    let mut trials = Vec::new();
    for test in tests {
        trials.push(Trial::test(test.to_str().unwrap().to_string(), move || {
            run_test(&test)
                .with_context(|| format!("failed to run tests {test:?}"))
                .map_err(|e| format!("{e:?}").into())
        }))
    }

    // These tests have some long names so use the "quiet" output by default.
    let mut arguments = Arguments::parse();
    if arguments.format.is_none() {
        arguments.quiet = true;
    }
    libtest_mimic::run(&arguments, trials).exit()
}

fn find_tests(path: &Path, dst: &mut Vec<PathBuf>) -> Result<()> {
    for file in path
        .read_dir()
        .with_context(|| format!("failed to read {path:?}"))?
    {
        let file = file.context("failed to read directory entry")?;
        let path = file.path();
        if file.file_type()?.is_dir() {
            find_tests(&path, dst)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("wat") {
            dst.push(path);
        }
    }
    Ok(())
}

fn run_test(path: &Path) -> Result<()> {
    let mut test = Test::new(path)?;
    let output = test.compile()?;

    assert_output(&test, output)?;

    Ok(())
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestConfig {
    target: String,
    #[serde(default)]
    test: TestKind,
    flags: Option<TestConfigFlags>,
    objdump: Option<TestConfigFlags>,
    filter: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TestConfigFlags {
    SpaceSeparated(String),
    List(Vec<String>),
}

impl TestConfigFlags {
    fn to_vec(&self) -> Vec<&str> {
        match self {
            TestConfigFlags::SpaceSeparated(s) => s.split_whitespace().collect(),
            TestConfigFlags::List(s) => s.iter().map(|s| s.as_str()).collect(),
        }
    }
}

struct Test {
    path: PathBuf,
    contents: String,
    opts: CommonOptions,
    config: TestConfig,
}

/// Which kind of test is being performed.
#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TestKind {
    /// Test the CLIF output, raw from translation.
    #[default]
    Clif,
    /// Compile output to machine code.
    Compile,
    /// Test the CLIF output, optimized.
    Optimize,
    /// Alias for "compile" plus `-C compiler=winch`
    Winch,
}

impl Test {
    /// Parse the contents of `path` looking for directive-based comments
    /// starting with `;;!` near the top of the file.
    fn new(path: &Path) -> Result<Test> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("failed to read {path:?}"))?;
        let config: TestConfig = wasmtime_test_util::wast::parse_test_config(&contents, ";;!")
            .context("failed to parse test configuration as TOML")?;
        let mut flags = vec!["wasmtime"];
        if let Some(config) = &config.flags {
            flags.extend(config.to_vec());
        }
        let mut opts = wasmtime_cli_flags::CommonOptions::try_parse_from(&flags)?;
        opts.codegen.cranelift_debug_verifier = Some(true);

        Ok(Test {
            path: path.to_path_buf(),
            config,
            opts,
            contents,
        })
    }

    /// Generates CLIF for all the wasm functions in this test.
    fn compile(&mut self) -> Result<CompileOutput> {
        // Use wasmtime::Config with its `emit_clif` option to get Wasmtime's
        // code generator to jettison CLIF out the back.
        let tempdir = TempDir::new().context("failed to make a tempdir")?;
        let mut config = self.opts.config(None)?;
        config.target(&self.config.target)?;
        match self.config.test {
            TestKind::Clif => {
                config.emit_clif(tempdir.path());
                config.cranelift_opt_level(OptLevel::None);
            }
            TestKind::Optimize => {
                config.emit_clif(tempdir.path());
            }
            TestKind::Compile => {}
            TestKind::Winch => {
                config.strategy(Strategy::Winch);
            }
        }
        let engine = Engine::new(&config).context("failed to create engine")?;
        let wasm = wat::parse_file(&self.path)?;
        let elf = if wasmparser::Parser::is_component(&wasm) {
            engine
                .precompile_component(&wasm)
                .context("failed to compile component")?
        } else {
            engine
                .precompile_module(&wasm)
                .context("failed to compile module")?
        };

        match self.config.test {
            TestKind::Clif | TestKind::Optimize => {
                // Read all `*.clif` files from the clif directory that the
                // compilation process just emitted.
                let mut clifs = Vec::new();

                // Sort entries for determinism; multiple wasm modules can
                // generate clif functions with the same names, so sorting the
                // resulting clif functions alone isn't good enough.
                let mut entries = tempdir
                    .path()
                    .read_dir()
                    .context("failed to read tempdir")?
                    .map(|e| Ok(e.context("failed to iterate over tempdir")?.path()))
                    .collect::<Result<Vec<_>>>()?;
                entries.sort();

                for path in entries {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        let filter = self.config.filter.as_deref().unwrap_or("wasm[0]--function");
                        if !name.contains(filter) {
                            continue;
                        }
                    }
                    let clif = std::fs::read_to_string(&path)
                        .with_context(|| format!("failed to read clif file {path:?}"))?;
                    clifs.push(clif);
                }

                // Parse the text format CLIF which is emitted by Wasmtime back
                // into in-memory data structures.
                let functions = clifs
                    .iter()
                    .map(|clif| {
                        let mut funcs = cranelift_reader::parse_functions(clif)?;
                        if funcs.len() != 1 {
                            bail!("expected one function per clif");
                        }
                        Ok(funcs.remove(0))
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(CompileOutput::Clif(functions))
            }
            TestKind::Compile | TestKind::Winch => Ok(CompileOutput::Elf(elf)),
        }
    }
}

enum CompileOutput {
    Clif(Vec<Function>),
    Elf(Vec<u8>),
}

/// Assert that `wat` contains the test expectations necessary for `funcs`.
fn assert_output(test: &Test, output: CompileOutput) -> Result<()> {
    let mut actual = String::new();
    match output {
        CompileOutput::Clif(funcs) => {
            for mut func in funcs {
                func.dfg.resolve_all_aliases();
                writeln!(&mut actual, "{}", func.display()).unwrap();
            }
        }
        CompileOutput::Elf(bytes) => {
            let mut cmd = wasmtime_test_util::command(env!("CARGO_BIN_EXE_wasmtime"));
            cmd.arg("objdump")
                .arg("--address-width=4")
                .arg("--address-jumps")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            match &test.config.objdump {
                Some(args) => {
                    cmd.args(args.to_vec());
                }
                None => {
                    cmd.arg("--traps=false");
                }
            }
            if let Some(filter) = &test.config.filter {
                cmd.arg("--filter").arg(filter);
            }

            let mut child = cmd.spawn().context("failed to run wasmtime")?;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(&bytes)
                .context("failed to write stdin")?;
            let output = child
                .wait_with_output()
                .context("failed to wait for child")?;
            if !output.status.success() {
                bail!(
                    "objdump failed: {}\nstderr: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr),
                );
            }
            actual = String::from_utf8(output.stdout).unwrap();
        }
    }
    let actual = actual.trim();
    assert_or_bless_output(&test.path, &test.contents, actual)
}

fn assert_or_bless_output(path: &Path, wat: &str, actual: &str) -> Result<()> {
    log::debug!("=== actual ===\n{actual}");
    // The test's expectation is the final comment.
    let mut expected_lines: Vec<_> = wat
        .lines()
        .rev()
        .map_while(|l| l.strip_prefix(";;"))
        .map(|l| l.strip_prefix(" ").unwrap_or(l))
        .collect();
    expected_lines.reverse();
    let expected = expected_lines.join("\n");
    let expected = expected.trim();
    log::debug!("=== expected ===\n{expected}");

    if actual == expected {
        return Ok(());
    }

    if std::env::var("WASMTIME_TEST_BLESS").unwrap_or_default() == "1" {
        let old_expectation_line_count = wat
            .lines()
            .rev()
            .take_while(|l| l.starts_with(";;"))
            .count();
        let old_wat_line_count = wat.lines().count();
        let new_wat_lines: Vec<_> = wat
            .lines()
            .take(old_wat_line_count - old_expectation_line_count)
            .map(|l| l.to_string())
            .chain(actual.lines().map(|l| {
                if l.is_empty() {
                    ";;".to_string()
                } else {
                    format!(";; {l}")
                }
            }))
            .collect();
        let mut new_wat = new_wat_lines.join("\n");
        new_wat.push('\n');
        std::fs::write(path, new_wat)
            .with_context(|| format!("failed to write file: {}", path.display()))?;
        return Ok(());
    }

    bail!(
        "Did not get the expected CLIF translation:\n\n\
         {}\n\n\
         Note: You can re-run with the `WASMTIME_TEST_BLESS=1` environment\n\
         variable set to update test expectations.",
        TextDiff::from_lines(expected, actual)
            .unified_diff()
            .header("expected", "actual")
    )
}
