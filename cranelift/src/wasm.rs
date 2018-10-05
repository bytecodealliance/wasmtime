//! CLI tool to use the functions provided by the [cranelift-wasm](../cranelift_wasm/index.html)
//! crate.
//!
//! Reads Wasm binary files, translates the functions' code to Cranelift IR.
#![cfg_attr(
    feature = "cargo-clippy",
    allow(too_many_arguments, cyclomatic_complexity)
)]

use cranelift_codegen::print_errors::{pretty_error, pretty_verifier_error};
use cranelift_codegen::settings::FlagsOrIsa;
use cranelift_codegen::Context;
use cranelift_entity::EntityRef;
use cranelift_wasm::{
    translate_module, DummyEnvironment, FuncIndex, ModuleEnvironment, ReturnMode,
};
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use term;
use utils::{parse_sets_and_triple, read_to_end};
use wabt::wat2wasm;

macro_rules! vprintln {
    ($x: expr, $($tts:tt)*) => {
        if $x {
            println!($($tts)*);
        }
    }
}

macro_rules! vprint {
    ($x: expr, $($tts:tt)*) => {
        if $x {
            print!($($tts)*);
        }
    }
}

pub fn run(
    files: Vec<String>,
    flag_verbose: bool,
    flag_just_decode: bool,
    flag_check_translation: bool,
    flag_print: bool,
    flag_set: &[String],
    flag_triple: &str,
    flag_print_size: bool,
) -> Result<(), String> {
    let parsed = parse_sets_and_triple(flag_set, flag_triple)?;

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(
            flag_verbose,
            flag_just_decode,
            flag_check_translation,
            flag_print,
            flag_print_size,
            &path.to_path_buf(),
            &name,
            parsed.as_fisa(),
        )?;
    }
    Ok(())
}

fn handle_module(
    flag_verbose: bool,
    flag_just_decode: bool,
    flag_check_translation: bool,
    flag_print: bool,
    flag_print_size: bool,
    path: &PathBuf,
    name: &str,
    fisa: FlagsOrIsa,
) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    let _ = terminal.fg(term::color::YELLOW);
    vprint!(flag_verbose, "Handling: ");
    let _ = terminal.reset();
    vprintln!(flag_verbose, "\"{}\"", name);
    let _ = terminal.fg(term::color::MAGENTA);
    vprint!(flag_verbose, "Translating... ");
    let _ = terminal.reset();

    let mut module_binary =
        read_to_end(path.clone()).map_err(|err| String::from(err.description()))?;

    if !module_binary.starts_with(&[b'\0', b'a', b's', b'm']) {
        module_binary = match wat2wasm(&module_binary) {
            Ok(data) => data,
            Err(e) => return Err(String::from(e.description())),
        };
    }

    let isa = match fisa.isa {
        Some(isa) => isa,
        None => {
            return Err(String::from(
                "Error: the wasm command requires an explicit isa.",
            ))
        }
    };

    let mut dummy_environ = DummyEnvironment::with_triple_flags(
        isa.triple().clone(),
        fisa.flags.clone(),
        ReturnMode::NormalReturns,
    );
    translate_module(&module_binary, &mut dummy_environ).map_err(|e| e.to_string())?;

    let _ = terminal.fg(term::color::GREEN);
    vprintln!(flag_verbose, "ok");
    let _ = terminal.reset();

    if flag_just_decode {
        if !flag_print {
            return Ok(());
        }

        let num_func_imports = dummy_environ.get_num_func_imports();
        for (def_index, func) in dummy_environ.info.function_bodies.iter() {
            let func_index = num_func_imports + def_index.index();
            let mut context = Context::new();
            context.func = func.clone();
            if let Some(start_func) = dummy_environ.info.start_func {
                if func_index == start_func.index() {
                    println!("; Selected as wasm start function");
                }
            }
            vprintln!(flag_verbose, "");
            for export_name in
                &dummy_environ.info.functions[FuncIndex::new(func_index)].export_names
            {
                println!("; Exported as \"{}\"", export_name);
            }
            println!("{}", context.func.display(None));
            vprintln!(flag_verbose, "");
        }
        let _ = terminal.reset();
        return Ok(());
    }

    let _ = terminal.fg(term::color::MAGENTA);
    if flag_check_translation {
        vprint!(flag_verbose, "Checking... ");
    } else {
        vprint!(flag_verbose, "Compiling... ");
    }
    let _ = terminal.reset();

    if flag_print_size {
        vprintln!(flag_verbose, "");
    }

    let num_func_imports = dummy_environ.get_num_func_imports();
    let mut total_module_code_size = 0;
    let mut context = Context::new();
    for (def_index, func) in dummy_environ.info.function_bodies.iter() {
        context.func = func.clone();

        let func_index = num_func_imports + def_index.index();
        if flag_check_translation {
            if let Err(errors) = context.verify(fisa) {
                return Err(pretty_verifier_error(&context.func, fisa.isa, None, errors));
            }
        } else {
            let compiled_size = context
                .compile(isa)
                .map_err(|err| pretty_error(&context.func, fisa.isa, err))?;
            if flag_print_size {
                println!(
                    "Function #{} code size: {} bytes",
                    func_index, compiled_size
                );
                total_module_code_size += compiled_size;
                println!(
                    "Function #{} bytecode size: {} bytes",
                    func_index,
                    dummy_environ.func_bytecode_sizes[def_index.index()]
                );
            }
        }

        if flag_print {
            vprintln!(flag_verbose, "");
            if let Some(start_func) = dummy_environ.info.start_func {
                if func_index == start_func.index() {
                    println!("; Selected as wasm start function");
                }
            }
            for export_name in
                &dummy_environ.info.functions[FuncIndex::new(func_index)].export_names
            {
                println!("; Exported as \"{}\"", export_name);
            }
            println!("{}", context.func.display(fisa.isa));
            vprintln!(flag_verbose, "");
        }

        context.clear();
    }

    if !flag_check_translation && flag_print_size {
        println!("Total module code size: {} bytes", total_module_code_size);
        let total_bytecode_size: usize = dummy_environ.func_bytecode_sizes.iter().sum();
        println!("Total module bytecode size: {} bytes", total_bytecode_size);
    }

    let _ = terminal.fg(term::color::GREEN);
    vprintln!(flag_verbose, "ok");
    let _ = terminal.reset();
    Ok(())
}
