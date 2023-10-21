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
use crate::flowgraph::ControlFlowGraph;
use crate::ir::immediates::Imm64;
use crate::ir::types::{self, I128, I64};
use crate::ir::{self, InstBuilder, InstructionData, Value};
use crate::isa::TargetIsa;
use crate::trace;

mod globalvalue;
pub mod isle;
mod table;

use self::globalvalue::expand_global_value;
use self::table::expand_table_addr;

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
pub fn simple_legalize(func: &mut ir::Function, cfg: &mut ControlFlowGraph, isa: &dyn TargetIsa) {
    trace!("Pre-legalization function:\n{}", func.display());

    let mut pos = FuncCursor::new(func);
    let func_begin = pos.position();
    pos.set_position(func_begin);
    while let Some(_block) = pos.next_block() {
        let mut prev_pos = pos.position();
        while let Some(inst) = pos.next_inst() {
            match pos.func.dfg.insts[inst] {
                // control flow
                InstructionData::CondTrap {
                    opcode: ir::Opcode::Trapnz,
                    arg,
                    code,
                } => {
                    expand_cond_trap(inst, &mut pos.func, cfg, ir::Opcode::Trapnz, arg, code);
                }

                // memory and constants
                InstructionData::UnaryGlobalValue {
                    opcode: ir::Opcode::GlobalValue,
                    global_value,
                } => expand_global_value(inst, &mut pos.func, isa, global_value),
                InstructionData::TableAddr {
                    opcode: ir::Opcode::TableAddr,
                    table,
                    arg,
                    offset,
                } => expand_table_addr(isa, inst, &mut pos.func, table, arg, offset),

                InstructionData::BinaryImm64 { opcode, arg, imm } => {
                    match opcode {
                        ir::Opcode::IaddImm => {
                            let imm = imm_const(&mut pos, arg, imm, true);
                            let replace = pos.func.dfg.replace(inst);
                            replace.iadd(arg, imm);
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

/// Custom expansion for conditional trap instructions.
fn expand_cond_trap(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    opcode: ir::Opcode,
    arg: ir::Value,
    code: ir::TrapCode,
) {
    trace!(
        "expanding conditional trap: {:?}: {}",
        inst,
        func.dfg.display_inst(inst)
    );

    // Parse the instruction.
    let trapz = match opcode {
        ir::Opcode::Trapz => true,
        ir::Opcode::Trapnz | ir::Opcode::ResumableTrapnz => false,
        _ => panic!("Expected cond trap: {}", func.dfg.display_inst(inst)),
    };

    // Split the block after `inst`:
    //
    //     trapnz arg
    //     ..
    //
    // Becomes:
    //
    //     brif arg, new_block_trap, new_block_resume
    //
    //   new_block_trap:
    //     trap
    //
    //   new_block_resume:
    //     ..
    let old_block = func
        .layout
        .inst_block(inst)
        .expect("Instruction not in layout.");
    let new_block_trap = func.dfg.make_block();
    let new_block_resume = func.dfg.make_block();

    // Trapping is a rare event, mark the trapping block as cold.
    func.layout.set_cold(new_block_trap);

    // Replace trap instruction by the inverted condition.
    if trapz {
        func.dfg
            .replace(inst)
            .brif(arg, new_block_resume, &[], new_block_trap, &[]);
    } else {
        func.dfg
            .replace(inst)
            .brif(arg, new_block_trap, &[], new_block_resume, &[]);
    }

    // Insert the new label and the unconditional trap terminator.
    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);
    pos.insert_block(new_block_trap);

    match opcode {
        ir::Opcode::Trapz | ir::Opcode::Trapnz => {
            pos.ins().trap(code);
        }
        ir::Opcode::ResumableTrapnz => {
            pos.ins().resumable_trap(code);
            pos.ins().jump(new_block_resume, &[]);
        }
        _ => unreachable!(),
    }

    // Insert the new label and resume the execution when the trap fails.
    pos.insert_block(new_block_resume);

    // Finally update the CFG.
    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, new_block_resume);
    cfg.recompute_block(pos.func, new_block_trap);
}
