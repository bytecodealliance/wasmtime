use super::*;
use crate::isa::aarch64::inst::{args::PairAMode, imms::Imm12, regs, ALUOp, Inst};
use crate::isa::unwind::input::{UnwindCode, UnwindInfo};
use crate::machinst::UnwindInfoContext;
use crate::result::CodegenResult;
use alloc::vec::Vec;
use regalloc::Reg;

#[cfg(feature = "unwind")]
pub(crate) mod systemv;

pub struct AArch64UnwindInfo;

impl UnwindInfoGenerator<Inst> for AArch64UnwindInfo {
    fn create_unwind_info(
        context: UnwindInfoContext<Inst>,
    ) -> CodegenResult<Option<UnwindInfo<Reg>>> {
        let word_size = 8u8;
        let pair_size = word_size * 2;
        let mut codes = Vec::new();

        for i in context.prologue.clone() {
            let i = i as usize;
            let inst = &context.insts[i];
            let offset = context.insts_layout[i];

            match inst {
                Inst::StoreP64 {
                    rt,
                    rt2,
                    mem: PairAMode::PreIndexed(rn, imm7),
                    ..
                } if *rt == regs::fp_reg()
                    && *rt2 == regs::link_reg()
                    && *rn == regs::writable_stack_reg()
                    && imm7.value == -(pair_size as i16) =>
                {
                    // stp fp (x29), lr (x30), [sp, #-16]!
                    codes.push((
                        offset,
                        UnwindCode::StackAlloc {
                            size: pair_size as u32,
                        },
                    ));
                    codes.push((
                        offset,
                        UnwindCode::SaveRegister {
                            reg: *rt,
                            stack_offset: 0,
                        },
                    ));
                    codes.push((
                        offset,
                        UnwindCode::SaveRegister {
                            reg: *rt2,
                            stack_offset: word_size as u32,
                        },
                    ));
                }
                Inst::StoreP64 {
                    rt,
                    rt2,
                    mem: PairAMode::PreIndexed(rn, imm7),
                    ..
                } if rn.to_reg() == regs::stack_reg() && imm7.value % (pair_size as i16) == 0 => {
                    // stp r1, r2, [sp, #(i * #16)]
                    let stack_offset = imm7.value as u32;
                    codes.push((
                        offset,
                        UnwindCode::SaveRegister {
                            reg: *rt,
                            stack_offset,
                        },
                    ));
                    if *rt2 != regs::zero_reg() {
                        codes.push((
                            offset,
                            UnwindCode::SaveRegister {
                                reg: *rt2,
                                stack_offset: stack_offset + word_size as u32,
                            },
                        ));
                    }
                }
                Inst::AluRRImm12 {
                    alu_op: ALUOp::Add64,
                    rd,
                    rn,
                    imm12:
                        Imm12 {
                            bits: 0,
                            shift12: false,
                        },
                } if *rd == regs::writable_fp_reg() && *rn == regs::stack_reg() => {
                    // mov fp (x29), sp.
                    codes.push((offset, UnwindCode::SetFramePointer { reg: rd.to_reg() }));
                }
                Inst::VirtualSPOffsetAdj { offset: adj } if offset > 0 => {
                    codes.push((offset, UnwindCode::StackAlloc { size: *adj as u32 }));
                }
                _ => {}
            }
        }

        // TODO epilogues

        let prologue_size = if context.prologue.len() == 0 {
            0
        } else {
            context.insts_layout[context.prologue.end as usize - 1]
        };

        Ok(Some(UnwindInfo {
            prologue_size,
            prologue_unwind_codes: codes,
            epilogues_unwind_codes: vec![],
            function_size: context.len,
            word_size,
            initial_sp_offset: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{Cursor, FuncCursor};
    use crate::ir::{ExternalName, Function, InstBuilder, Signature, StackSlotData, StackSlotKind};
    use crate::isa::{lookup, CallConv};
    use crate::settings::{builder, Flags};
    use crate::Context;
    use std::str::FromStr;
    use target_lexicon::triple;

    #[test]
    fn test_simple_func() {
        let isa = lookup(triple!("aarch64"))
            .expect("expect aarch64 ISA")
            .finish(Flags::new(builder()));

        let mut context = Context::for_function(create_function(
            CallConv::SystemV,
            Some(StackSlotData::new(StackSlotKind::ExplicitSlot, 64)),
        ));

        context.compile(&*isa).expect("expected compilation");

        let result = context.mach_compile_result.unwrap();
        let unwind_info = result.unwind_info.unwrap();

        assert_eq!(
            unwind_info,
            UnwindInfo {
                prologue_size: 12,
                prologue_unwind_codes: vec![
                    (4, UnwindCode::StackAlloc { size: 16 }),
                    (
                        4,
                        UnwindCode::SaveRegister {
                            reg: regs::fp_reg(),
                            stack_offset: 0
                        }
                    ),
                    (
                        4,
                        UnwindCode::SaveRegister {
                            reg: regs::link_reg(),
                            stack_offset: 8
                        }
                    ),
                    (
                        8,
                        UnwindCode::SetFramePointer {
                            reg: regs::fp_reg()
                        }
                    )
                ],
                epilogues_unwind_codes: vec![],
                function_size: 24,
                word_size: 8,
                initial_sp_offset: 0,
            }
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
