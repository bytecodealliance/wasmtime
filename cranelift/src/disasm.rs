use anyhow::Result;
use cfg_if::cfg_if;
use cranelift_codegen::ir::function::FunctionParameters;
use cranelift_codegen::ir::Function;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{FinalizedMachReloc, MachTrap};
use std::fmt::Write;

fn print_relocs(func_params: &FunctionParameters, relocs: &[FinalizedMachReloc]) -> String {
    let mut text = String::new();
    for &FinalizedMachReloc {
        kind,
        offset,
        ref target,
        addend,
    } in relocs
    {
        writeln!(
            text,
            "reloc_external: {} {} {} at {}",
            kind,
            target.display(Some(func_params)),
            addend,
            offset
        )
        .unwrap();
    }
    text
}

pub fn print_traps(traps: &[MachTrap]) -> String {
    let mut text = String::new();
    for &MachTrap { offset, code } in traps {
        writeln!(text, "trap: {code} at {offset:#x}").unwrap();
    }
    text
}

cfg_if! {
    if #[cfg(feature = "disas")] {
        pub fn print_disassembly(func: &Function, isa: &dyn TargetIsa, mem: &[u8]) -> Result<()> {
            #[cfg(feature = "pulley")]
            let is_pulley = match isa.triple().architecture {
                target_lexicon::Architecture::Pulley32 | target_lexicon::Architecture::Pulley64 => true,
                _ => false,
            };
            println!("\nDisassembly of {} bytes <{}>:", mem.len(), func.name);

            #[cfg(feature = "pulley")]
            if is_pulley {
                let mut disas = pulley_interpreter::disas::Disassembler::new(mem);
                pulley_interpreter::decode::Decoder::decode_all(&mut disas)?;
                println!("{}", disas.disas());
                return Ok(());
            }
            let cs = isa.to_capstone().map_err(|e| anyhow::format_err!("{}", e))?;

            let insns = cs.disasm_all(&mem, 0x0).unwrap();
            for i in insns.iter() {
                let mut line = String::new();

                write!(&mut line, "{:4x}:\t", i.address()).unwrap();

                let mut bytes_str = String::new();
                let mut len = 0;
                let mut first = true;
                for b in i.bytes() {
                    if !first {
                        write!(&mut bytes_str, " ").unwrap();
                    }
                    write!(&mut bytes_str, "{b:02x}").unwrap();
                    len += 1;
                    first = false;
                }
                write!(&mut line, "{bytes_str:21}\t").unwrap();
                if len > 8 {
                    write!(&mut line, "\n\t\t\t\t").unwrap();
                }

                if let Some(s) = i.mnemonic() {
                    write!(&mut line, "{s}\t").unwrap();
                }

                if let Some(s) = i.op_str() {
                    write!(&mut line, "{s}").unwrap();
                }

                println!("{line}");
            }
            Ok(())
        }
    } else {
        pub fn print_disassembly(_: &Function, _: &dyn TargetIsa, _: &[u8]) -> Result<()> {
            println!("\nNo disassembly available.");
            Ok(())
        }
    }
}

pub fn print_all(
    isa: &dyn TargetIsa,
    func: &Function,
    mem: &[u8],
    code_size: u32,
    print: bool,
    relocs: &[FinalizedMachReloc],
    traps: &[MachTrap],
) -> Result<()> {
    print_bytes(&mem);
    print_disassembly(func, isa, &mem[0..code_size as usize])?;
    if print {
        println!(
            "\n{}\n{}",
            print_relocs(&func.params, relocs),
            print_traps(traps),
        );
    }
    Ok(())
}

pub fn print_bytes(mem: &[u8]) {
    print!(".byte ");
    let mut first = true;
    for byte in mem.iter() {
        if first {
            first = false;
        } else {
            print!(", ");
        }
        print!("{byte}");
    }
    println!();
}
