//! CLI tool to read Cretonne IR files and compile them into native code.

use capstone::prelude::*;
use cretonne_codegen::isa::TargetIsa;
use cretonne_codegen::print_errors::pretty_error;
use cretonne_codegen::settings::FlagsOrIsa;
use cretonne_codegen::Context;
use cretonne_codegen::{binemit, ir};
use cretonne_reader::parse_test;
use std::path::Path;
use std::path::PathBuf;
use utils::{parse_sets_and_isa, read_to_string};

struct PrintRelocs {
    flag_print: bool,
}

impl binemit::RelocSink for PrintRelocs {
    fn reloc_ebb(
        &mut self,
        where_: binemit::CodeOffset,
        r: binemit::Reloc,
        offset: binemit::CodeOffset,
    ) {
        if self.flag_print {
            println!("reloc_ebb: {} {} at {}", r, offset, where_);
        }
    }

    fn reloc_external(
        &mut self,
        where_: binemit::CodeOffset,
        r: binemit::Reloc,
        name: &ir::ExternalName,
        addend: binemit::Addend,
    ) {
        if self.flag_print {
            println!("reloc_ebb: {} {} {} at {}", r, name, addend, where_);
        }
    }

    fn reloc_jt(&mut self, where_: binemit::CodeOffset, r: binemit::Reloc, jt: ir::JumpTable) {
        if self.flag_print {
            println!("reloc_ebb: {} {} at {}", r, jt, where_);
        }
    }
}

struct PrintTraps {
    flag_print: bool,
}

impl binemit::TrapSink for PrintTraps {
    fn trap(&mut self, offset: binemit::CodeOffset, _srcloc: ir::SourceLoc, code: ir::TrapCode) {
        if self.flag_print {
            println!("trap: {} at {}", code, offset);
        }
    }
}

pub fn run(
    files: Vec<String>,
    flag_print: bool,
    flag_set: &[String],
    flag_isa: &str,
) -> Result<(), String> {
    let parsed = parse_sets_and_isa(flag_set, flag_isa)?;

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(flag_print, &path.to_path_buf(), &name, parsed.as_fisa())?;
    }
    Ok(())
}

fn handle_module(
    flag_print: bool,
    path: &PathBuf,
    name: &str,
    fisa: FlagsOrIsa,
) -> Result<(), String> {
    let buffer = read_to_string(&path).map_err(|e| format!("{}: {}", name, e))?;
    let test_file = parse_test(&buffer).map_err(|e| format!("{}: {}", name, e))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = if let Some(isa) = fisa.isa {
        isa
    } else if let Some(isa) = test_file.isa_spec.unique_isa() {
        isa
    } else {
        return Err(String::from("compilation requires a target isa"));
    };

    for (func, _) in test_file.functions {
        let mut context = Context::new();
        context.func = func;

        // Compile and encode the result to machine code.
        let mut mem = Vec::new();
        let mut relocs = PrintRelocs { flag_print };
        let mut traps = PrintTraps { flag_print };
        context
            .compile_and_emit(isa, &mut mem, &mut relocs, &mut traps)
            .map_err(|err| pretty_error(&context.func, Some(isa), err))?;

        if flag_print {
            println!("{}", context.func.display(isa));
        }

        if flag_print {
            print!(".byte ");
            let mut first = true;
            for byte in &mem {
                if first {
                    first = false;
                } else {
                    print!(", ");
                }
                print!("{}", byte);
            }
            println!();

            let cs = get_disassembler(isa)?;

            println!("\nDisassembly:");
            let insns = cs.disasm_all(&mem, 0x0).unwrap();
            for i in insns.iter() {
                println!("{}", i);
            }
        }
    }

    Ok(())
}

fn get_disassembler(isa: &TargetIsa) -> Result<Capstone, String> {
    let cs = match isa.name() {
        "riscv" => return Err(String::from("No disassembler for RiscV")),
        "x86" => {
            if isa.flags().is_64bit() {
                Capstone::new()
                    .x86()
                    .mode(arch::x86::ArchMode::Mode64)
                    .build()
            } else {
                Capstone::new()
                    .x86()
                    .mode(arch::x86::ArchMode::Mode32)
                    .build()
            }
        }
        "arm32" => Capstone::new().arm().mode(arch::arm::ArchMode::Arm).build(),
        "arm64" => Capstone::new()
            .arm64()
            .mode(arch::arm64::ArchMode::Arm)
            .build(),
        _ => return Err(String::from("Unknown ISA")),
    };

    cs.map_err(|err| err.to_string())
}
