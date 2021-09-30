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
use crate::ir::types::I32;
use crate::ir::{self, InstBuilder, MemFlags};
use crate::isa::TargetIsa;

mod globalvalue;
mod heap;
mod table;

use self::globalvalue::expand_global_value;
use self::heap::expand_heap_addr;
use self::table::expand_table_addr;

/// Perform a simple legalization by expansion of the function, without
/// platform-specific transforms.
pub fn simple_legalize(func: &mut ir::Function, cfg: &mut ControlFlowGraph, isa: &dyn TargetIsa) {
    macro_rules! expand_imm_op {
        ($pos:ident, $inst:ident: $from:ident => $to:ident) => {{
            let (arg, imm) = match $pos.func.dfg[$inst] {
                ir::InstructionData::BinaryImm64 {
                    opcode: _,
                    arg,
                    imm,
                } => (arg, imm),
                _ => panic!(
                    concat!("Expected ", stringify!($from), ": {}"),
                    $pos.func.dfg.display_inst($inst)
                ),
            };
            let ty = $pos.func.dfg.value_type(arg);
            let imm = $pos.ins().iconst(ty, imm);
            $pos.func.dfg.replace($inst).$to(arg, imm);
        }};

        ($pos:ident, $inst:ident<$ty:ident>: $from:ident => $to:ident) => {{
            let (arg, imm) = match $pos.func.dfg[$inst] {
                ir::InstructionData::BinaryImm64 {
                    opcode: _,
                    arg,
                    imm,
                } => (arg, imm),
                _ => panic!(
                    concat!("Expected ", stringify!($from), ": {}"),
                    $pos.func.dfg.display_inst($inst)
                ),
            };
            let imm = $pos.ins().iconst($ty, imm);
            $pos.func.dfg.replace($inst).$to(arg, imm);
        }};
    }

    let mut pos = FuncCursor::new(func);
    let func_begin = pos.position();
    pos.set_position(func_begin);
    while let Some(_block) = pos.next_block() {
        let mut prev_pos = pos.position();
        while let Some(inst) = pos.next_inst() {
            match pos.func.dfg[inst].opcode() {
                // control flow
                ir::Opcode::BrIcmp => expand_br_icmp(inst, &mut pos.func, cfg, isa),
                ir::Opcode::Trapnz | ir::Opcode::Trapz | ir::Opcode::ResumableTrapnz => {
                    expand_cond_trap(inst, &mut pos.func, cfg, isa);
                }

                // memory and constants
                ir::Opcode::GlobalValue => expand_global_value(inst, &mut pos.func, cfg, isa),
                ir::Opcode::HeapAddr => expand_heap_addr(inst, &mut pos.func, cfg, isa),
                ir::Opcode::StackLoad => expand_stack_load(inst, &mut pos.func, cfg, isa),
                ir::Opcode::StackStore => expand_stack_store(inst, &mut pos.func, cfg, isa),
                ir::Opcode::TableAddr => expand_table_addr(inst, &mut pos.func, cfg, isa),

                // bitops
                ir::Opcode::BandImm => expand_imm_op!(pos, inst: band_imm => band),
                ir::Opcode::BorImm => expand_imm_op!(pos, inst: bor_imm => bor),
                ir::Opcode::BxorImm => expand_imm_op!(pos, inst: bxor_imm => bxor),
                ir::Opcode::IaddImm => expand_imm_op!(pos, inst: iadd_imm => iadd),

                // bitshifting
                ir::Opcode::IshlImm => expand_imm_op!(pos, inst<I32>: ishl_imm => ishl),
                ir::Opcode::RotlImm => expand_imm_op!(pos, inst<I32>: rotl_imm => rotl),
                ir::Opcode::RotrImm => expand_imm_op!(pos, inst<I32>: rotr_imm => rotr),
                ir::Opcode::SshrImm => expand_imm_op!(pos, inst<I32>: sshr_imm => sshr),
                ir::Opcode::UshrImm => expand_imm_op!(pos, inst<I32>: ushr_imm => ushr),

                // math
                ir::Opcode::IrsubImm => {
                    let (arg, imm) = match pos.func.dfg[inst] {
                        ir::InstructionData::BinaryImm64 {
                            opcode: _,
                            arg,
                            imm,
                        } => (arg, imm),
                        _ => panic!("Expected irsub_imm: {}", pos.func.dfg.display_inst(inst)),
                    };
                    let ty = pos.func.dfg.value_type(arg);
                    let imm = pos.ins().iconst(ty, imm);
                    pos.func.dfg.replace(inst).isub(imm, arg); // note: arg order reversed
                }
                ir::Opcode::ImulImm => expand_imm_op!(pos, inst: imul_imm => imul),
                ir::Opcode::SdivImm => expand_imm_op!(pos, inst: sdiv_imm => sdiv),
                ir::Opcode::SremImm => expand_imm_op!(pos, inst: srem_imm => srem),
                ir::Opcode::UdivImm => expand_imm_op!(pos, inst: udiv_imm => udiv),
                ir::Opcode::UremImm => expand_imm_op!(pos, inst: urem_imm => urem),

                // comparisons
                ir::Opcode::IfcmpImm => expand_imm_op!(pos, inst: ifcmp_imm => ifcmp),
                ir::Opcode::IcmpImm => {
                    let (cc, x, y) = match pos.func.dfg[inst] {
                        ir::InstructionData::IntCompareImm {
                            opcode: _,
                            cond,
                            arg,
                            imm,
                        } => (cond, arg, imm),
                        _ => panic!("Expected ircmp_imm: {}", pos.func.dfg.display_inst(inst)),
                    };
                    let ty = pos.func.dfg.value_type(x);
                    let y = pos.ins().iconst(ty, y);
                    pos.func.dfg.replace(inst).icmp(cc, x, y);
                }

                _ => {
                    prev_pos = pos.position();
                    continue;
                }
            };

            // Legalization implementations require fixpoint loop here.
            // TODO: fix this.
            pos.set_position(prev_pos);
        }
    }
}

