//! A filetest-lookalike test suite using Cranelift tooling but built on
//! Wasmtime's code generator.
//!
//! This test will read the `tests/asm/*` directory and interpret all files in
//! that directory as a test. Each test must be in the wasm text format and
//! start with directives that look like:
//!
//! ```wasm
//! ;;! target: x86_64
//! ;;! compile
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
//! * `optimize` - CLIF is emitted, then optimized, then inspected.
//! * `compile` - backends are run to produce machine code and that's inspected.
//!
//! Tests may also have a `flags` directive which are CLI flags to Wasmtime
//! itself:
//!
//! ```wasm
//! ;;! target: x86_64
//! ;;! flags: -O opt-level=s
//!
//! (module
//!     ;; ...
//! )
//! ```
//!
//! Flags are parsed by the `wasmtime_cli_flags` crate to build a `Config`.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use cranelift_codegen::ir::Function;
use cranelift_codegen::settings::{Configurable, Flags, SetError};
use cranelift_filetests::test_wasm::{run_functions, TestKind};
use std::path::Path;
use tempfile::TempDir;
use wasmtime::{Engine, OptLevel};

fn main() {
    // First discover all tests ...
    let mut tests = Vec::new();
    for file in std::fs::read_dir("./tests/asm").unwrap() {
        tests.push(file.unwrap().path());
    }

    // ... then run all tests!
    for test in tests.iter() {
        run_test(&test)
            .with_context(|| format!("failed to run tests {test:?}"))
            .unwrap();
    }
}

fn run_test(path: &Path) -> Result<()> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("failed to read {path:?}"))?;

    // Parse the `contents` looking for directive-based comments starting with
    // `;;!` near the top of the file.
    let mut flags = vec!["wasmtime"];
    let mut compile = false;
    let mut optimize = false;
    let mut target = None;
    for line in contents.lines() {
        let directive = match line.strip_prefix(";;!") {
            Some("") | None => continue,
            Some(s) => s.trim(),
        };
        if directive == "compile" {
            compile = true;
            continue;
        }
        if directive == "optimize" {
            optimize = true;
            continue;
        }
        if let Some(s) = directive.strip_prefix("flags: ") {
            flags.extend(s.split_whitespace().filter(|s| !s.is_empty()));
            continue;
        }
        if let Some(s) = directive.strip_prefix("target: ") {
            if target.is_some() {
                bail!("two targets have been specified");
            }
            target = Some(s);
            continue;
        }

        bail!("unknown directive: {directive}");
    }

    // Use the file-based directives to create a `wasmtime::Config`. Note that
    // this config is configured to emit CLIF in a temporary directory, and
    // then the config is used to compile the input wasm file.
    let tempdir = TempDir::new().context("failed to make a tempdir")?;
    let target =
        target.ok_or_else(|| anyhow!("test must specify a target with `;;! target: ...`"))?;
    let mut opts = wasmtime_cli_flags::CommonOptions::try_parse_from(flags)?;
    let mut config = opts.config(Some(target))?;
    config.emit_clif(tempdir.path());
    let engine = Engine::new(&config).context("failed to create engine")?;
    let module = wat::parse_file(path)?;
    engine
        .precompile_module(&module)
        .context("failed to compile module")?;

    // Read all `*.clif` files from the clif directory that the compilation
    // process just emitted.
    //
    // Afterward remove the temporary directory and then use `cranelift_reader`
    // to parse everything into a `Function`.
    let mut clifs = Vec::new();
    for entry in tempdir
        .path()
        .read_dir()
        .context("failed to read tempdir")?
    {
        let entry = entry.context("failed to iterate over tempdir")?;
        let path = entry.path();
        let clif = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read clif file {path:?}"))?;
        clifs.push(clif);
    }
    drop(tempdir);
    clifs.sort();
    let functions = clifs
        .iter()
        .map(|s| parse_clif(s))
        .collect::<Result<Vec<_>>>()?;

    // Determine the kind of test and build a `TargetIsa` based on the `target`
    // name and configuration settings in the CLI flags.
    let kind = if compile {
        if optimize {
            bail!("can't be both an `optimize` and `compile` test");
        }
        TestKind::Compile
    } else {
        TestKind::Clif { optimize }
    };
    let mut builder = cranelift_codegen::isa::lookup_by_name(target)?;
    let mut flags = cranelift_codegen::settings::builder();
    let opt_level = match opts.opts.opt_level {
        None | Some(OptLevel::Speed) => "speed",
        Some(OptLevel::SpeedAndSize) => "speed_and_size",
        Some(OptLevel::None) => "none",
        _ => unreachable!(),
    };
    flags.set("opt_level", opt_level)?;
    for (key, val) in opts.codegen.cranelift.iter() {
        let key = &key.replace("-", "_");
        let target_res = match val {
            Some(val) => builder.set(key, val),
            None => builder.enable(key),
        };
        match target_res {
            Ok(()) => continue,
            Err(SetError::BadName(_)) => {}
            Err(e) => bail!(e),
        }
        match val {
            Some(val) => flags.set(key, val)?,
            None => flags.enable(key)?,
        }
    }
    let isa = builder.finish(Flags::new(flags))?;

    // And finally, use `cranelift_filetests` to perform the rest of the test.
    run_functions(path, &contents, &*isa, kind, &functions)?;

    Ok(())
}

fn parse_clif(clif: &str) -> Result<Function> {
    let mut funcs = cranelift_reader::parse_functions(clif)?;
    if funcs.len() != 1 {
        bail!("expected one function per clif");
    }
    Ok(funcs.remove(0))
}
