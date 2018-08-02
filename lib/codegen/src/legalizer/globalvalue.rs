//! Legalization of global values.
//!
//! This module exports the `expand_global_value` function which transforms a `global_value`
//! instruction into code that depends on the kind of global value referenced.

use cursor::{Cursor, FuncCursor};
use flowgraph::ControlFlowGraph;
use ir::{self, InstBuilder};
use isa::TargetIsa;

/// Expand a `global_value` instruction according to the definition of the global value.
pub fn expand_global_value(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    _isa: &TargetIsa,
) {
    // Unpack the instruction.
    let gv = match func.dfg[inst] {
        ir::InstructionData::UnaryGlobalValue {
            opcode,
            global_value,
        } => {
            debug_assert_eq!(opcode, ir::Opcode::GlobalValue);
            global_value
        }
        _ => panic!("Wanted global_value: {}", func.dfg.display_inst(inst, None)),
    };

    match func.global_values[gv] {
        ir::GlobalValueData::VMContext { offset } => vmctx_addr(inst, func, offset.into()),
        ir::GlobalValueData::Deref { base, offset } => deref_addr(inst, func, base, offset.into()),
        ir::GlobalValueData::Sym { .. } => globalsym(inst, func, gv),
    }
}

/// Expand a `global_value` instruction for a vmctx global.
fn vmctx_addr(inst: ir::Inst, func: &mut ir::Function, offset: i64) {
    // Get the value representing the `vmctx` argument.
    let vmctx = func
        .special_param(ir::ArgumentPurpose::VMContext)
        .expect("Missing vmctx parameter");

    // Simply replace the `global_value` instruction with an `iadd_imm`, reusing the result value.
    func.dfg.replace(inst).iadd_imm(vmctx, offset);
}

/// Expand a `global_value` instruction for a deref global.
fn deref_addr(inst: ir::Inst, func: &mut ir::Function, base: ir::GlobalValue, offset: i64) {
    // We need to load a pointer from the `base` global value, so insert a new `global_value`
    // instruction. This depends on the iterative legalization loop. Note that the IR verifier
    // detects any cycles in the `deref` globals.
    let ptr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    let base_addr = pos.ins().global_value(ptr_ty, base);
    let mut mflags = ir::MemFlags::new();
    // Deref globals are required to be accessible and aligned.
    mflags.set_notrap();
    mflags.set_aligned();
    let base_ptr = pos.ins().load(ptr_ty, mflags, base_addr, 0);
    pos.func.dfg.replace(inst).iadd_imm(base_ptr, offset);
}

/// Expand a `global_value` instruction for a symbolic name global.
fn globalsym(inst: ir::Inst, func: &mut ir::Function, gv: ir::GlobalValue) {
    let ptr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    func.dfg.replace(inst).globalsym_addr(ptr_ty, gv);
}
