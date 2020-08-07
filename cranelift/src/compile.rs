//! CLI tool to read Cranelift IR files and compile them into native code.

use crate::disasm::{print_all, PrintRelocs, PrintStackMaps, PrintTraps};
use crate::utils::{parse_sets_and_triple, read_to_string};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::settings::FlagsOrIsa;
use cranelift_codegen::timing;
use cranelift_codegen::Context;
use cranelift_reader::{parse_test, ParseOptions};
use std::path::Path;
use std::path::PathBuf;

pub fn run(
    files: Vec<String>,
    flag_print: bool,
    flag_disasm: bool,
    flag_report_times: bool,
    flag_set: &[String],
    flag_isa: &str,
) -> Result<(), String> {
    let parsed = parse_sets_and_triple(flag_set, flag_isa)?;

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(
            flag_print,
            flag_disasm,
            flag_report_times,
            &path.to_path_buf(),
            &name,
            parsed.as_fisa(),
        )?;
    }
    Ok(())
}

fn handle_module(
    flag_print: bool,
    flag_disasm: bool,
    flag_report_times: bool,
    path: &PathBuf,
    name: &str,
    fisa: FlagsOrIsa,
) -> Result<(), String> {
    let buffer = read_to_string(&path).map_err(|e| format!("{}: {}", name, e))?;
    let test_file =
        parse_test(&buffer, ParseOptions::default()).map_err(|e| format!("{}: {}", name, e))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = fisa.isa.or(test_file.isa_spec.unique_isa());

    if isa.is_none() {
        return Err(String::from("compilation requires a target isa"));
    };

    for (func, _) in test_file.functions {
        let mut relocs = PrintRelocs::new(flag_print);
        let mut traps = PrintTraps::new(flag_print);
        let mut stack_maps = PrintStackMaps::new(flag_print);

        if let Some(isa) = isa {
            let mut context = Context::new();
            context.func = func;
            let mut mem = vec![];

            // Compile and encode the result to machine code.
            let code_info = context
                .compile_and_emit(isa, &mut mem, &mut relocs, &mut traps, &mut stack_maps)
                .map_err(|err| pretty_error(&context.func, Some(isa), err))?;

            if flag_print {
                println!("{}", context.func.display(isa));
            }

            if flag_disasm {
                print_all(
                    isa,
                    &mem,
                    code_info.code_size,
                    code_info.jumptables_size + code_info.rodata_size,
                    &relocs,
                    &traps,
                    &stack_maps,
                )?;
            }
        }
    }

    if flag_report_times {
        print!("{}", timing::take_current());
    }

    Ok(())
}
