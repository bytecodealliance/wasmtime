//! Legalization of heaps.
//!
//! This module exports the `expand_heap_addr` function which transforms a `heap_addr`
//! instruction into code that depends on the kind of heap referenced.

use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::condcodes::IntCC;
use crate::ir::immediates::{Uimm32, Uimm8};
use crate::ir::{self, InstBuilder, RelSourceLoc};
use crate::isa::TargetIsa;
use crate::trace;

/// Expand a `heap_addr` instruction according to the definition of the heap.
pub fn expand_heap_addr(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    isa: &dyn TargetIsa,
    heap: ir::Heap,
    index_operand: ir::Value,
    offset_immediate: Uimm32,
    access_size: Uimm8,
) {
    trace!(
        "expanding heap_addr: {:?}: {}",
        inst,
        func.dfg.display_inst(inst)
    );

    let ir::HeapData {
        offset_guard_size,
        style,
        ..
    } = &func.heaps[heap];
    match *style {
        ir::HeapStyle::Dynamic { bound_gv } => dynamic_addr(
            isa,
            inst,
            heap,
            index_operand,
            u32::from(offset_immediate),
            u8::from(access_size),
            bound_gv,
            func,
        ),
        ir::HeapStyle::Static { bound } => static_addr(
            isa,
            inst,
            heap,
            index_operand,
            u32::from(offset_immediate),
            u8::from(access_size),
            bound.into(),
            (*offset_guard_size).into(),
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
    index: ir::Value,
    offset: u32,
    access_size: u8,
    bound_gv: ir::GlobalValue,
    func: &mut ir::Function,
) {
    let index_ty = func.dfg.value_type(index);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let min_size = func.heaps[heap].min_size.into();
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    let index = cast_index_to_pointer_ty(index, index_ty, addr_ty, &mut pos);

    // Start with the bounds check. Trap if `index + offset + access_size > bound`.
    let bound = pos.ins().global_value(addr_ty, bound_gv);
    let (cc, lhs, bound) = if offset == 0 && access_size == 1 {
        // `index > bound - 1` is the same as `index >= bound`.
        (IntCC::UnsignedGreaterThanOrEqual, index, bound)
    } else if offset_plus_size(offset, access_size) <= min_size {
        // We know that `bound >= min_size`, so here we can compare `offset >
        // bound - (offset + access_size)` without wrapping.
        let adj_bound = pos
            .ins()
            .iadd_imm(bound, -(offset_plus_size(offset, access_size) as i64));
        trace!(
            "  inserting: {}",
            pos.func.dfg.display_value_inst(adj_bound)
        );
        (IntCC::UnsignedGreaterThan, index, adj_bound)
    } else {
        // We need an overflow check for the adjusted offset.
        let access_size_val = pos
            .ins()
            .iconst(addr_ty, offset_plus_size(offset, access_size) as i64);
        let adj_offset =
            pos.ins()
                .uadd_overflow_trap(index, access_size_val, ir::TrapCode::HeapOutOfBounds);
        trace!(
            "  inserting: {}",
            pos.func.dfg.display_value_inst(adj_offset)
        );
        (IntCC::UnsignedGreaterThan, adj_offset, bound)
    };

    let spectre_oob_comparison = if isa.flags().enable_heap_access_spectre_mitigation() {
        // When we emit a spectre-guarded heap access, we do a `select
        // is_out_of_bounds, NULL, addr` to compute the address, and so the load
        // will trap if the address is out of bounds, which means we don't need
        // to do another explicit bounds check like we do below.
        Some(SpectreOobComparison {
            cc,
            lhs,
            rhs: bound,
        })
    } else {
        let oob = pos.ins().icmp(cc, lhs, bound);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(oob));

        let trapnz = pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
        trace!("  inserting: {}", pos.func.dfg.display_inst(trapnz));

        None
    };

    compute_addr(
        isa,
        inst,
        heap,
        addr_ty,
        index,
        offset,
        pos.func,
        spectre_oob_comparison,
    );
}

/// Expand a `heap_addr` for a static heap.
fn static_addr(
    isa: &dyn TargetIsa,
    inst: ir::Inst,
    heap: ir::Heap,
    index: ir::Value,
    offset: u32,
    access_size: u8,
    bound: u64,
    guard_size: u64,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
) {
    let index_ty = func.dfg.value_type(index);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // The goal here is to trap if `index + offset + access_size > bound`.
    //
    // This first case is a trivial case where we can statically trap.
    if offset_plus_size(offset, access_size) > bound {
        // This will simply always trap since `offset >= 0`.
        let trap = pos.ins().trap(ir::TrapCode::HeapOutOfBounds);
        trace!("  inserting: {}", pos.func.dfg.display_inst(trap));
        let iconst = pos.func.dfg.replace(inst).iconst(addr_ty, 0);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(iconst));

        // Split the block, as the trap is a terminator instruction.
        let curr_block = pos.current_block().expect("Cursor is not in a block");
        let new_block = pos.func.dfg.make_block();
        pos.insert_block(new_block);
        cfg.recompute_block(pos.func, curr_block);
        cfg.recompute_block(pos.func, new_block);
        return;
    }

    // After the trivial case is done we're now mostly interested in trapping if
    //
    //     index + offset + size > bound
    //
    // We know `bound - offset - access_size` here is non-negative from the
    // above comparison, so we can rewrite that as
    //
    //     index > bound - offset - size
    //
    // Additionally, we add our guard pages (if any) to the right-hand side,
    // since we can rely on the virtual memory subsystem at runtime to catch
    // out-of-bound accesses within the range `bound .. bound + guard_size`. So
    // now we are dealing with
    //
    //     index > bound + guard_size - offset - size
    //
    // (Note that `bound + guard_size` cannot overflow for correctly-configured
    // heaps, as otherwise the heap wouldn't fit in a 64-bit memory space.)
    //
    // If we know the right-hand side is greater than or equal to 4GiB - 1, aka
    // 0xffff_ffff, then with a 32-bit index we're guaranteed:
    //
    //     index <= 0xffff_ffff <= bound + guard_size - offset - access_size
    //
    // meaning that `index` is always either in bounds or within the guard page
    // region, neither of which require emitting an explicit bounds check.
    assert!(
        bound.checked_add(guard_size).is_some(),
        "heap's configuration doesn't fit in a 64-bit memory space"
    );
    let mut spectre_oob_comparison = None;
    let index = cast_index_to_pointer_ty(index, index_ty, addr_ty, &mut pos);
    if index_ty == ir::types::I32
        && bound + guard_size - offset_plus_size(offset, access_size) >= 0xffff_ffff
    {
        // Happy path! No bounds checks necessary!
    } else {
        // Since we have to emit explicit bounds checks anyways, ignore the
        // guard pages and test against the precise limit.
        let limit = bound - offset_plus_size(offset, access_size);
        // Here we want to test the condition `index > limit` and if that's true
        // then this is an out-of-bounds access and needs to trap.
        let oob = pos
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThan, index, limit as i64);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(oob));

        let trapnz = pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
        trace!("  inserting: {}", pos.func.dfg.display_inst(trapnz));

        if isa.flags().enable_heap_access_spectre_mitigation() {
            let limit = pos.ins().iconst(addr_ty, limit as i64);
            trace!("  inserting: {}", pos.func.dfg.display_value_inst(limit));
            spectre_oob_comparison = Some(SpectreOobComparison {
                cc: IntCC::UnsignedGreaterThan,
                lhs: index,
                rhs: limit,
            });
        }
    }

    compute_addr(
        isa,
        inst,
        heap,
        addr_ty,
        index,
        offset,
        pos.func,
        spectre_oob_comparison,
    );
}

