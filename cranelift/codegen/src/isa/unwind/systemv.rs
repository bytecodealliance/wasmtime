//! System V ABI unwind information.

use crate::isa::unwind::input;
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
pub(crate) trait RegisterMapper<Reg> {
    /// Maps Reg.
    fn map(&self, reg: Reg) -> Result<Register, RegisterMappingError>;
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
    pub(crate) fn build<'b, Reg: PartialEq + Copy>(
        unwind: input::UnwindInfo<Reg>,
        map_reg: &'b dyn RegisterMapper<Reg>,
    ) -> CodegenResult<Self> {
        use input::UnwindCode;
        let mut builder = InstructionBuilder::new(unwind.word_size, map_reg);

        for c in unwind.prologue_unwind_codes.iter().chain(
            unwind
                .epilogues_unwind_codes
                .iter()
                .map(|c| c.iter())
                .flatten(),
        ) {
            match c {
                UnwindCode::SaveRegister {
                    offset,
                    reg,
                    stack_offset: 0,
                } => {
                    builder
                        .save_reg(*offset, *reg)
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
                        .restore_reg(*offset, *reg)
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

struct InstructionBuilder<'a, Reg: PartialEq + Copy> {
    sp_offset: i32,
    frame_register: Option<Reg>,
    saved_state: Option<(i32, Option<Reg>)>,
    map_reg: &'a dyn RegisterMapper<Reg>,
    instructions: Vec<(u32, CallFrameInstruction)>,
}

impl<'a, Reg: PartialEq + Copy> InstructionBuilder<'a, Reg> {
    fn new(word_size: u8, map_reg: &'a (dyn RegisterMapper<Reg> + 'a)) -> Self {
        Self {
            sp_offset: word_size as i32, // CFA offset starts at word size offset to account for the return address on stack
            saved_state: None,
            frame_register: None,
            map_reg,
            instructions: Vec::new(),
        }
    }

    fn save_reg(&mut self, offset: u32, reg: Reg) -> Result<(), RegisterMappingError> {
        // Pushes in the prologue are register saves, so record an offset of the save
        self.instructions.push((
            offset,
            CallFrameInstruction::Offset(self.map_reg.map(reg)?, -self.sp_offset),
        ));

        Ok(())
    }

    fn adjust_sp_down_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        self.sp_offset += imm as i32;

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.sp_offset)));
    }

    fn adjust_sp_up_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        self.sp_offset -= imm as i32;

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        let cfa_inst_ofs = {
            // Scan to find and merge with CFA instruction with the same offset.
            let mut it = self.instructions.iter_mut();
            loop {
                match it.next_back() {
                    Some((i_offset, i)) if *i_offset == offset => {
                        if let CallFrameInstruction::Cfa(_, o) = i {
                            break Some(o);
                        }
                    }
                    _ => {
                        break None;
                    }
                }
            }
        };

        if let Some(o) = cfa_inst_ofs {
            // Update previous CFA instruction.
            *o = self.sp_offset;
        } else {
            // Add just CFA offset instruction.
            self.instructions
                .push((offset, CallFrameInstruction::CfaOffset(self.sp_offset)));
        }
    }

    fn set_cfa_reg(&mut self, offset: u32, reg: Reg) -> Result<(), RegisterMappingError> {
        self.instructions.push((
            offset,
            CallFrameInstruction::CfaRegister(self.map_reg.map(reg)?),
        ));
        self.frame_register = Some(reg);
        Ok(())
    }

    fn restore_reg(&mut self, offset: u32, reg: Reg) -> Result<(), RegisterMappingError> {
        // Update the CFA if this is the restore of the frame pointer register.
        if Some(reg) == self.frame_register {
            self.frame_register = None;
            self.instructions.push((
                offset,
                CallFrameInstruction::Cfa(self.map_reg.rsp(), self.sp_offset),
            ));
        }
        // Pops in the epilogue are register restores, so record a "same value" for the register
        self.instructions.push((
            offset,
            CallFrameInstruction::SameValue(self.map_reg.map(reg)?),
        ));

        Ok(())
    }

    fn remember_state(&mut self, offset: u32) {
        self.saved_state = Some((self.sp_offset, self.frame_register));

        self.instructions
            .push((offset, CallFrameInstruction::RememberState));
    }

    fn restore_state(&mut self, offset: u32) {
        let (sp_offset, frame_register) = self.saved_state.take().unwrap();
        self.sp_offset = sp_offset;
        self.frame_register = frame_register;

        self.instructions
            .push((offset, CallFrameInstruction::RestoreState));
    }
}
