//! Legalize instructions.
//!
//! A legal instruction is one that can be mapped directly to a machine code instruction for the
//! target ISA. The `legalize_function()` function takes as input any function and transforms it
//! into an equivalent function using only legal instructions.
//!
//! The characteristics of legal instructions depend on the target ISA, so any given instruction
//! can be legal for one ISA and illegal for another.
//!
//! Besides transforming instructions, the legalizer also fills out the `function.encodings` map
//! which provides a legal encoding recipe for every instruction.
//!
//! The legalizer does not deal with register allocation constraints. These constraints are derived
//! from the encoding recipes, and solved later by the register allocator.

use crate::cursor::{Cursor, FuncCursor};
use crate::ir::immediates::Imm64;
use crate::ir::types::{self, I128, I64};
use crate::ir::{self, InstBuilder, InstructionData, MemFlags, Value};
use crate::isa::TargetIsa;
use crate::trace;

mod globalvalue;

use self::globalvalue::expand_global_value;

fn imm_const(pos: &mut FuncCursor, arg: Value, imm: Imm64, is_signed: bool) -> Value {
    let ty = pos.func.dfg.value_type(arg);
    match (ty, is_signed) {
        (I128, true) => {
            let imm = pos.ins().iconst(I64, imm);
            pos.ins().sextend(I128, imm)
        }
        (I128, false) => {
            let imm = pos.ins().iconst(I64, imm);
            pos.ins().uextend(I128, imm)
        }
        _ => {
            let bits = imm.bits();
            let unsigned = match ty.lane_type() {
                types::I8 => bits as u8 as i64,
                types::I16 => bits as u16 as i64,
                types::I32 => bits as u32 as i64,
                types::I64 => bits,
                _ => unreachable!(),
            };
            pos.ins().iconst(ty.lane_type(), unsigned)
        }
    }
}

