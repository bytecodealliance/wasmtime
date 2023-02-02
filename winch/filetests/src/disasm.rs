//! Disassembly utilities.

use anyhow::{bail, Result};
use capstone::prelude::*;
use std::fmt::Write;
use target_lexicon::Architecture;
use winch_codegen::TargetIsa;

/// Disassemble and print a machine code buffer.
pub fn disasm(bytes: &[u8], isa: &dyn TargetIsa) -> Result<Vec<String>> {
    let dis = disassembler_for(isa)?;
    let insts = dis.disasm_all(bytes, 0x0).unwrap();

    let disassembled_lines = insts
        .iter()
        .map(|i| {
            let mut line = String::new();

            write!(&mut line, "{:4x}:\t ", i.address()).unwrap();

            let mut bytes_str = String::new();
            let mut len = 0;
            for b in i.bytes() {
                write!(&mut bytes_str, "{:02x}", b).unwrap();
                len += 1;
            }
            write!(&mut line, "{:21}\t", bytes_str).unwrap();
            if len > 8 {
                write!(&mut line, "\n\t\t\t\t").unwrap();
            }

            if let Some(s) = i.mnemonic() {
                write!(&mut line, "{}\t", s).unwrap();
            }

            if let Some(s) = i.op_str() {
                write!(&mut line, "{}", s).unwrap();
            }

            line
        })
        .collect();

    Ok(disassembled_lines)
}

fn disassembler_for(isa: &dyn TargetIsa) -> Result<Capstone> {
    let disasm = match isa.triple().architecture {
        Architecture::X86_64 => Capstone::new()
            .x86()
            .mode(arch::x86::ArchMode::Mode64)
            .build()
            .map_err(|e| anyhow::format_err!("{}", e))?,

        _ => bail!("Unsupported ISA"),
    };

    Ok(disasm)
}