/// Custom expansion for conditional trap instructions.
/// TODO: Add CFG support to the Rust DSL patterns so we won't have to do this.
fn expand_cond_trap(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    // Parse the instruction.
    let trapz;
    let (arg, code, opcode) = match func.dfg[inst] {
        ir::InstructionData::CondTrap { opcode, arg, code } => {
            // We want to branch *over* an unconditional trap.
            trapz = match opcode {
                ir::Opcode::Trapz => true,
                ir::Opcode::Trapnz | ir::Opcode::ResumableTrapnz => false,
                _ => panic!("Expected cond trap: {}", func.dfg.display_inst(inst)),
            };
            (arg, code, opcode)
        }
        _ => panic!("Expected cond trap: {}", func.dfg.display_inst(inst)),
    };

    // Split the block after `inst`:
    //
    //     trapnz arg
    //     ..
    //
    // Becomes:
    //
    //     brz arg, new_block_resume
    //     jump new_block_trap
    //
    //   new_block_trap:
    //     trap
    //
    //   new_block_resume:
    //     ..
    let old_block = func.layout.pp_block(inst);
    let new_block_trap = func.dfg.make_block();
    let new_block_resume = func.dfg.make_block();

    // Replace trap instruction by the inverted condition.
    if trapz {
        func.dfg.replace(inst).brnz(arg, new_block_resume, &[]);
    } else {
        func.dfg.replace(inst).brz(arg, new_block_resume, &[]);
    }

    // Add jump instruction after the inverted branch.
    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);
    pos.ins().jump(new_block_trap, &[]);

    // Insert the new label and the unconditional trap terminator.
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

fn expand_br_icmp(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &dyn TargetIsa,
) {
    let (cond, a, b, destination, block_args) = match func.dfg[inst] {
        ir::InstructionData::BranchIcmp {
            cond,
            destination,
            ref args,
            ..
        } => (
            cond,
            args.get(0, &func.dfg.value_lists).unwrap(),
            args.get(1, &func.dfg.value_lists).unwrap(),
            destination,
            args.as_slice(&func.dfg.value_lists)[2..].to_vec(),
        ),
        _ => panic!("Expected br_icmp {}", func.dfg.display_inst(inst)),
    };

    let old_block = func.layout.pp_block(inst);
    func.dfg.clear_results(inst);

    let icmp_res = func.dfg.replace(inst).icmp(cond, a, b);
    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);
    pos.ins().brnz(icmp_res, destination, &block_args);

    cfg.recompute_block(pos.func, destination);
    cfg.recompute_block(pos.func, old_block);
}

/// Expand illegal `stack_load` instructions.
fn expand_stack_load(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let ty = func.dfg.value_type(func.dfg.first_result(inst));
    let addr_ty = isa.pointer_type();

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    let (stack_slot, offset) = match pos.func.dfg[inst] {
        ir::InstructionData::StackLoad {
            opcode: _opcode,
            stack_slot,
            offset,
        } => (stack_slot, offset),
        _ => panic!("Expected stack_load: {}", pos.func.dfg.display_inst(inst)),
    };

    let addr = pos.ins().stack_addr(addr_ty, stack_slot, offset);

    // Stack slots are required to be accessible and aligned.
    let mflags = MemFlags::trusted();
    pos.func.dfg.replace(inst).load(ty, mflags, addr, 0);
}

/// Expand illegal `stack_store` instructions.
fn expand_stack_store(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    let addr_ty = isa.pointer_type();

    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    let (val, stack_slot, offset) = match pos.func.dfg[inst] {
        ir::InstructionData::StackStore {
            opcode: _opcode,
            arg,
            stack_slot,
            offset,
        } => (arg, stack_slot, offset),
        _ => panic!("Expected stack_store: {}", pos.func.dfg.display_inst(inst)),
    };

    let addr = pos.ins().stack_addr(addr_ty, stack_slot, offset);

    let mut mflags = MemFlags::new();
    // Stack slots are required to be accessible and aligned.
    mflags.set_notrap();
    mflags.set_aligned();
    pos.func.dfg.replace(inst).store(mflags, val, addr, 0);
}
