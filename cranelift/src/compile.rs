//! CLI tool to read Cranelift IR files and compile them into native code.

use crate::disasm::print_all;
use crate::utils::{parse_sets_and_triple, read_to_string};
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings::FlagsOrIsa;
use cranelift_codegen::timing;
use cranelift_codegen::Context;
use cranelift_reader::{parse_test, ParseOptions};
use std::path::Path;
use std::path::PathBuf;

/// Compiles Cranelift IR into target language
#[derive(Parser)]
pub struct Options {
    /// Print the resulting Cranelift IR
    #[clap(short)]
    print: bool,

    /// Print pass timing report
    #[clap(short = 'T')]
    report_times: bool,

    /// Print machine code disassembly
    #[clap(short = 'D', long)]
    disasm: bool,

    /// Configure Cranelift settings
    #[clap(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target
    #[clap(long = "target")]
    target: String,

    /// Specify an input file to be used. Use '-' for stdin.
    files: Vec<PathBuf>,
}

pub fn run(options: &Options) -> Result<()> {
    let parsed = parse_sets_and_triple(&options.settings, &options.target)?;
    for path in &options.files {
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(options, path, &name, parsed.as_fisa())?;
    }
    Ok(())
}

fn handle_module(options: &Options, path: &Path, name: &str, fisa: FlagsOrIsa) -> Result<()> {
    let buffer = read_to_string(&path)?;
    let test_file = parse_test(&buffer, ParseOptions::default())
        .with_context(|| format!("failed to parse {}", name))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = fisa.isa.or(test_file.isa_spec.unique_isa());

    if isa.is_none() {
        anyhow::bail!("compilation requires a target isa");
    };

    for (func, _) in test_file.functions {
        if let Some(isa) = isa {
            let mut context = Context::new();
            context.func = func;
            let mut mem = vec![];

            // Compile and encode the result to machine code.
            context
                .compile_and_emit(isa, &mut mem)
                .map_err(|err| anyhow::anyhow!("{}", pretty_error(&context.func, err)))?;
            let result = context.mach_compile_result.as_ref().unwrap();
            let code_info = result.code_info();

            if options.print {
                println!("{}", context.func.display());
            }

            if options.disasm {
                print_all(
                    isa,
                    &mem,
                    code_info.total_size,
                    options.print,
                    result.buffer.relocs(),
                    result.buffer.traps(),
                    result.buffer.stack_maps(),
                )?;
            }
        }
    }

    if options.report_times {
        print!("{}", timing::take_current());
    }

    Ok(())
}
