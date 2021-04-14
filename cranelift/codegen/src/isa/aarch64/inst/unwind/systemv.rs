//! Unwind information for System V ABI (Aarch64).

use crate::isa::aarch64::inst::regs;
use crate::isa::unwind::systemv::RegisterMappingError;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register};
use regalloc::{Reg, RegClass};

/// Creates a new aarch64 common information entry (CIE).
pub fn create_cie() -> CommonInformationEntry {
    use gimli::write::CallFrameInstruction;

    let mut entry = CommonInformationEntry::new(
        Encoding {
            address_size: 8,
            format: Format::Dwarf32,
            version: 1,
        },
        4,  // Code alignment factor
        -8, // Data alignment factor
        Register(regs::link_reg().get_hw_encoding().into()),
    );

    // Every frame will start with the call frame address (CFA) at SP
    let sp = Register(regs::stack_reg().get_hw_encoding().into());
    entry.add_instruction(CallFrameInstruction::Cfa(sp, 0));

    entry
}

/// Map Cranelift registers to their corresponding Gimli registers.
pub fn map_reg(reg: Reg) -> Result<Register, RegisterMappingError> {
    // For AArch64 DWARF register mappings, see:
    //
    // https://developer.arm.com/documentation/ihi0057/e/?lang=en#dwarf-register-names
    //
    // X0--X31 is 0--31; V0--V31 is 64--95.
    match reg.get_class() {
        RegClass::I64 => {
            let reg = reg.get_hw_encoding() as u16;
            Ok(Register(reg))
        }
        RegClass::V128 => {
            let reg = reg.get_hw_encoding() as u16;
            Ok(Register(64 + reg))
        }
        _ => Err(RegisterMappingError::UnsupportedRegisterBank("class?")),
    }
}

pub(crate) struct RegisterMapper;

impl crate::isa::unwind::systemv::RegisterMapper<Reg> for RegisterMapper {
    fn map(&self, reg: Reg) -> Result<u16, RegisterMappingError> {
        Ok(map_reg(reg)?.0)
    }
    fn sp(&self) -> u16 {
        regs::stack_reg().get_hw_encoding().into()
    }
    fn fp(&self) -> Option<u16> {
        Some(regs::fp_reg().get_hw_encoding().into())
    }
    fn lr(&self) -> Option<u16> {
        Some(regs::link_reg().get_hw_encoding().into())
    }
    fn lr_offset(&self) -> Option<u32> {
        Some(8)
    }
}
