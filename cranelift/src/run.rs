//! CLI tool to compile Cranelift IR files to native code in memory and execute them.

use crate::utils::{iterate_files, read_to_string};
use anyhow::Result;
use clap::Parser;
use cranelift_codegen::isa::{CallConv, OwnedTargetIsa};
use cranelift_filetests::TestFileCompiler;
use cranelift_native::builder as host_isa_builder;
use cranelift_reader::{Details, IsaSpec, ParseOptions, parse_run_command, parse_test};
use std::path::{Path, PathBuf};
use target_lexicon::{Architecture, HOST, Triple};

/// Execute clif code and verify with test expressions
#[derive(Parser)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Be more verbose
    #[arg(short, long)]
    verbose: bool,
}

pub fn run(options: &Options) -> Result<()> {
    let stdin_exist = options
        .files
        .iter()
        .find(|file| *file == Path::new("-"))
        .is_some();
    let filtered_files = options
        .files
        .iter()
        .cloned()
        .filter(|file| *file != Path::new("-"))
        .collect::<Vec<_>>();
    let mut total = 0;
    let mut errors = 0;
    let mut special_files: Vec<PathBuf> = vec![];
    if stdin_exist {
        special_files.push("-".into());
    }
    for file in iterate_files(&filtered_files).chain(special_files) {
        total += 1;
        match run_single_file(&file) {
            Ok(_) => {
                if options.verbose {
                    println!("{}", file.to_string_lossy());
                }
            }
            Err(e) => {
                if options.verbose {
                    println!("{}: {}", file.to_string_lossy(), e);
                }
                errors += 1;
            }
        }
    }

    if options.verbose {
        match total {
            0 => println!("0 files"),
            1 => println!("1 file"),
            n => println!("{n} files"),
        }
    }

    match errors {
        0 => Ok(()),
        1 => anyhow::bail!("1 failure"),
        n => anyhow::bail!("{n} failures"),
    }
}

/// Run all functions in a file that are succeeded by "run:" comments
fn run_single_file(path: &PathBuf) -> Result<()> {
    let file_contents = read_to_string(&path)?;
    run_file_contents(file_contents)
}

/// Main body of `run_single_file` separated for testing
fn run_file_contents(file_contents: String) -> Result<()> {
    let options = ParseOptions {
        default_calling_convention: CallConv::triple_default(&Triple::host()), // use the host's default calling convention
        ..ParseOptions::default()
    };
    let test_file = parse_test(&file_contents, options)?;
    let isa = create_target_isa(&test_file.isa_spec)?;
    let mut tfc = TestFileCompiler::new(isa);
    tfc.add_testfile(&test_file)?;
    let compiled = tfc.compile()?;

    for (func, Details { comments, .. }) in test_file.functions {
        for comment in comments {
            if let Some(command) = parse_run_command(comment.text, &func.signature)? {
                let trampoline = compiled.get_trampoline(&func).unwrap();

                command
                    .run(|_, args| Ok(trampoline.call(&compiled, args)))
                    .map_err(|s| anyhow::anyhow!("{s}"))?;
            }
        }
    }
    Ok(())
}

/// Build an ISA based on the current machine running this code (the host)
fn create_target_isa(isa_spec: &IsaSpec) -> Result<OwnedTargetIsa> {
    let builder = host_isa_builder().map_err(|s| anyhow::anyhow!("{s}"))?;
    match *isa_spec {
        IsaSpec::None(ref flags) => {
            // build an ISA for the current machine
            Ok(builder.finish(flags.clone())?)
        }
        IsaSpec::Some(ref isas) => {
            // Keep host feature detection. Only the shared flags come from the
            // file, and RISC-V 64-bit variants are interchangeable.
            for isa in isas {
                if architectures_compatible(HOST.architecture, isa.triple().architecture) {
                    return Ok(builder.finish(isa.flags().clone())?);
                }
            }
            anyhow::bail!(
                "The target ISA specified in the file is not compatible with the host ISA"
            )
        }
    }
}

/// Architectures `clif-util run` can execute on this host.
///
/// Identical architectures match. `riscv64` and `riscv64gc` (and the other
/// 64-bit RISC-V variants) match each other. Unrelated architectures do not.
pub(crate) fn architectures_compatible(host: Architecture, requested: Architecture) -> bool {
    host == requested
        || matches!(
            (host, requested),
            (Architecture::Riscv64(_), Architecture::Riscv64(_))
        )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn riscv64_architecture_variants_are_compatible() {
        use target_lexicon::{Riscv32Architecture, Riscv64Architecture};

        let riscv64 = Architecture::Riscv64(Riscv64Architecture::Riscv64);
        let riscv64gc = Architecture::Riscv64(Riscv64Architecture::Riscv64gc);
        let riscv32 = Architecture::Riscv32(Riscv32Architecture::Riscv32);
        let x64 = Architecture::X86_64;

        assert!(architectures_compatible(riscv64, riscv64gc));
        assert!(architectures_compatible(riscv64gc, riscv64));
        assert!(architectures_compatible(riscv64, riscv64));
        assert!(architectures_compatible(riscv64gc, riscv64gc));
        assert!(architectures_compatible(x64, x64));
        assert!(!architectures_compatible(riscv64, x64));
        assert!(!architectures_compatible(x64, riscv64gc));
        assert!(!architectures_compatible(riscv64, riscv32));
        assert!(!architectures_compatible(riscv32, riscv64gc));
    }

    #[test]
    fn nop() {
        if cranelift_native::builder().is_err() {
            return;
        }
        let code = String::from(
            "
            function %test() -> i8 {
            block0:
                nop
                v1 = iconst.i8 -1
                return v1
            }
            ; run
            ",
        );
        run_file_contents(code).unwrap()
    }
}
