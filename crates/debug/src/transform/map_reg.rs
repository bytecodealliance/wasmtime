use anyhow::{bail, Result};
use gimli::{Register, X86_64};
use wasmtime_environ::isa::{RegUnit, TargetIsa};

pub(crate) fn map_reg(isa: &dyn TargetIsa, reg: RegUnit) -> Result<Register> {
    // TODO avoid duplication with fde.rs
    assert!(isa.name() == "x86" && isa.pointer_bits() == 64);
    // Mapping from https://github.com/bytecodealliance/cranelift/pull/902 by @iximeow
    const X86_GP_REG_MAP: [Register; 16] = [
        X86_64::RAX,
        X86_64::RCX,
        X86_64::RDX,
        X86_64::RBX,
        X86_64::RSP,
        X86_64::RBP,
        X86_64::RSI,
        X86_64::RDI,
        X86_64::R8,
        X86_64::R9,
        X86_64::R10,
        X86_64::R11,
        X86_64::R12,
        X86_64::R13,
        X86_64::R14,
        X86_64::R15,
    ];
    const X86_XMM_REG_MAP: [Register; 16] = [
        X86_64::XMM0,
        X86_64::XMM1,
        X86_64::XMM2,
        X86_64::XMM3,
        X86_64::XMM4,
        X86_64::XMM5,
        X86_64::XMM6,
        X86_64::XMM7,
        X86_64::XMM8,
        X86_64::XMM9,
        X86_64::XMM10,
        X86_64::XMM11,
        X86_64::XMM12,
        X86_64::XMM13,
        X86_64::XMM14,
        X86_64::XMM15,
    ];
    let reg_info = isa.register_info();
    let bank = reg_info.bank_containing_regunit(reg).unwrap();
    match bank.name {
        "IntRegs" => {
            // x86 GP registers have a weird mapping to DWARF registers, so we use a
            // lookup table.
            Ok(X86_GP_REG_MAP[(reg - bank.first_unit) as usize])
        }
        "FloatRegs" => Ok(X86_XMM_REG_MAP[(reg - bank.first_unit) as usize]),
        bank_name => {
            bail!("unsupported register bank: {}", bank_name);
        }
    }
}
