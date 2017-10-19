//! Legalization of global variables.
//!
//! This module exports the `expand_global_addr` function which transforms a `global_addr`
//! instruction into code that depends on the kind of global variable referenced.

use cursor::{Cursor, FuncCursor};
use flowgraph::ControlFlowGraph;
use ir::{self, InstBuilder};

/// Expand a `global_addr` instruction according to the definition of the global variable.
pub fn expand_global_addr(inst: ir::Inst, func: &mut ir::Function, _cfg: &mut ControlFlowGraph) {
    // Unpack the instruction.
    let gv = match func.dfg[inst] {
        ir::InstructionData::UnaryGlobalVar { opcode, global_var } => {
            assert_eq!(opcode, ir::Opcode::GlobalAddr);
            global_var
        }
        _ => panic!("Wanted global_addr: {}", func.dfg.display_inst(inst, None)),
    };

    match func.global_vars[gv] {
        ir::GlobalVarData::VmCtx { offset } => vmctx_addr(inst, func, offset.into()),
        ir::GlobalVarData::Deref { base, offset } => deref_addr(inst, func, base, offset.into()),
    }
}

/// Expand a `global_addr` instruction for a vmctx global.
fn vmctx_addr(inst: ir::Inst, func: &mut ir::Function, offset: i64) {
    // Get the value representing the `vmctx` argument.
    let vmctx = func.special_arg(ir::ArgumentPurpose::VMContext).expect(
        "Missing vmctx parameter",
    );

    // Simply replace the `global_addr` instruction with an `iadd_imm`, reusing the result value.
    func.dfg.replace(inst).iadd_imm(vmctx, offset);
}

/// Expand a `global_addr` instruction for a deref global.
fn deref_addr(inst: ir::Inst, func: &mut ir::Function, base: ir::GlobalVar, offset: i64) {
    // We need to load a pointer from the `base` global variable, so insert a new `global_addr`
    // instruction. This depends on the iterative legalization loop. Note that the IL verifier
    // detects any cycles in the `deref` globals.
    let ptr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    let base_addr = pos.ins().global_addr(ptr_ty, base);
    // TODO: We could probably set both `notrap` and `aligned` on this load instruction.
    let base_ptr = pos.ins().load(ptr_ty, ir::MemFlags::new(), base_addr, 0);
    pos.func.dfg.replace(inst).iadd_imm(base_ptr, offset);
}
