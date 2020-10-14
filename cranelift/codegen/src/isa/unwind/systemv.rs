//! System V ABI unwind information.

use crate::isa::{unwind::input, RegUnit};
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use gimli::write::{Address, FrameDescriptionEntry};
use thiserror::Error;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

type Register = u16;

/// Enumerate the errors possible in mapping Cranelift registers to their DWARF equivalent.
#[allow(missing_docs)]
#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegisterMappingError {
    #[error("unable to find bank for register info")]
    MissingBank,
    #[error("register mapping is currently only implemented for x86_64")]
    UnsupportedArchitecture,
    #[error("unsupported register bank: {0}")]
    UnsupportedRegisterBank(&'static str),
}

// This mirrors gimli's CallFrameInstruction, but is serializable
// This excludes CfaExpression, Expression, ValExpression due to
// https://github.com/gimli-rs/gimli/issues/513.
// TODO: if gimli ever adds serialization support, remove this type
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub(crate) enum CallFrameInstruction {
    Cfa(Register, i32),
    CfaRegister(Register),
    CfaOffset(i32),
    Restore(Register),
    Undefined(Register),
    SameValue(Register),
    Offset(Register, i32),
    ValOffset(Register, i32),
    Register(Register, Register),
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
            CallFrameInstruction::Restore(reg) => Self::Restore(reg.0),
            CallFrameInstruction::Undefined(reg) => Self::Undefined(reg.0),
            CallFrameInstruction::SameValue(reg) => Self::SameValue(reg.0),
            CallFrameInstruction::Offset(reg, offset) => Self::Offset(reg.0, offset),
            CallFrameInstruction::ValOffset(reg, offset) => Self::ValOffset(reg.0, offset),
            CallFrameInstruction::Register(reg1, reg2) => Self::Register(reg1.0, reg2.0),
            CallFrameInstruction::RememberState => Self::RememberState,
            CallFrameInstruction::RestoreState => Self::RestoreState,
            CallFrameInstruction::ArgsSize(size) => Self::ArgsSize(size),
            _ => {
                // Cranelift's unwind support does not generate `CallFrameInstruction`s with
                // Expression at this moment, and it is not trivial to
                // serialize such instructions.
                panic!("CallFrameInstruction with Expression not supported");
            }
        }
    }
}

impl Into<gimli::write::CallFrameInstruction> for CallFrameInstruction {
    fn into(self) -> gimli::write::CallFrameInstruction {
        use gimli::{write::CallFrameInstruction, Register};

        match self {
            Self::Cfa(reg, offset) => CallFrameInstruction::Cfa(Register(reg), offset),
            Self::CfaRegister(reg) => CallFrameInstruction::CfaRegister(Register(reg)),
            Self::CfaOffset(offset) => CallFrameInstruction::CfaOffset(offset),
            Self::Restore(reg) => CallFrameInstruction::Restore(Register(reg)),
            Self::Undefined(reg) => CallFrameInstruction::Undefined(Register(reg)),
            Self::SameValue(reg) => CallFrameInstruction::SameValue(Register(reg)),
            Self::Offset(reg, offset) => CallFrameInstruction::Offset(Register(reg), offset),
            Self::ValOffset(reg, offset) => CallFrameInstruction::ValOffset(Register(reg), offset),
            Self::Register(reg1, reg2) => {
                CallFrameInstruction::Register(Register(reg1), Register(reg2))
            }
            Self::RememberState => CallFrameInstruction::RememberState,
            Self::RestoreState => CallFrameInstruction::RestoreState,
            Self::ArgsSize(size) => CallFrameInstruction::ArgsSize(size),
        }
    }
}