/// Perform a simple legalization by expansion of the function, without
/// platform-specific transforms.
pub fn simple_legalize(func: &mut ir::Function, isa: &dyn TargetIsa) {
    trace!("Pre-legalization function:\n{}", func.display());

    let mut pos = FuncCursor::new(func);
    let func_begin = pos.position();
    pos.set_position(func_begin);
    while let Some(_block) = pos.next_block() {
        let mut prev_pos = pos.position();
        while let Some(inst) = pos.next_inst() {
            match pos.func.dfg.insts[inst] {
                // memory and constants
                InstructionData::UnaryGlobalValue {
                    opcode: ir::Opcode::GlobalValue,
                    global_value,
                } => expand_global_value(inst, &mut pos.func, isa, global_value),
                InstructionData::StackLoad {
                    opcode: ir::Opcode::StackLoad,
                    stack_slot,
                    offset,
                } => {
                    let ty = pos.func.dfg.value_type(pos.func.dfg.first_result(inst));
                    let addr_ty = isa.pointer_type();

                    let mut pos = FuncCursor::new(pos.func).at_inst(inst);
                    pos.use_srcloc(inst);

                    let addr = pos.ins().stack_addr(addr_ty, stack_slot, offset);

                    // Stack slots are required to be accessible.
                    // We can't currently ensure that they are aligned.
                    let mut mflags = MemFlags::new();
                    mflags.set_notrap();
                    pos.func.dfg.replace(inst).load(ty, mflags, addr, 0);
                }
                InstructionData::StackStore {
                    opcode: ir::Opcode::StackStore,
                    arg,
                    stack_slot,
                    offset,
                } => {
                    let addr_ty = isa.pointer_type();

                    let mut pos = FuncCursor::new(pos.func).at_inst(inst);
                    pos.use_srcloc(inst);

                    let addr = pos.ins().stack_addr(addr_ty, stack_slot, offset);

                    // Stack slots are required to be accessible.
                    // We can't currently ensure that they are aligned.
                    let mut mflags = MemFlags::new();
                    mflags.set_notrap();
                    pos.func.dfg.replace(inst).store(mflags, arg, addr, 0);
                }
                InstructionData::DynamicStackLoad {
                    opcode: ir::Opcode::DynamicStackLoad,
                    dynamic_stack_slot,
                } => {
                    let ty = pos.func.dfg.value_type(pos.func.dfg.first_result(inst));
                    assert!(ty.is_dynamic_vector());
                    let addr_ty = isa.pointer_type();

                    let mut pos = FuncCursor::new(pos.func).at_inst(inst);
                    pos.use_srcloc(inst);

                    let addr = pos.ins().dynamic_stack_addr(addr_ty, dynamic_stack_slot);

                    // Stack slots are required to be accessible and aligned.
                    let mflags = MemFlags::trusted();
                    pos.func.dfg.replace(inst).load(ty, mflags, addr, 0);
                }
                InstructionData::DynamicStackStore {
                    opcode: ir::Opcode::DynamicStackStore,
                    arg,
                    dynamic_stack_slot,
                } => {
                    pos.use_srcloc(inst);
                    let addr_ty = isa.pointer_type();
                    let vector_ty = pos.func.dfg.value_type(arg);
                    assert!(vector_ty.is_dynamic_vector());

                    let addr = pos.ins().dynamic_stack_addr(addr_ty, dynamic_stack_slot);

                    let mut mflags = MemFlags::new();
                    // Stack slots are required to be accessible and aligned.
                    mflags.set_notrap();
                    mflags.set_aligned();
                    pos.func.dfg.replace(inst).store(mflags, arg, addr, 0);
                }

                InstructionData::BinaryImm64 { opcode, arg, imm } => {
                    let is_signed = match opcode {
                        ir::Opcode::IaddImm
                        | ir::Opcode::IrsubImm
                        | ir::Opcode::ImulImm
                        | ir::Opcode::SdivImm
                        | ir::Opcode::SremImm => true,
                        _ => false,
                    };

                    let imm = imm_const(&mut pos, arg, imm, is_signed);
                    let replace = pos.func.dfg.replace(inst);
                    match opcode {
                        // bitops
                        ir::Opcode::BandImm => {
                            replace.band(arg, imm);
                        }
                        ir::Opcode::BorImm => {
                            replace.bor(arg, imm);
                        }
                        ir::Opcode::BxorImm => {
                            replace.bxor(arg, imm);
                        }
                        // bitshifting
                        ir::Opcode::IshlImm => {
                            replace.ishl(arg, imm);
                        }
                        ir::Opcode::RotlImm => {
                            replace.rotl(arg, imm);
                        }
                        ir::Opcode::RotrImm => {
                            replace.rotr(arg, imm);
                        }
                        ir::Opcode::SshrImm => {
                            replace.sshr(arg, imm);
                        }
                        ir::Opcode::UshrImm => {
                            replace.ushr(arg, imm);
                        }
                        // math
                        ir::Opcode::IaddImm => {
                            replace.iadd(arg, imm);
                        }
                        ir::Opcode::IrsubImm => {
                            // note: arg order reversed
                            replace.isub(imm, arg);
                        }
                        ir::Opcode::ImulImm => {
                            replace.imul(arg, imm);
                        }
                        ir::Opcode::SdivImm => {
                            replace.sdiv(arg, imm);
                        }
                        ir::Opcode::SremImm => {
                            replace.srem(arg, imm);
                        }
                        ir::Opcode::UdivImm => {
                            replace.udiv(arg, imm);
                        }
                        ir::Opcode::UremImm => {
                            replace.urem(arg, imm);
                        }
                        _ => prev_pos = pos.position(),
                    };
                }

                // comparisons
                InstructionData::IntCompareImm {
                    opcode: ir::Opcode::IcmpImm,
                    cond,
                    arg,
                    imm,
                } => {
                    let imm = imm_const(&mut pos, arg, imm, true);
                    pos.func.dfg.replace(inst).icmp(cond, arg, imm);
                }

                // Legalize the fused bitwise-plus-not instructions into simpler
                // instructions to assist with optimizations. Lowering will
                // pattern match this sequence regardless when architectures
                // support the instruction natively.
                InstructionData::Binary { opcode, args } => {
                    match opcode {
                        ir::Opcode::BandNot => {
                            let neg = pos.ins().bnot(args[1]);
                            pos.func.dfg.replace(inst).band(args[0], neg);
                        }
                        ir::Opcode::BorNot => {
                            let neg = pos.ins().bnot(args[1]);
                            pos.func.dfg.replace(inst).bor(args[0], neg);
                        }
                        ir::Opcode::BxorNot => {
                            let neg = pos.ins().bnot(args[1]);
                            pos.func.dfg.replace(inst).bxor(args[0], neg);
                        }
                        _ => prev_pos = pos.position(),
                    };
                }

                _ => {
                    prev_pos = pos.position();
                    continue;
                }
            }

            // Legalization implementations require fixpoint loop here.
            // TODO: fix this.
            pos.set_position(prev_pos);
        }
    }

    trace!("Post-legalization function:\n{}", func.display());
}