fn cast_index_to_pointer_ty(
    index: ir::Value,
    index_ty: ir::Type,
    addr_ty: ir::Type,
    pos: &mut FuncCursor,
) -> ir::Value {
    if index_ty == addr_ty {
        return index;
    }
    // Note that using 64-bit heaps on a 32-bit host is not currently supported,
    // would require at least a bounds check here to ensure that the truncation
    // from 64-to-32 bits doesn't lose any upper bits. For now though we're
    // mostly interested in the 32-bit-heaps-on-64-bit-hosts cast.
    assert!(index_ty.bits() < addr_ty.bits());

    // Convert `index` to `addr_ty`.
    let extended_index = pos.ins().uextend(addr_ty, index);

    // Add debug value-label alias so that debuginfo can name the extended
    // value as the address
    let loc = pos.srcloc();
    let loc = RelSourceLoc::from_base_offset(pos.func.params.base_srcloc(), loc);
    pos.func
        .stencil
        .dfg
        .add_value_label_alias(extended_index, loc, index);

    extended_index
}

struct SpectreOobComparison {
    cc: IntCC,
    lhs: ir::Value,
    rhs: ir::Value,
}

/// Emit code for the base address computation of a `heap_addr` instruction.
fn compute_addr(
    isa: &dyn TargetIsa,
    inst: ir::Inst,
    heap: ir::Heap,
    addr_ty: ir::Type,
    index: ir::Value,
    offset: u32,
    func: &mut ir::Function,
    // If we are performing Spectre mitigation with conditional selects, the
    // values to compare and the condition code that indicates an out-of bounds
    // condition; on this condition, the conditional move will choose a
    // speculatively safe address (a zero / null pointer) instead.
    spectre_oob_comparison: Option<SpectreOobComparison>,
) {
    debug_assert_eq!(func.dfg.value_type(index), addr_ty);
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Add the heap base address base
    let base = if isa.flags().enable_pinned_reg() && isa.flags().use_pinned_reg_as_heap_base() {
        let base = pos.ins().get_pinned_reg(isa.pointer_type());
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(base));
        base
    } else {
        let base_gv = pos.func.heaps[heap].base;
        let base = pos.ins().global_value(addr_ty, base_gv);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(base));
        base
    };

    if let Some(SpectreOobComparison { cc, lhs, rhs }) = spectre_oob_comparison {
        let final_base = pos.ins().iadd(base, index);
        // NB: The addition of the offset immediate must happen *before* the
        // `select_spectre_guard`. If it happens after, then we potentially are
        // letting speculative execution read the whole first 4GiB of memory.
        let final_addr = if offset == 0 {
            final_base
        } else {
            let final_addr = pos.ins().iadd_imm(final_base, offset as i64);
            trace!(
                "  inserting: {}",
                pos.func.dfg.display_value_inst(final_addr)
            );
            final_addr
        };
        let zero = pos.ins().iconst(addr_ty, 0);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(zero));

        let cmp = pos.ins().icmp(cc, lhs, rhs);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(cmp));

        let value = pos
            .func
            .dfg
            .replace(inst)
            .select_spectre_guard(cmp, zero, final_addr);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(value));
    } else if offset == 0 {
        let addr = pos.func.dfg.replace(inst).iadd(base, index);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(addr));
    } else {
        let final_base = pos.ins().iadd(base, index);
        trace!(
            "  inserting: {}",
            pos.func.dfg.display_value_inst(final_base)
        );
        let addr = pos
            .func
            .dfg
            .replace(inst)
            .iadd_imm(final_base, offset as i64);
        trace!("  inserting: {}", pos.func.dfg.display_value_inst(addr));
    }
}

fn offset_plus_size(offset: u32, size: u8) -> u64 {
    // Cannot overflow because we are widening to `u64`.
    offset as u64 + size as u64
}
