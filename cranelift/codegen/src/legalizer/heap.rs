//! Legalization of heaps.
//!
//! This module exports the `expand_heap_addr` function which transforms a `heap_addr`
//! instruction into code that depends on the kind of heap referenced.

use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::condcodes::IntCC;
use crate::ir::{self, InstBuilder};
use crate::isa::TargetIsa;

/// Expand a `heap_addr` instruction according to the definition of the heap.
pub fn expand_heap_addr(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
) {
    // Unpack the instruction.
    let (heap, offset, access_size) = match func.dfg[inst] {
        ir::InstructionData::HeapAddr {
            opcode,
            heap,
            arg,
            imm,
        } => {
            debug_assert_eq!(opcode, ir::Opcode::HeapAddr);
            (heap, arg, imm.into())
        }
        _ => panic!("Wanted heap_addr: {}", func.dfg.display_inst(inst, None)),
    };

    match func.heaps[heap].style {
        ir::HeapStyle::Dynamic { bound_gv } => {
            dynamic_addr(isa, inst, heap, offset, access_size, bound_gv, func)
        }
        ir::HeapStyle::Static { bound } => static_addr(
            isa,
            inst,
            heap,
            offset,
            access_size,
            bound.into(),
            func,
            cfg,
        ),
    }
}

/// Expand a `heap_addr` for a dynamic heap.
fn dynamic_addr(
    isa: &dyn TargetIsa,
    inst: ir::Inst,
    heap: ir::Heap,
    offset: ir::Value,
    access_size: u32,
    bound_gv: ir::GlobalValue,
    func: &mut ir::Function,
) {
    let access_size = u64::from(access_size);
    let offset_ty = func.dfg.value_type(offset);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let min_size = func.heaps[heap].min_size.into();
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start with the bounds check. Trap if `offset + access_size > bound`.
    let bound = pos.ins().global_value(offset_ty, bound_gv);
    let oob;
    if access_size == 1 {
        // `offset > bound - 1` is the same as `offset >= bound`.
        oob = pos
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, offset, bound);
    } else if access_size <= min_size {
        // We know that bound >= min_size, so here we can compare `offset > bound - access_size`
        // without wrapping.
        let adj_bound = pos.ins().iadd_imm(bound, -(access_size as i64));
        oob = pos
            .ins()
            .icmp(IntCC::UnsignedGreaterThan, offset, adj_bound);
    } else {
        // We need an overflow check for the adjusted offset.
        let access_size_val = pos.ins().iconst(offset_ty, access_size as i64);
        let (adj_offset, overflow) = pos.ins().iadd_ifcout(offset, access_size_val);
        pos.ins().trapif(
            isa.unsigned_add_overflow_condition(),
            overflow,
            ir::TrapCode::HeapOutOfBounds,
        );
        oob = pos
            .ins()
            .icmp(IntCC::UnsignedGreaterThan, adj_offset, bound);
    }
    pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);

    compute_addr(isa, inst, heap, addr_ty, offset, offset_ty, pos.func);
}

/// Expand a `heap_addr` for a static heap.
fn static_addr(
    isa: &dyn TargetIsa,
    inst: ir::Inst,
    heap: ir::Heap,
    offset: ir::Value,
    access_size: u32,
    bound: u64,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
) {
    let access_size = u64::from(access_size);
    let offset_ty = func.dfg.value_type(offset);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // The goal here is to trap if `offset + access_size > bound`.
    //
    // This first case is a trivial case where we can easily trap.
    if access_size > bound {
        // This will simply always trap since `offset >= 0`.
        pos.ins().trap(ir::TrapCode::HeapOutOfBounds);
        pos.func.dfg.replace(inst).iconst(addr_ty, 0);

        // Split Block, as the trap is a terminator instruction.
        let curr_block = pos.current_block().expect("Cursor is not in a block");
        let new_block = pos.func.dfg.make_block();
        pos.insert_block(new_block);
        cfg.recompute_block(pos.func, curr_block);
        cfg.recompute_block(pos.func, new_block);
        return;
    }

    // After the trivial case is done we're now mostly interested in trapping
    // if `offset > bound - access_size`. We know `bound - access_size` here is
    // non-negative from the above comparison.
    //
    // If we can know `bound - access_size >= 4GB` then with a 32-bit offset
    // we're guaranteed:
    //
    //      bound - access_size >= 4GB > offset
    //
    // or, in other words, `offset < bound - access_size`, meaning we can't trap
    // for any value of `offset`.
    //
    // With that we have an optimization here where with 32-bit offsets and
    // `bound - access_size >= 4GB` we can omit a bounds check.
    let limit = bound - access_size;
    if offset_ty != ir::types::I32 || limit < 0xffff_ffff {
        let oob = if limit & 1 == 1 {
            // Prefer testing `offset >= limit - 1` when limit is odd because an even number is
            // likely to be a convenient constant on ARM and other RISC architectures.
            pos.ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, offset, limit as i64 - 1)
        } else {
            pos.ins()
                .icmp_imm(IntCC::UnsignedGreaterThan, offset, limit as i64)
        };
        pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
    }

    compute_addr(isa, inst, heap, addr_ty, offset, offset_ty, pos.func);
}

/// Emit code for the base address computation of a `heap_addr` instruction.
fn compute_addr(
    isa: &dyn TargetIsa,
    inst: ir::Inst,
    heap: ir::Heap,
    addr_ty: ir::Type,
    mut offset: ir::Value,
    offset_ty: ir::Type,
    func: &mut ir::Function,
) {
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Convert `offset` to `addr_ty`.
    if offset_ty != addr_ty {
        let labels_value = offset;
        offset = pos.ins().uextend(addr_ty, offset);
        if let Some(values_labels) = pos.func.dfg.values_labels.as_mut() {
            values_labels.insert(
                offset,
                ir::ValueLabelAssignments::Alias {
                    from: pos.func.srclocs[inst],
                    value: labels_value,
                },
            );
        }
    }

    // Add the heap base address base
    let base = if isa.flags().enable_pinned_reg() && isa.flags().use_pinned_reg_as_heap_base() {
        pos.ins().get_pinned_reg(isa.pointer_type())
    } else {
        let base_gv = pos.func.heaps[heap].base;
        pos.ins().global_value(addr_ty, base_gv)
    };

    pos.func.dfg.replace(inst).iadd(base, offset);
}
