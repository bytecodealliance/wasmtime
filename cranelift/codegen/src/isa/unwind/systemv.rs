//! System V ABI unwind information.

use alloc::vec::Vec;
use gimli::write::{Address, FrameDescriptionEntry};
use thiserror::Error;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

type Register = u16;
type Expression = Vec<u8>;

/// Enumerate the errors possible in mapping Cranelift registers to their DWARF equivalent.
#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RegisterMappingError {
    #[error("unable to find bank for register info")]
    MissingBank,
    #[error("register mapping is currently only implemented for x86_64")]
    UnsupportedArchitecture,
    #[error("unsupported register bank: {0}")]
    UnsupportedRegisterBank(&'static str),
}

// This mirrors gimli's CallFrameInstruction, but is serializable
// TODO: if gimli ever adds serialization support, remove this type
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub(crate) enum CallFrameInstruction {
    Cfa(Register, i32),
    CfaRegister(Register),
    CfaOffset(i32),
    CfaExpression(Expression),
    Restore(Register),
    Undefined(Register),
    SameValue(Register),
    Offset(Register, i32),
    ValOffset(Register, i32),
    Register(Register, Register),
    Expression(Register, Expression),
    ValExpression(Register, Expression),
    RememberState,
    RestoreState,
    ArgsSize(u32),
}

impl From<gimli::write::CallFrameInstruction> for CallFrameInstruction {
    fn from(cfi: gimli::write::CallFrameInstruction) -> Self {
        use gimli::write::CallFrameInstruction;

        match cfi {
            CallFrameInstruction::Cfa(reg, offset) => Self::Cfa(reg.0, offset),
            CallFrameInstruction::CfaRegister(reg) => Self::CfaRegister(reg.0),
            CallFrameInstruction::CfaOffset(offset) => Self::CfaOffset(offset),
            CallFrameInstruction::CfaExpression(expr) => Self::CfaExpression(expr.0),
            CallFrameInstruction::Restore(reg) => Self::Restore(reg.0),
            CallFrameInstruction::Undefined(reg) => Self::Undefined(reg.0),
            CallFrameInstruction::SameValue(reg) => Self::SameValue(reg.0),
            CallFrameInstruction::Offset(reg, offset) => Self::Offset(reg.0, offset),
            CallFrameInstruction::ValOffset(reg, offset) => Self::ValOffset(reg.0, offset),
            CallFrameInstruction::Register(reg1, reg2) => Self::Register(reg1.0, reg2.0),
            CallFrameInstruction::Expression(reg, expr) => Self::Expression(reg.0, expr.0),
            CallFrameInstruction::ValExpression(reg, expr) => Self::ValExpression(reg.0, expr.0),
            CallFrameInstruction::RememberState => Self::RememberState,
            CallFrameInstruction::RestoreState => Self::RestoreState,
            CallFrameInstruction::ArgsSize(size) => Self::ArgsSize(size),
        }
    }
}

impl Into<gimli::write::CallFrameInstruction> for CallFrameInstruction {
    fn into(self) -> gimli::write::CallFrameInstruction {
        use gimli::{
            write::{CallFrameInstruction, Expression},
            Register,
        };

        match self {
            Self::Cfa(reg, offset) => CallFrameInstruction::Cfa(Register(reg), offset),
            Self::CfaRegister(reg) => CallFrameInstruction::CfaRegister(Register(reg)),
            Self::CfaOffset(offset) => CallFrameInstruction::CfaOffset(offset),
            Self::CfaExpression(expr) => CallFrameInstruction::CfaExpression(Expression(expr)),
            Self::Restore(reg) => CallFrameInstruction::Restore(Register(reg)),
            Self::Undefined(reg) => CallFrameInstruction::Undefined(Register(reg)),
            Self::SameValue(reg) => CallFrameInstruction::SameValue(Register(reg)),
            Self::Offset(reg, offset) => CallFrameInstruction::Offset(Register(reg), offset),
            Self::ValOffset(reg, offset) => CallFrameInstruction::ValOffset(Register(reg), offset),
            Self::Register(reg1, reg2) => {
                CallFrameInstruction::Register(Register(reg1), Register(reg2))
            }
            Self::Expression(reg, expr) => {
                CallFrameInstruction::Expression(Register(reg), Expression(expr))
            }
            Self::ValExpression(reg, expr) => {
                CallFrameInstruction::ValExpression(Register(reg), Expression(expr))
            }
            Self::RememberState => CallFrameInstruction::RememberState,
            Self::RestoreState => CallFrameInstruction::RestoreState,
            Self::ArgsSize(size) => CallFrameInstruction::ArgsSize(size),
        }
    }
}

/// Represents unwind information for a single System V ABI function.
///
/// This representation is not ISA specific.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct UnwindInfo {
    instructions: Vec<(u32, CallFrameInstruction)>,
    len: u32,
}

impl UnwindInfo {
    pub(crate) fn new(instructions: Vec<(u32, CallFrameInstruction)>, len: u32) -> Self {
        Self { instructions, len }
    }

    /// Converts the unwind information into a `FrameDescriptionEntry`.
    pub fn to_fde(&self, address: Address) -> gimli::write::FrameDescriptionEntry {
        let mut fde = FrameDescriptionEntry::new(address, self.len);

        for (offset, inst) in &self.instructions {
            fde.add_instruction(*offset, inst.clone().into());
        }

        fde
    }
}
