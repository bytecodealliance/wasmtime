//! A generic representation for addressing memory.
use crate::isa::reg::Reg;

// TODO
// Add the other modes
#[derive(Debug, Copy, Clone)]
pub(crate) enum Address {
    Base { base: Reg, imm: u32 },
}

impl Address {
    pub fn base(base: Reg, imm: u32) -> Address {
        Address::Base { base, imm }
    }
}
