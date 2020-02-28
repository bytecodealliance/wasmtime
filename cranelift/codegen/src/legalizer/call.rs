//! Legalization of calls.
//!
//! This module exports the `expand_call` function which transforms a `call`
//! instruction into `func_addr` and `call_indirect` instructions.

use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{self, InstBuilder};
use crate::isa::TargetIsa;

/// Expand a `call` instruction. This lowers it to a `call_indirect`, which
/// is only done if the ABI doesn't support direct calls.
pub fn expand_call(
    inst: ir::Inst,
    func: &mut ir::Function,
    _cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    // Unpack the instruction.
    let (func_ref, old_args) = match func.dfg[inst] {
        ir::InstructionData::Call {
            opcode,
            ref args,
            func_ref,
        } => {
            debug_assert_eq!(opcode, ir::Opcode::Call);
            (func_ref, args.clone())
        }
        _ => panic!("Wanted call: {}", func.dfg.display_inst(inst, None)),
    };

    let ptr_ty = isa.pointer_type();

    let sig = func.dfg.ext_funcs[func_ref].signature;

    let callee = {
        let mut pos = FuncCursor::new(func).at_inst(inst);
        pos.use_srcloc(inst);
        pos.ins().func_addr(ptr_ty, func_ref)
    };

    let mut new_args = ir::ValueList::default();
    new_args.push(callee, &mut func.dfg.value_lists);
    for i in 0..old_args.len(&func.dfg.value_lists) {
        new_args.push(
            old_args.as_slice(&func.dfg.value_lists)[i],
            &mut func.dfg.value_lists,
        );
    }

    func.dfg
        .replace(inst)
        .CallIndirect(ir::Opcode::CallIndirect, ptr_ty, sig, new_args);
}
