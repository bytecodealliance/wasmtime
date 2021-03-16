//! System V ABI unwind information.

use crate::binemit::CodeOffset;
use crate::isa::unwind::input;
use crate::isa::unwind::UnwindInst;
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use gimli::write::{Address, FrameDescriptionEntry};

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

type Register = u16;

/// Enumerate the errors possible in mapping Cranelift registers to their DWARF equivalent.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Eq)]
pub enum RegisterMappingError {
    MissingBank,
    UnsupportedArchitecture,
    UnsupportedRegisterBank(&'static str),
}

impl std::error::Error for RegisterMappingError {}

impl std::fmt::Display for RegisterMappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RegisterMappingError::MissingBank => write!(f, "unable to find bank for register info"),
            RegisterMappingError::UnsupportedArchitecture => write!(
                f,
                "register mapping is currently only implemented for x86_64"
            ),
            RegisterMappingError::UnsupportedRegisterBank(bank) => {
                write!(f, "unsupported register bank: {}", bank)
            }
        }
    }
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
    /// Enables or disables pointer authentication on aarch64 platforms post ARMv8.3.  This
    /// particular item maps to gimli::ValExpression(RA_SIGN_STATE, lit0/lit1).
    Aarch64SetPointerAuth {
        return_addresses: bool,
    },
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
        use gimli::{write::CallFrameInstruction, write::Expression, Register};

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
            Self::Aarch64SetPointerAuth { return_addresses } => {
                // To enable pointer authentication for return addresses in dwarf directives, we
                // use a small dwarf expression that sets the value of the pseudo-register
                // RA_SIGN_STATE (RA stands for return address) to 0 or 1. This behavior is
                // documented in
                // https://github.com/ARM-software/abi-aa/blob/master/aadwarf64/aadwarf64.rst#41dwarf-register-names.
                let mut expr = Expression::new();
                expr.op(if return_addresses {
                    gimli::DW_OP_lit1
                } else {
                    gimli::DW_OP_lit0
                });
                const RA_SIGN_STATE: Register = Register(34);
                CallFrameInstruction::ValExpression(RA_SIGN_STATE, expr)
            }
        }
    }
}

