//! Disassembly utilities.

use anyhow::{bail, Result};
use capstone::prelude::*;
use std::fmt::Write;
use target_lexicon::Architecture;
use winch_codegen::TargetIsa;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OffsetStyle {
    Minimal,
    Full,
}

/// Disassemble and print a machine code buffer.
pub fn disasm(
    bytes: &[u8],
    isa: &Box<dyn TargetIsa>,
    offset_style: OffsetStyle,
) -> Result<Vec<String>> {
    let dis = disassembler_for(isa)?;
    let insts = dis.disasm_all(bytes, 0x0).unwrap();

    let mut prev_jump = false;
    let mut write_offsets = offset_style == OffsetStyle::Full;

    let disassembled_lines = insts
        .iter()
        .map(|i| {
            use capstone::InsnGroupType::{CS_GRP_JUMP, CS_GRP_RET};

            let detail = dis.insn_detail(&i).unwrap();

            let is_jump = detail
                .groups()
                .find(|g| g.0 as u32 == CS_GRP_JUMP)
                .is_some();

            let mut line = String::new();

            if write_offsets || (prev_jump && !is_jump) {
                write!(&mut line, "{:4x}:\t ", i.address()).unwrap();
            } else {
                write!(&mut line, "     \t ").unwrap();
            }

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

            prev_jump = is_jump;

            // Flip write_offsets to true once we've seen a `ret`, as instructions that follow the
            // return are often related to trap tables.
            write_offsets =
                write_offsets || detail.groups().find(|g| g.0 as u32 == CS_GRP_RET).is_some();

            line
        })
        .collect();

    Ok(disassembled_lines)
}

fn disassembler_for(isa: &Box<dyn TargetIsa>) -> Result<Capstone> {
    let disasm = match isa.triple().architecture {
        Architecture::X86_64 => Capstone::new()
            .x86()
            .mode(arch::x86::ArchMode::Mode64)
            .detail(true)
            .build()
            .map_err(|e| anyhow::format_err!("{}", e))?,

        Architecture::Aarch64 { .. } => {
            let mut cs = Capstone::new()
                .arm64()
                .mode(arch::arm64::ArchMode::Arm)
                .detail(true)
                .build()
                .map_err(|e| anyhow::format_err!("{}", e))?;

            cs.set_skipdata(true)
                .map_err(|e| anyhow::format_err!("{}", e))?;
            cs
        }

        _ => bail!("Unsupported ISA"),
    };

    Ok(disasm)
}
