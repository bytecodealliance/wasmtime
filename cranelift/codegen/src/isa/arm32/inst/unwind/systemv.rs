//! Unwind information for System V ABI (ARM32).

use crate::isa::unwind::systemv::{RegisterMapper as SystemVRegisterMapper, RegisterMappingError};
use crate::machinst::Reg;
use gimli::{Encoding, Format, Register, write::CommonInformationEntry};

/// Creates a new ARM32 common information entry (CIE).
pub fn create_cie() -> CommonInformationEntry {
    let entry = CommonInformationEntry::new(
        Encoding {
            address_size: 4,
            format: Format::Dwarf32,
            version: 1,
        },
        2,            // Code alignment factor
        -4,           // Data alignment factor (ARM stack grows down)
        Register(14), // Return address register (lr = r14)
    );

    entry
}

//#[derive(Clone, Copy, Debug)]
pub(crate) struct RegisterMapper;

impl SystemVRegisterMapper<Reg> for RegisterMapper {
    fn map(&self, _reg: Reg) -> Result<u16, RegisterMappingError> {
        Err(RegisterMappingError::UnsupportedArchitecture)
    }

    fn fp(&self) -> Option<u16> {
        Some(11) // FP is r11 on ARM32 (AAPCS)
    }

    fn lr(&self) -> Option<u16> {
        Some(14) // LR is r14 on ARM32
    }
}

pub fn map_reg(_reg: Reg) -> Result<Register, RegisterMappingError> {
    Err(RegisterMappingError::UnsupportedArchitecture)
}