/// Maps UnwindInfo register to gimli's index space.
pub(crate) trait RegisterMapper<Reg> {
    /// Maps Reg.
    fn map(&self, reg: Reg) -> Result<Register, RegisterMappingError>;
    /// Gets stack pointer register.
    fn sp(&self) -> Register;
    /// Gets the frame pointer register, if any.
    fn fp(&self) -> Option<Register> {
        None
    }
    /// Gets the link register, if any.
    fn lr(&self) -> Option<Register> {
        None
    }
    /// What is the offset from saved FP to saved LR?
    fn lr_offset(&self) -> Option<u32> {
        None
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

pub(crate) fn create_unwind_info_from_insts<MR: RegisterMapper<regalloc::Reg>>(
    insts: &[(CodeOffset, UnwindInst)],
    code_len: usize,
    mr: &MR,
) -> CodegenResult<UnwindInfo> {
    let mut instructions = vec![];

    let mut cfa_offset = 0;
    let mut clobber_offset_to_cfa = 0;
    for &(instruction_offset, ref inst) in insts {
        match inst {
            &UnwindInst::PushFrameRegs {
                offset_upward_to_caller_sp,
            } => {
                // Define CFA in terms of current SP (SP changed and we haven't
                // set FP yet).
                instructions.push((
                    instruction_offset,
                    CallFrameInstruction::CfaOffset(offset_upward_to_caller_sp as i32),
                ));
                // Note that we saved the old FP value on the stack.  Use of this
                // operation implies that the target defines a FP register.
                instructions.push((
                    instruction_offset,
                    CallFrameInstruction::Offset(
                        mr.fp().unwrap(),
                        -(offset_upward_to_caller_sp as i32),
                    ),
                ));
                // If there is a link register on this architecture, note that
                // we saved it as well.
                if let Some(lr) = mr.lr() {
                    instructions.push((
                        instruction_offset,
                        CallFrameInstruction::Offset(
                            lr,
                            -(offset_upward_to_caller_sp as i32)
                                + mr.lr_offset().expect("LR offset not provided") as i32,
                        ),
                    ));
                }
            }
            &UnwindInst::DefineNewFrame {
                offset_upward_to_caller_sp,
                offset_downward_to_clobbers,
            } => {
                // Define CFA in terms of FP. Note that we assume it was already
                // defined correctly in terms of the current SP, and FP has just
                // been set to the current SP, so we do not need to change the
                // offset, only the register.  (This is done only if the target
                // defines a frame pointer register.)
                if let Some(fp) = mr.fp() {
                    instructions.push((instruction_offset, CallFrameInstruction::CfaRegister(fp)));
                }
                // Record initial CFA offset.  This will be used with later
                // StackAlloc calls if we do not have a frame pointer.
                cfa_offset = offset_upward_to_caller_sp;
                // Record distance from CFA downward to clobber area so we can
                // express clobber offsets later in terms of CFA.
                clobber_offset_to_cfa = offset_upward_to_caller_sp + offset_downward_to_clobbers;
            }
            &UnwindInst::StackAlloc { size } => {
                // If we do not use a frame pointer, we need to update the
                // CFA offset whenever the stack pointer changes.
                if mr.fp().is_none() {
                    cfa_offset += size;
                    instructions.push((
                        instruction_offset,
                        CallFrameInstruction::CfaOffset(cfa_offset as i32),
                    ));
                }
            }
            &UnwindInst::SaveReg {
                clobber_offset,
                reg,
            } => {
                let reg = mr
                    .map(reg.to_reg())
                    .map_err(|e| CodegenError::RegisterMappingError(e))?;
                let off = (clobber_offset as i32) - (clobber_offset_to_cfa as i32);
                instructions.push((instruction_offset, CallFrameInstruction::Offset(reg, off)));
            }
            &UnwindInst::Aarch64SetPointerAuth { return_addresses } => {
                instructions.push((
                    instruction_offset,
                    CallFrameInstruction::Aarch64SetPointerAuth { return_addresses },
                ));
            }
        }
    }

    Ok(UnwindInfo {
        instructions,
        len: code_len as u32,
    })
}

impl UnwindInfo {
    // TODO: remove `build()` below when old backend is removed. The new backend uses a simpler
    // approach in `create_unwind_info_from_insts()` above.

    pub(crate) fn build<'b, Reg: PartialEq + Copy>(
        unwind: input::UnwindInfo<Reg>,
        map_reg: &'b dyn RegisterMapper<Reg>,
    ) -> CodegenResult<Self> {
        use input::UnwindCode;
        let mut builder = InstructionBuilder::new(unwind.initial_sp_offset, map_reg);

        for (offset, c) in unwind.prologue_unwind_codes.iter().chain(
            unwind
                .epilogues_unwind_codes
                .iter()
                .map(|c| c.iter())
                .flatten(),
        ) {
            match c {
                UnwindCode::SaveRegister { reg, stack_offset } => {
                    builder
                        .save_reg(*offset, *reg, *stack_offset)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::StackAlloc { size } => {
                    builder.adjust_sp_down_imm(*offset, *size as i64);
                }
                UnwindCode::StackDealloc { size } => {
                    builder.adjust_sp_up_imm(*offset, *size as i64);
                }
                UnwindCode::RestoreRegister { reg } => {
                    builder
                        .restore_reg(*offset, *reg)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::SetFramePointer { reg } => {
                    builder
                        .set_cfa_reg(*offset, *reg)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                UnwindCode::RestoreFramePointer => {
                    builder.restore_cfa(*offset);
                }
                UnwindCode::RememberState => {
                    builder.remember_state(*offset);
                }
                UnwindCode::RestoreState => {
                    builder.restore_state(*offset);
                }
                UnwindCode::Aarch64SetPointerAuth { return_addresses } => {
                    builder.set_aarch64_pauth(*offset, *return_addresses);
                }
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

// TODO: delete the builder below when the old backend is removed.

struct InstructionBuilder<'a, Reg: PartialEq + Copy> {
    sp_offset: i32,
    frame_register: Option<Reg>,
    saved_state: Option<(i32, Option<Reg>)>,
    map_reg: &'a dyn RegisterMapper<Reg>,
    instructions: Vec<(u32, CallFrameInstruction)>,
}

impl<'a, Reg: PartialEq + Copy> InstructionBuilder<'a, Reg> {
    fn new(sp_offset: u8, map_reg: &'a (dyn RegisterMapper<Reg> + 'a)) -> Self {
        Self {
            sp_offset: sp_offset as i32, // CFA offset starts at the specified offset to account for the return address on stack
            saved_state: None,
            frame_register: None,
            map_reg,
            instructions: Vec::new(),
        }
    }

    fn save_reg(
        &mut self,
        offset: u32,
        reg: Reg,
        stack_offset: u32,
    ) -> Result<(), RegisterMappingError> {
        // Pushes in the prologue are register saves, so record an offset of the save
        self.instructions.push((
            offset,
            CallFrameInstruction::Offset(
                self.map_reg.map(reg)?,
                stack_offset as i32 - self.sp_offset,
            ),
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

    fn restore_cfa(&mut self, offset: u32) {
        // Restore SP and its offset.
        self.instructions.push((
            offset,
            CallFrameInstruction::Cfa(self.map_reg.sp(), self.sp_offset),
        ));
        self.frame_register = None;
    }

    fn restore_reg(&mut self, offset: u32, reg: Reg) -> Result<(), RegisterMappingError> {
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

    fn set_aarch64_pauth(&mut self, offset: u32, return_addresses: bool) {
        self.instructions.push((
            offset,
            CallFrameInstruction::Aarch64SetPointerAuth { return_addresses },
        ));
    }
}
