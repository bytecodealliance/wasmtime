//! Unwind information for System V ABI (x86-64).

use crate::isa::unwind::systemv::{CallFrameInstruction, RegisterMappingError, UnwindInfo};
use crate::isa::x64::inst::{
    args::{AluRmiROpcode, Amode, RegMemImm, SyntheticAmode},
    regs, Inst,
};
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register, X86_64};
use regalloc::{Reg, RegClass};
use std::boxed::Box;
use std::collections::HashSet;

/// Creates a new x86-64 common information entry (CIE).
pub fn create_cie() -> CommonInformationEntry {
    use gimli::write::CallFrameInstruction;

    let mut entry = CommonInformationEntry::new(
        Encoding {
            address_size: 8,
            format: Format::Dwarf32,
            version: 1,
        },
        1,  // Code alignment factor
        -8, // Data alignment factor
        X86_64::RA,
    );

    // Every frame will start with the call frame address (CFA) at RSP+8
    // It is +8 to account for the push of the return address by the call instruction
    entry.add_instruction(CallFrameInstruction::Cfa(X86_64::RSP, 8));

    // Every frame will start with the return address at RSP (CFA-8 = RSP+8-8 = RSP)
    entry.add_instruction(CallFrameInstruction::Offset(X86_64::RA, -8));

    entry
}

/// Map Cranelift registers to their corresponding Gimli registers.
pub fn map_reg(reg: Reg) -> Result<Register, RegisterMappingError> {
    // Mapping from https://github.com/bytecodealliance/cranelift/pull/902 by @iximeow
    const X86_GP_REG_MAP: [gimli::Register; 16] = [
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
    const X86_XMM_REG_MAP: [gimli::Register; 16] = [
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

    match reg.get_class() {
        RegClass::I64 => {
            // x86 GP registers have a weird mapping to DWARF registers, so we use a
            // lookup table.
            Ok(X86_GP_REG_MAP[reg.get_hw_encoding() as usize])
        }
        RegClass::V128 => Ok(X86_XMM_REG_MAP[reg.get_hw_encoding() as usize]),
        _ => Err(RegisterMappingError::UnsupportedRegisterBank("class?")),
    }
}

struct InstructionBuilder {
    cfa_offset: i32,
    frame_register: Option<Reg>,
    saved_registers: HashSet<Reg>,
    instructions: Vec<(u32, CallFrameInstruction)>,
}

impl InstructionBuilder {
    fn new(frame_register: Option<Reg>) -> Self {
        Self {
            cfa_offset: 8, // CFA offset starts at 8 to account to return address on stack
            frame_register,
            saved_registers: HashSet::new(),
            instructions: Vec::new(),
        }
    }

    fn push_reg(&mut self, offset: u32, reg: Reg) -> Result<(), RegisterMappingError> {
        self.cfa_offset += 8;

        // Update the CFA if this is the save of the frame pointer register or if a frame pointer isn't being used
        // When using a frame pointer, we only need to update the CFA to account for the push of the frame pointer itself
        if match self.frame_register {
            Some(fp) => reg == fp,
            None => true,
        } {
            self.instructions
                .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
        }
        self.store_reg_at(offset, 0, reg)
    }

    fn store_reg_at(
        &mut self,
        offset: u32,
        pos: u32,
        reg: Reg,
    ) -> Result<(), RegisterMappingError> {
        if self.saved_registers.contains(&reg) {
            // Already saved the register on stack.
            return Ok(());
        }

        // Pushes in the prologue are register saves, so record an offset of the save
        self.instructions.push((
            offset,
            CallFrameInstruction::Offset(map_reg(reg)?.0, pos as i32 - self.cfa_offset),
        ));
        self.saved_registers.insert(reg);

        Ok(())
    }

    fn adjust_sp_down_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        self.cfa_offset += imm as i32;

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
    }

    fn adjust_sp_up_imm(&mut self, offset: u32, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);

        self.cfa_offset -= imm as i32;

        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
    }

    fn move_reg(&mut self, offset: u32, src: Reg, dst: Reg) -> Result<(), RegisterMappingError> {
        if let Some(fp) = self.frame_register {
            // Check for change in CFA register (RSP is always the starting CFA)
            if src == regs::rsp() && dst == fp {
                self.instructions
                    .push((offset, CallFrameInstruction::CfaRegister(map_reg(dst)?.0)));
            }
        }

        Ok(())
    }

    fn remember_state(&mut self, offset: u32) {
        self.instructions
            .push((offset, CallFrameInstruction::RememberState));
    }

    fn restore_state(&mut self, offset: u32) {
        self.instructions
            .push((offset, CallFrameInstruction::RestoreState));
    }
}

