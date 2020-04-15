//! Unwind information for System V ABI (x86-64).

use crate::ir::{Function, Inst, InstructionData, Opcode, Value};
use crate::isa::{
    unwind::systemv::{CallFrameInstruction, RegisterMappingError, UnwindInfo},
    x86::registers::RU,
    CallConv, RegUnit, TargetIsa,
};
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use gimli::{write::CommonInformationEntry, Encoding, Format, Register, X86_64};

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
pub fn map_reg(isa: &dyn TargetIsa, reg: RegUnit) -> Result<Register, RegisterMappingError> {
    if isa.name() != "x86" || isa.pointer_bits() != 64 {
        return Err(RegisterMappingError::UnsupportedArchitecture);
    }

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

    let reg_info = isa.register_info();
    let bank = reg_info
        .bank_containing_regunit(reg)
        .ok_or_else(|| RegisterMappingError::MissingBank)?;
    match bank.name {
        "IntRegs" => {
            // x86 GP registers have a weird mapping to DWARF registers, so we use a
            // lookup table.
            Ok(X86_GP_REG_MAP[(reg - bank.first_unit) as usize])
        }
        "FloatRegs" => Ok(X86_XMM_REG_MAP[(reg - bank.first_unit) as usize]),
        _ => Err(RegisterMappingError::UnsupportedRegisterBank(bank.name)),
    }
}

struct InstructionBuilder<'a> {
    func: &'a Function,
    isa: &'a dyn TargetIsa,
    cfa_offset: i32,
    frame_register: Option<RegUnit>,
    instructions: Vec<(u32, CallFrameInstruction)>,
    stack_size: Option<i32>,
    epilogue_pop_offsets: Vec<u32>,
}

impl<'a> InstructionBuilder<'a> {
    fn new(func: &'a Function, isa: &'a dyn TargetIsa, frame_register: Option<RegUnit>) -> Self {
        Self {
            func,
            isa,
            cfa_offset: 8, // CFA offset starts at 8 to account to return address on stack
            frame_register,
            instructions: Vec::new(),
            stack_size: None,
            epilogue_pop_offsets: Vec::new(),
        }
    }

    fn push_reg(&mut self, offset: u32, arg: Value) -> Result<(), RegisterMappingError> {
        self.cfa_offset += 8;

        let reg = self.func.locations[arg].unwrap_reg();

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
            CallFrameInstruction::Offset(map_reg(self.isa, reg)?.0, -self.cfa_offset),
        ));

        Ok(())
    }

    fn adjust_sp_down(&mut self, offset: u32) {
        // Don't adjust the CFA if we're using a frame pointer
        if self.frame_register.is_some() {
            return;
        }

        self.cfa_offset += self
            .stack_size
            .expect("expected a previous stack size instruction");
        self.instructions
            .push((offset, CallFrameInstruction::CfaOffset(self.cfa_offset)));
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

    fn move_reg(
        &mut self,
        offset: u32,
        src: RegUnit,
        dst: RegUnit,
    ) -> Result<(), RegisterMappingError> {
        if let Some(fp) = self.frame_register {
            // Check for change in CFA register (RSP is always the starting CFA)
            if src == (RU::rsp as RegUnit) && dst == fp {
                self.instructions.push((
                    offset,
                    CallFrameInstruction::CfaRegister(map_reg(self.isa, dst)?.0),
                ));
            }
        }

        Ok(())
    }

    fn prologue_imm_const(&mut self, imm: i64) {
        assert!(imm <= core::u32::MAX as i64);
        assert!(self.stack_size.is_none());

        // This instruction should only appear in a prologue to pass an
        // argument of the stack size to a stack check function.
        // Record the stack size so we know what it is when we encounter the adjustment
        // instruction (which will adjust via the register assigned to this instruction).
        self.stack_size = Some(imm as i32);
    }

    fn ret(&mut self, inst: Inst) -> Result<(), RegisterMappingError> {
        let args = self.func.dfg.inst_args(inst);

        for (i, arg) in args.iter().rev().enumerate() {
            // Only walk back the args for the pop instructions encountered
            if i >= self.epilogue_pop_offsets.len() {
                break;
            }

            self.cfa_offset -= 8;
            let reg = self.func.locations[*arg].unwrap_reg();

            // Update the CFA if this is the restore of the frame pointer register or if a frame pointer isn't being used
            match self.frame_register {
                Some(fp) => {
                    if reg == fp {
                        self.instructions.push((
                            self.epilogue_pop_offsets[i],
                            CallFrameInstruction::Cfa(
                                map_reg(self.isa, RU::rsp as RegUnit)?.0,
                                self.cfa_offset,
                            ),
                        ));
                    }
                }
                None => {
                    self.instructions.push((
                        self.epilogue_pop_offsets[i],
                        CallFrameInstruction::CfaOffset(self.cfa_offset),
                    ));

                    // Pops in the epilogue are register restores, so record a "same value" for the register
                    // This isn't necessary when using a frame pointer as the CFA doesn't change for CSR restores
                    self.instructions.push((
                        self.epilogue_pop_offsets[i],
                        CallFrameInstruction::SameValue(map_reg(self.isa, reg)?.0),
                    ));
                }
            };
        }

        self.epilogue_pop_offsets.clear();

        Ok(())
    }

    fn insert_pop_offset(&mut self, offset: u32) {
        self.epilogue_pop_offsets.push(offset);
    }

    fn remember_state(&mut self, offset: u32) {
        self.instructions
            .push((offset, CallFrameInstruction::RememberState));
    }

    fn restore_state(&mut self, offset: u32) {
        self.instructions
            .push((offset, CallFrameInstruction::RestoreState));
    }

    fn is_prologue_end(&self, inst: Inst) -> bool {
        self.func.prologue_end == Some(inst)
    }

    fn is_epilogue_start(&self, inst: Inst) -> bool {
        self.func.epilogues_start.contains(&inst)
    }
}