/// Maps UnwindInfo register to gimli's index space.
pub(crate) trait RegisterMapper {
    /// Maps RegUnit.
    fn map(&self, reg: RegUnit) -> Result<Register, RegisterMappingError>;
    /// Gets RSP in gimli's index space.
    fn rsp(&self) -> Register;
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
    pub(crate) fn build<'b>(
        unwind: input::UnwindInfo<RegUnit>,
        word_size: u8,
        frame_register: Option<RegUnit>,
        map_reg: &'b dyn RegisterMapper,
    ) -> CodegenResult<Self> {
        use input::UnwindCode;
        let mut builder = InstructionBuilder::new(word_size, frame_register, map_reg);

        for c in unwind.prologue_unwind_codes.iter().chain(
            unwind
                .epilogues_unwind_codes
                .iter()
                .map(|c| c.iter())
                .flatten(),
        ) {
            match c {
                UnwindCode::SaveRegister { offset, reg } => {
                    builder
                        .push_reg(*offset, *reg)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::StackAlloc { offset, size } => {
                    builder.adjust_sp_down_imm(*offset, *size as i64);
                }
                UnwindCode::StackDealloc { offset, size } => {
                    builder.adjust_sp_up_imm(*offset, *size as i64);
                }
                UnwindCode::RestoreRegister { offset, reg } => {
                    builder
                        .pop_reg(*offset, *reg)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::SetFramePointer { offset, reg } => {
                    builder
                        .set_cfa_reg(*offset, *reg)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::RememberState { offset } => {
                    builder.remember_state(*offset);
                }
                UnwindCode::RestoreState { offset } => {
                    builder.restore_state(*offset);
                }
                _ => {}
            }
        }

        let instructions = builder.instructions;
        let len = unwind.function_size;

        Ok(Self { instructions, len })
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

struct InstructionBuilder<'a> {
    word_size: u8,
    cfa_offset: i32,
    saved_state: Option<i32>,
    frame_register: Option<RegUnit>,
    map_reg: &'a dyn RegisterMapper,
    instructions: Vec<(u32, CallFrameInstruction)>,
}

impl<'a> InstructionBuilder<'a> {
    fn new(
        word_size: u8,
        frame_register: Option<RegUnit>,
        map_reg: &'a (dyn RegisterMapper + 'a),
    ) -> Self {
        Self {
            word_size,
            cfa_offset: word_size as i32, // CFA offset starts at word size offset to account for the return address on stack
            saved_state: None,
            frame_register,
            map_reg,
            instructions: Vec::new(),
        }
    }

    fn push_reg(&mut self, offset: u32, reg: RegUnit) -> Result<(), RegisterMappingError> {
        self.cfa_offset += self.word_size as i32;
        // Update the CFA if this is the save of the frame pointer register or if a frame pointer isn't being used
        // When using a frame pointer, we only need to update the CFA to account for the push of the frame pointer itself
        if match self.frame_register {
            Some(fp) => reg == fp,
            None => true,
        } {
            self.instructions
                .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
        }

        // Pushes in the prologue are register saves, so record an offset of the save
        self.instructions.push((
            offset,
            CallFrameInstruction::Offset(self.map_reg.map(reg)?, -self.cfa_offset),
        ));

        Ok(())
    }

    fn adjust_sp_down_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.cfa_offset += imm as i32;
        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
    }

    fn adjust_sp_up_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.cfa_offset -= imm as i32;
        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
    }

    fn set_cfa_reg(&mut self, offset: u32, reg: RegUnit) -> Result<(), RegisterMappingError> {
        self.instructions.push((
            offset,
            CallFrameInstruction::CfaRegister(self.map_reg.map(reg)?),
        ));
        Ok(())
    }

    fn pop_reg(&mut self, offset: u32, reg: RegUnit) -> Result<(), RegisterMappingError> {
        self.cfa_offset -= self.word_size as i32;

        // Update the CFA if this is the restore of the frame pointer register or if a frame pointer isn't being used
        match self.frame_register {
            Some(fp) => {
                if reg == fp {
                    self.instructions.push((
                        offset,
                        CallFrameInstruction::Cfa(self.map_reg.rsp(), self.cfa_offset),
                    ));
                }
            }
            None => {
                self.instructions
                    .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));

                // Pops in the epilogue are register restores, so record a "same value" for the register
                // This isn't necessary when using a frame pointer as the CFA doesn't change for CSR restores
                self.instructions.push((
                    offset,
                    CallFrameInstruction::SameValue(self.map_reg.map(reg)?),
                ));
            }
        };

        Ok(())
    }

    fn remember_state(&mut self, offset: u32) {
        self.saved_state = Some(self.cfa_offset);

        self.instructions
            .push((offset, CallFrameInstruction::RememberState));
    }

    fn restore_state(&mut self, offset: u32) {
        let cfa_offset = self.saved_state.take().unwrap();
        self.cfa_offset = cfa_offset;

        self.instructions
            .push((offset, CallFrameInstruction::RestoreState));
    }
}
