//! Unwind information for System V ABI (x86-64).

use crate::isa::unwind::systemv::{CallFrameInstruction, RegisterMappingError, UnwindInfo};
use crate::isa::x64::inst::{
    args::{AluRmiROpcode, RegMemImm},
    regs, Inst,
};
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register, X86_64};
use regalloc::{Reg, RegClass};
use std::boxed::Box;

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
    instructions: Vec<(u32, CallFrameInstruction)>,
}

impl InstructionBuilder {
    fn new(frame_register: Option<Reg>) -> Self {
        Self {
            cfa_offset: 8, // CFA offset starts at 8 to account to return address on stack
            frame_register,
            instructions: Vec::new(),
        }
    }

    fn push_reg(&mut self, offset: u32, arg: &RegMemImm) -> Result<(), RegisterMappingError> {
        self.cfa_offset += 8;

        let reg = *match arg {
            RegMemImm::Reg { reg } => reg,
            _ => {
                panic!();
            }
        };

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
            CallFrameInstruction::Offset(map_reg(reg)?.0, -self.cfa_offset),
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
    insts_layout: &[(u32, u32)],
    prologue_epilogue: &(u32, u32, Box<[u32]>),
    frame_register: Option<Reg>,
) -> CodegenResult<Option<UnwindInfo>> {
    // Only System V-like calling conventions are supported
    // match func.signature.call_conv {
    //     CallConv::Fast | CallConv::Cold | CallConv::SystemV => {}
    //     _ => return Ok(None),
    // }

    // if func.prologue_end.is_none() || isa.name() != "x86" || isa.pointer_bits() != 64 {
    //     return Ok(None);
    // }

    let mut layout = insts_layout
        .iter()
        .map(|(i, j)| (*i as usize, *j))
        .collect::<Vec<(usize, u32)>>();
    layout.sort();
    let len = layout.last().unwrap().1;

    let mut builder = InstructionBuilder::new(frame_register);

    let mut layout_index = 0;

    let prologue_start = prologue_epilogue.0 as usize;
    let prologue_end = prologue_epilogue.1 as usize;
    for i in prologue_start..prologue_end {
        let inst = &insts[i];

        while layout_index < layout.len() && layout[layout_index].0 < i {
            layout_index += 1;
        }
        let offset = layout[layout_index].1; // TODO protect layout_index oob

        match inst {
            Inst::Push64 { src } => {
                builder
                    .push_reg(offset, src)
                    .map_err(CodegenError::RegisterMappingError)?;
            }
            Inst::Mov_R_R { src, dst, .. } => {
                builder
                    .move_reg(offset, *src, dst.to_reg())
                    .map_err(CodegenError::RegisterMappingError)?;
            }
            Inst::Alu_RMI_R {
                is_64: true,
                op: AluRmiROpcode::Sub,
                src: RegMemImm::Imm { simm32 },
                dst,
                ..
            } if dst.to_reg() == regs::rsp() => {
                let imm = *simm32 as i32;
                builder.adjust_sp_down_imm(offset, imm.into());
            }
            Inst::Alu_RMI_R {
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

    for (j, epilogue_point) in prologue_epilogue.2.iter().enumerate() {
        let i = *epilogue_point as usize;
        while layout_index < layout.len() && layout[layout_index].0 < i {
            layout_index += 1;
        }
        let offset = layout[layout_index].1; // TODO protect layout_index oob

        if j & 1 == 0 {
            builder.remember_state(offset);
        } else {
            builder.restore_state(offset);
        }
    }

    Ok(Some(UnwindInfo::new(builder.instructions, len)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{
        types, AbiParam, ExternalName, InstBuilder, Signature, StackSlotData, StackSlotKind,
    };
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use gimli::write::Address;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    #[cfg_attr(feature = "x64", should_panic)] // TODO #2079
    fn test_simple_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let fde = match isa
            .create_unwind_info(&context.func)
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(1234))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(1234), length: 16, lsda: None, instructions: [(2, CfaOffset(16)), (2, Offset(Register(6), -16)), (5, CfaRegister(Register(6))), (15, Cfa(Register(7), 8))] }");
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
    #[cfg_attr(feature = "x64", should_panic)] // TODO #2079
    fn test_multi_return_func() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_multi_return_function(CallConv::SystemV));

        context.compile(&*isa).expect("expected compilation");

        let fde = match isa
            .create_unwind_info(&context.func)
            .expect("can create unwind info")
        {
            Some(crate::isa::unwind::UnwindInfo::SystemV(info)) => {
                info.to_fde(Address::Constant(4321))
            }
            _ => panic!("expected unwind information"),
        };

        assert_eq!(format!("{:?}", fde), "FrameDescriptionEntry { address: Constant(4321), length: 16, lsda: None, instructions: [(2, CfaOffset(16)), (2, Offset(Register(6), -16)), (5, CfaRegister(Register(6))), (12, RememberState), (12, Cfa(Register(7), 8)), (13, RestoreState), (15, Cfa(Register(7), 0))] }");
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