pub(crate) fn create_unwind_info(
    func: &Function,
    isa: &dyn TargetIsa,
    frame_register: Option<RegUnit>,
) -> CodegenResult<Option<UnwindInfo>> {
    // Only System V-like calling conventions are supported
    match func.signature.call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => {}
        _ => return Ok(None),
    }

    if func.prologue_end.is_none() || isa.name() != "x86" || isa.pointer_bits() != 64 {
        return Ok(None);
    }

    let mut builder = InstructionBuilder::new(func, isa, frame_register);
    let mut in_prologue = true;
    let mut in_epilogue = false;
    let mut len = 0;

    let mut blocks = func.layout.blocks().collect::<Vec<_>>();
    blocks.sort_by_key(|b| func.offsets[*b]);

    for (i, block) in blocks.iter().enumerate() {
        for (offset, inst, size) in func.inst_offsets(*block, &isa.encoding_info()) {
            let offset = offset + size;
            assert!(len <= offset);
            len = offset;

            let is_last_block = i == blocks.len() - 1;

            if in_prologue {
                // Check for prologue end (inclusive)
                in_prologue = !builder.is_prologue_end(inst);
            } else if !in_epilogue && builder.is_epilogue_start(inst) {
                // Now in an epilogue, emit a remember state instruction if not last block
                in_epilogue = true;

                if !is_last_block {
                    builder.remember_state(offset);
                }
            } else if !in_epilogue {
                // Ignore normal instructions
                continue;
            }

            match builder.func.dfg[inst] {
                InstructionData::Unary { opcode, arg } => match opcode {
                    Opcode::X86Push => {
                        builder
                            .push_reg(offset, arg)
                            .map_err(CodegenError::RegisterMappingError)?;
                    }
                    Opcode::AdjustSpDown => {
                        builder.adjust_sp_down(offset);
                    }
                    _ => {}
                },
                InstructionData::CopySpecial { src, dst, .. } => {
                    builder
                        .move_reg(offset, src, dst)
                        .map_err(CodegenError::RegisterMappingError)?;
                }
                InstructionData::NullAry { opcode } => match opcode {
                    Opcode::X86Pop => {
                        builder.insert_pop_offset(offset);
                    }
                    _ => {}
                },
                InstructionData::UnaryImm { opcode, imm } => match opcode {
                    Opcode::Iconst => {
                        builder.prologue_imm_const(imm.into());
                    }
                    Opcode::AdjustSpDownImm => {
                        builder.adjust_sp_down_imm(offset, imm.into());
                    }
                    Opcode::AdjustSpUpImm => {
                        builder.adjust_sp_up_imm(offset, imm.into());
                    }
                    _ => {}
                },
                InstructionData::MultiAry { opcode, .. } => match opcode {
                    Opcode::Return => {
                        builder
                            .ret(inst)
                            .map_err(CodegenError::RegisterMappingError)?;

                        if !is_last_block {
                            builder.restore_state(offset);
                        }

                        in_epilogue = false;
                    }
                    _ => {}
                },
                _ => {}
            };
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
