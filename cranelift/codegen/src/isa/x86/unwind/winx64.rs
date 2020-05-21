//! Unwind information for Windows x64 ABI.

use crate::ir::{Function, InstructionData, Opcode, ValueLoc};
use crate::isa::x86::registers::{FPR, GPR, RU};
use crate::isa::{
    unwind::winx64::{UnwindCode, UnwindInfo},
    CallConv, RegUnit, TargetIsa,
};
use crate::result::{CodegenError, CodegenResult};
use alloc::vec::Vec;
use log::warn;

pub(crate) fn create_unwind_info(
    func: &Function,
    isa: &dyn TargetIsa,
    frame_register: Option<RegUnit>,
) -> CodegenResult<Option<UnwindInfo>> {
    // Only Windows fastcall is supported for unwind information
    if func.signature.call_conv != CallConv::WindowsFastcall || func.prologue_end.is_none() {
        return Ok(None);
    }

    let prologue_end = func.prologue_end.unwrap();
    let entry_block = func.layout.entry_block().expect("missing entry block");

    // Stores the stack size when SP is not adjusted via an immediate value
    let mut stack_size = None;
    let mut prologue_size = 0;
    let mut unwind_codes = Vec::new();
    let mut found_end = false;
    let mut xmm_save_count: u8 = 0;

    for (offset, inst, size) in func.inst_offsets(entry_block, &isa.encoding_info()) {
        // x64 ABI prologues cannot exceed 255 bytes in length
        if (offset + size) > 255 {
            warn!("function prologues cannot exceed 255 bytes in size for Windows x64");
            return Err(CodegenError::CodeTooLarge);
        }

        prologue_size += size;

        let unwind_offset = (offset + size) as u8;

        match func.dfg[inst] {
            InstructionData::Unary { opcode, arg } => {
                match opcode {
                    Opcode::X86Push => {
                        unwind_codes.push(UnwindCode::PushRegister {
                            offset: unwind_offset,
                            reg: GPR.index_of(func.locations[arg].unwrap_reg()) as u8,
                        });
                    }
                    Opcode::AdjustSpDown => {
                        let stack_size =
                            stack_size.expect("expected a previous stack size instruction");

                        // This is used when calling a stack check function
                        // We need to track the assignment to RAX which has the size of the stack
                        unwind_codes.push(UnwindCode::StackAlloc {
                            offset: unwind_offset,
                            size: stack_size,
                        });
                    }
                    _ => {}
                }
            }
            InstructionData::CopySpecial { src, dst, .. } => {
                if let Some(frame_register) = frame_register {
                    if src == (RU::rsp as RegUnit) && dst == frame_register {
                        unwind_codes.push(UnwindCode::SetFramePointer {
                            offset: unwind_offset,
                            sp_offset: 0,
                        });
                    }
                }
            }
            InstructionData::UnaryImm { opcode, imm } => {
                match opcode {
                    Opcode::Iconst => {
                        let imm: i64 = imm.into();
                        assert!(imm <= core::u32::MAX as i64);
                        assert!(stack_size.is_none());

                        // This instruction should only appear in a prologue to pass an
                        // argument of the stack size to a stack check function.
                        // Record the stack size so we know what it is when we encounter the adjustment
                        // instruction (which will adjust via the register assigned to this instruction).
                        stack_size = Some(imm as u32);
                    }
                    Opcode::AdjustSpDownImm => {
                        let imm: i64 = imm.into();
                        assert!(imm <= core::u32::MAX as i64);

                        stack_size = Some(imm as u32);

                        unwind_codes.push(UnwindCode::StackAlloc {
                            offset: unwind_offset,
                            size: imm as u32,
                        });
                    }
                    _ => {}
                }
            }
            InstructionData::Store {
                opcode: Opcode::Store,
                args: [arg1, arg2],
                offset,
                ..
            } => {
                if let (ValueLoc::Reg(src), ValueLoc::Reg(dst)) =
                    (func.locations[arg1], func.locations[arg2])
                {
                    // If this is a save of an FPR, record an unwind operation
                    // Note: the stack_offset here is relative to an adjusted SP
                    // This will be fixed up later to be based on the frame pointer offset
                    if dst == (RU::rsp as RegUnit) && FPR.contains(src) {
                        let offset: i32 = offset.into();
                        unwind_codes.push(UnwindCode::SaveXmm {
                            offset: unwind_offset,
                            reg: src as u8,
                            stack_offset: offset as u32,
                        });

                        xmm_save_count += 1;
                    }
                }
            }
            _ => {}
        };

        if inst == prologue_end {
            found_end = true;
            break;
        }
    }

    assert!(found_end);

    let mut frame_register_offset = 0;
    if xmm_save_count > 0 {
        // If there are XMM saves, determine the number of 16-byte slots used for all CSRs (including GPRs)
        // The "frame register offset" will point at the last slot used (i.e. the last saved FPR)
        // Assumption: each FPR is stored at a lower address than the previous one
        let mut last_stack_offset = None;
        let mut fpr_save_count: u8 = 0;
        let mut gpr_push_count: u8 = 0;
        for code in unwind_codes.iter_mut() {
            match code {
                UnwindCode::SaveXmm { stack_offset, .. } => {
                    if let Some(last) = last_stack_offset {
                        assert!(last > *stack_offset);
                    }
                    last_stack_offset = Some(*stack_offset);
                    fpr_save_count += 1;
                    *stack_offset = (xmm_save_count - fpr_save_count) as u32 * 16;
                }
                UnwindCode::PushRegister { .. } => {
                    gpr_push_count += 1;
                }
                _ => {}
            }
        }
        assert_eq!(fpr_save_count, xmm_save_count);

        // Account for alignment space when there's an odd number of GPR pushes
        // Assumption: an FPR (16 bytes) is twice the size of a GPR (8 bytes), hence the (rounded-up) integer division
        frame_register_offset = fpr_save_count + ((gpr_push_count + 1) / 2);
    }

    Ok(Some(UnwindInfo {
        flags: 0, // this assumes cranelift functions have no SEH handlers
        prologue_size: prologue_size as u8,
        frame_register: frame_register.map(|r| GPR.index_of(r) as u8),
        frame_register_offset,
        unwind_codes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{ExternalName, InstBuilder, Signature, StackSlotData, StackSlotKind};
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    fn test_wrong_calling_convention() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(CallConv::SystemV, None));

        context.compile(&*isa).expect("expected compilation");

        assert_eq!(
            create_unwind_info(&context.func, &*isa, None).expect("can create unwind info"),
            None
        );
    }

    #[test]
    fn test_small_alloc() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::WindowsFastcall,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let unwind = create_unwind_info(&context.func, &*isa, Some(RU::rbp.into()))
            .expect("can create unwind info")
            .expect("expected unwind info");

        assert_eq!(
            unwind,
            UnwindInfo {
                flags: 0,
                prologue_size: 9,
                frame_register: Some(GPR.index_of(RU::rbp.into()) as u8),
                frame_register_offset: 0,
                unwind_codes: vec![
                    UnwindCode::PushRegister {
                        offset: 2,
                        reg: GPR.index_of(RU::rbp.into()) as u8
                    },
                    UnwindCode::SetFramePointer {
                        offset: 5,
                        sp_offset: 0
                    },
                    UnwindCode::StackAlloc {
                        offset: 9,
                        size: 64
                    }
                ]
            }
        );

        assert_eq!(unwind.emit_size(), 12);

        let mut buf = [0u8; 12];
        unwind.emit(&mut buf);

        assert_eq!(
            buf,
            [
                0x01, // Version and flags (version 1, no flags)
                0x09, // Prologue size
                0x03, // Unwind code count (1 for stack alloc, 1 for save frame reg, 1 for push reg)
                0x05, // Frame register + offset (RBP with 0 offset)
                0x09, // Prolog offset
                0x72, // Operation 2 (small stack alloc), size = 0xB slots (e.g. (0x7 * 8) + 8 = 64 bytes)
                0x05, // Prolog offset
                0x03, // Operation 3 (save frame register), stack pointer offset = 0
                0x02, // Prolog offset
                0x50, // Operation 0 (save nonvolatile register), reg = 5 (RBP)
                0x00, // Padding byte
                0x00, // Padding byte
            ]
        );
    }

    #[test]
    fn test_medium_alloc() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::WindowsFastcall,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 10000)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let unwind = create_unwind_info(&context.func, &*isa, Some(RU::rbp.into()))
            .expect("can create unwind info")
            .expect("expected unwind info");

        assert_eq!(
            unwind,
            UnwindInfo {
                flags: 0,
                prologue_size: 27,
                frame_register: Some(GPR.index_of(RU::rbp.into()) as u8),
                frame_register_offset: 0,
                unwind_codes: vec![
                    UnwindCode::PushRegister {
                        offset: 2,
                        reg: GPR.index_of(RU::rbp.into()) as u8
                    },
                    UnwindCode::SetFramePointer {
                        offset: 5,
                        sp_offset: 0
                    },
                    UnwindCode::StackAlloc {
                        offset: 27,
                        size: 10000
                    }
                ]
            }
        );

        assert_eq!(unwind.emit_size(), 12);

        let mut buf = [0u8; 12];
        unwind.emit(&mut buf);

        assert_eq!(
            buf,
            [
                0x01, // Version and flags (version 1, no flags)
                0x1B, // Prologue size
                0x04, // Unwind code count (2 for stack alloc, 1 for save frame reg, 1 for push reg)
                0x05, // Frame register + offset (RBP with 0 offset)
                0x1B, // Prolog offset
                0x01, // Operation 1 (large stack alloc), size is scaled 16-bits (info = 0)
                0xE2, // Low size byte
                0x04, // High size byte (e.g. 0x04E2 * 8 = 10000 bytes)
                0x05, // Prolog offset
                0x03, // Operation 3 (save frame register), stack pointer offset = 0
                0x02, // Prolog offset
                0x50, // Operation 0 (push nonvolatile register), reg = 5 (RBP)
            ]
        );
    }

    #[test]
    fn test_large_alloc() {
        let isa = lookup(triple!("x86_64"))
            .expect("expect x86 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::WindowsFastcall,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 1000000)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let unwind = create_unwind_info(&context.func, &*isa, Some(RU::rbp.into()))
            .expect("can create unwind info")
            .expect("expected unwind info");

        assert_eq!(
            unwind,
            UnwindInfo {
                flags: 0,
                prologue_size: 27,
                frame_register: Some(GPR.index_of(RU::rbp.into()) as u8),
                frame_register_offset: 0,
                unwind_codes: vec![
                    UnwindCode::PushRegister {
                        offset: 2,
                        reg: GPR.index_of(RU::rbp.into()) as u8
                    },
                    UnwindCode::SetFramePointer {
                        offset: 5,
                        sp_offset: 0
                    },
                    UnwindCode::StackAlloc {
                        offset: 27,
                        size: 1000000
                    }
                ]
            }
        );

        assert_eq!(unwind.emit_size(), 16);

        let mut buf = [0u8; 16];
        unwind.emit(&mut buf);

        assert_eq!(
            buf,
            [
                0x01, // Version and flags (version 1, no flags)
                0x1B, // Prologue size
                0x05, // Unwind code count (3 for stack alloc, 1 for save frame reg, 1 for push reg)
                0x05, // Frame register + offset (RBP with 0 offset)
                0x1B, // Prolog offset
                0x11, // Operation 1 (large stack alloc), size is unscaled 32-bits (info = 1)
                0x40, // Byte 1 of size
                0x42, // Byte 2 of size
                0x0F, // Byte 3 of size
                0x00, // Byte 4 of size (size is 0xF4240 = 1000000 bytes)
                0x05, // Prolog offset
                0x03, // Operation 3 (save frame register), stack pointer offset = 0
                0x02, // Prolog offset
                0x50, // Operation 0 (push nonvolatile register), reg = 5 (RBP)
                0x00, // Padding byte
                0x00, // Padding byte
            ]
        );
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
}