pub(crate) fn create_unwind_info(
    insts: &[Inst],
    insts_layout: &[u32],
    len: u32,
    prologue_epilogue: &(u32, u32, Box<[(u32, u32)]>),
    frame_register: Option<Reg>,
) -> CodegenResult<Option<UnwindInfo>> {
    let mut builder = InstructionBuilder::new(frame_register);

    let prologue_start = prologue_epilogue.0 as usize;
    let prologue_end = prologue_epilogue.1 as usize;
    for i in prologue_start..prologue_end {
        let inst = &insts[i];
        let offset = insts_layout[i];

        // TODO sub and `mov reg, imm(rsp)`
        match inst {
            Inst::Push64 {
                src: RegMemImm::Reg { reg },
            } => {
                builder
                    .push_reg(offset, *reg)
                    .map_err(CodegenError::RegisterMappingError)?;
            }
            Inst::MovRR { src, dst, .. } => {
                builder
                    .move_reg(offset, *src, dst.to_reg())
                    .map_err(CodegenError::RegisterMappingError)?;
            }
            Inst::AluRmiR {
                is_64: true,
                op: AluRmiROpcode::Sub,
                src: RegMemImm::Imm { simm32 },
                dst,
                ..
            } if dst.to_reg() == regs::rsp() => {
                let imm = *simm32 as i32;
                builder.adjust_sp_down_imm(offset, imm.into());
            }
            Inst::MovRM {
                src,
                dst: SyntheticAmode::Real(Amode::ImmReg { simm32, base }),
                ..
            } if *base == regs::rsp() => {
                // `mov reg, imm(rsp)` -- similar to push
                builder
                    .store_reg_at(offset, *simm32, *src)
                    .map_err(CodegenError::RegisterMappingError)?;
            }
            Inst::AluRmiR {
                is_64: true,
                op: AluRmiROpcode::Add,
                src: RegMemImm::Imm { simm32 },
                dst,
                ..
            } if dst.to_reg() == regs::rsp() => {
                let imm = *simm32 as i32;
                builder.adjust_sp_up_imm(offset, imm.into());
            }
            _ => {}
        }
    }

    for (epilogue_start, epilogue_end) in prologue_epilogue.2.iter() {
        let i = *epilogue_start as usize;
        let offset = insts_layout[i];
        builder.remember_state(offset);

        let i = *epilogue_end as usize;
        let offset = insts_layout[i];
        builder.restore_state(offset);
    }

    Ok(Some(UnwindInfo::new(builder.instructions, len)))
}

#[cfg(test)]
mod tests {
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        types, AbiParam, ExternalName, Function, InstBuilder, Signature, StackSlotData,
        StackSlotKind,
    };
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use gimli::write::Address;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    fn test_simple_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let fde = match context
            .create_unwind_info(isa.as_ref())
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(1234))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(1234), length: 13, lsda: None, instructions: [(1, CfaOffset(16)), (1, Offset(Register(6), -16)), (4, CfaRegister(Register(6)))] }");
    }

    fn create_function(call_conv: CallConv, stack_slot: Option<StackSlotData>) -> Function {
        let mut func =
            Function::with_name_signature(ExternalName::user(0, 0), Signature::new(call_conv));

        let block0 = func.dfg.make_block();
        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(block0);
        pos.ins().return_(&[]);

        if let Some(stack_slot) = stack_slot {
            func.stack_slots.push(stack_slot);
        }

        func
    }

    #[test]
    //#[cfg_attr(feature = "x64", should_panic)] // TODO #2079
    fn test_multi_return_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_multi_return_function(CallConv::SystemV));

        context.compile(&*isa).expect("expected compilation");

        let fde = match context
            .create_unwind_info(isa.as_ref())
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(4321))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(4321), length: 23, lsda: None, instructions: [(1, CfaOffset(16)), (1, Offset(Register(6), -16)), (4, CfaRegister(Register(6))), (16, RememberState), (21, RestoreState)] }");
    }

    fn create_multi_return_function(call_conv: CallConv) -> Function {
        let mut sig = Signature::new(call_conv);
        sig.params.push(AbiParam::new(types::I32));
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);

        let block0 = func.dfg.make_block();
        let v0 = func.dfg.append_block_param(block0, types::I32);
        let block1 = func.dfg.make_block();
        let block2 = func.dfg.make_block();

        let mut pos = FuncCursor::new(&mut func);
        pos.insert_block(block0);
        pos.ins().brnz(v0, block2, &[]);
        pos.ins().jump(block1, &[]);

        pos.insert_block(block1);
        pos.ins().return_(&[]);

        pos.insert_block(block2);
        pos.ins().return_(&[]);

        func
    }
}
