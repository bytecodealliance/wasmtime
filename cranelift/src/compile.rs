//! CLI tool to read Cranelift IR files and compile them into native code.

use crate::disasm::print_all;
use crate::utils::read_to_string;
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings::FlagsOrIsa;
use cranelift_codegen::timing;
use cranelift_codegen::Context;
use cranelift_reader::OwnedFlagsOrIsa;
use cranelift_reader::{parse_sets_and_triple, parse_test, ParseOptions};
use std::path::Path;
use std::path::PathBuf;

/// Compiles Cranelift IR into target language
#[derive(Parser)]
pub struct Options {
    /// Print the resulting Cranelift IR
    #[arg(short)]
    print: bool,

    /// Print pass timing report
    #[arg(short = 'T')]
    report_times: bool,

    /// Print machine code disassembly
    #[arg(short = 'D', long)]
    disasm: bool,

    /// Configure Cranelift settings
    #[arg(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target
    #[arg(long = "target")]
    target: String,

    /// Specify an input file to be used. Use '-' for stdin.
    files: Vec<PathBuf>,

    /// Output object file
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
}

pub fn run(options: &Options) -> Result<()> {
    let parsed = parse_sets_and_triple(&options.settings, &options.target)?;

    let mut module = match (&options.output, &parsed) {
        (Some(output), OwnedFlagsOrIsa::Isa(isa)) => {
            let builder = cranelift_object::ObjectBuilder::new(
                isa.clone(),
                output
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("a.out"),
                cranelift_module::default_libcall_names(),
            )?;
            Some(cranelift_object::ObjectModule::new(builder))
        }
        _ => None,
    };

    for path in &options.files {
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(options, path, &name, parsed.as_fisa(), module.as_mut())?;
    }

    if let (Some(module), Some(output)) = (module, &options.output) {
        let bytes = module.finish().emit()?;
        std::fs::write(output, bytes)?;
    }

    Ok(())
}

fn handle_module(
    options: &Options,
    path: &Path,
    name: &str,
    fisa: FlagsOrIsa,
    module: Option<&mut impl cranelift_module::Module>,
) -> Result<()> {
    let buffer = read_to_string(&path)?;
    let test_file = parse_test(&buffer, ParseOptions::default())
        .with_context(|| format!("failed to parse {name}"))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = fisa.isa.or(test_file.isa_spec.unique_isa());

    let isa = match isa {
        None => anyhow::bail!("compilation requires a target isa"),
        Some(isa) => isa,
    };

    for (func, _) in test_file.functions {
        let mut context = Context::new();
        context.func = func;
        let mut mem = vec![];

        // Compile and encode the result to machine code.
        let compiled_code = context
            .compile_and_emit(isa, &mut mem, &mut Default::default())
            .map_err(|err| anyhow::anyhow!("{}", pretty_error(&err.func, err.inner)))?;
        let code_info = compiled_code.code_info();

        if let Some(&mut ref mut module) = module {
            let name = context.func.name.to_string();
            let fid = module.declare_function(
                &name,
                cranelift_module::Linkage::Export,
                &context.func.signature,
            )?;
            module.define_function_with_control_plane(
                fid,
                &mut context,
                &mut Default::default(),
            )?;
        }

        if options.print {
            println!("{}", context.func.display());
        }

        if options.disasm {
            let result = context.compiled_code().unwrap();
            print_all(
                isa,
                &context.func,
                &mem,
                code_info.total_size,
                options.print,
                result.buffer.relocs(),
                result.buffer.traps(),
                result.buffer.stack_maps(),
            )?;
        }
    }

    if options.report_times {
        print!("{}", timing::take_current());
    }

    Ok(())
}
