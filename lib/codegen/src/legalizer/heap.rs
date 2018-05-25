//! Legalization of heaps.
//!
//! This module exports the `expand_heap_addr` function which transforms a `heap_addr`
//! instruction into code that depends on the kind of heap referenced.

use cursor::{Cursor, FuncCursor};
use flowgraph::ControlFlowGraph;
use ir::condcodes::IntCC;
use ir::{self, InstBuilder, MemFlags};
use isa::TargetIsa;

/// Expand a `heap_addr` instruction according to the definition of the heap.
pub fn expand_heap_addr(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    _isa: &TargetIsa,
) {
    // Unpack the instruction.
    let (heap, offset, size) = match func.dfg[inst] {
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
            dynamic_addr(inst, heap, offset, size, bound_gv, func)
        }
        ir::HeapStyle::Static { bound } => {
            static_addr(inst, heap, offset, size, bound.into(), func, cfg)
        }
    }
}

/// Expand a `heap_addr` for a dynamic heap.
fn dynamic_addr(
    inst: ir::Inst,
    heap: ir::Heap,
    offset: ir::Value,
    size: u32,
    bound_gv: ir::GlobalVar,
    func: &mut ir::Function,
) {
    let size = i64::from(size);
    let offset_ty = func.dfg.value_type(offset);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let min_size = func.heaps[heap].min_size.into();
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start with the bounds check. Trap if `offset + size > bound`.
    let bound_addr = pos.ins().global_addr(addr_ty, bound_gv);
    let mut mflags = MemFlags::new();
    // The bound variable is requied to be accessible and aligned.
    mflags.set_notrap();
    mflags.set_aligned();
    let bound = pos.ins().load(offset_ty, mflags, bound_addr, 0);

    let oob;
    if size == 1 {
        // `offset > bound - 1` is the same as `offset >= bound`.
        oob = pos.ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, offset, bound);
    } else if size <= min_size {
        // We know that bound >= min_size, so here we can compare `offset > bound - size` without
        // wrapping.
        let adj_bound = pos.ins().iadd_imm(bound, -size);
        oob = pos.ins()
            .icmp(IntCC::UnsignedGreaterThan, offset, adj_bound);
    } else {
        // We need an overflow check for the adjusted offset.
        let size_val = pos.ins().iconst(offset_ty, size);
        let (adj_offset, overflow) = pos.ins().iadd_cout(offset, size_val);
        pos.ins().trapnz(overflow, ir::TrapCode::HeapOutOfBounds);
        oob = pos.ins()
            .icmp(IntCC::UnsignedGreaterThan, adj_offset, bound);
    }
    pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);

    offset_addr(inst, heap, addr_ty, offset, offset_ty, pos.func);
}

/// Expand a `heap_addr` for a static heap.
fn static_addr(
    inst: ir::Inst,
    heap: ir::Heap,
    offset: ir::Value,
    size: u32,
    bound: i64,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
) {
    let size = i64::from(size);
    let offset_ty = func.dfg.value_type(offset);
    let addr_ty = func.dfg.value_type(func.dfg.first_result(inst));
    let mut pos = FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start with the bounds check. Trap if `offset + size > bound`.
    if size > bound {
        // This will simply always trap since `offset >= 0`.
        pos.ins().trap(ir::TrapCode::HeapOutOfBounds);
        pos.func.dfg.replace(inst).iconst(addr_ty, 0);

        // Split Ebb, as the trap is a terminator instruction.
        let curr_ebb = pos.current_ebb().expect("Cursor is not in an ebb");
        let new_ebb = pos.func.dfg.make_ebb();
        pos.insert_ebb(new_ebb);
        cfg.recompute_ebb(pos.func, curr_ebb);
        cfg.recompute_ebb(pos.func, new_ebb);
        return;
    }

    // Check `offset > limit` which is now known non-negative.
    let limit = bound - size;

    // We may be able to omit the check entirely for 32-bit offsets if the heap bound is 4 GB or
    // more.
    if offset_ty != ir::types::I32 || limit < 0xffff_ffff {
        let oob = if limit & 1 == 1 {
            // Prefer testing `offset >= limit - 1` when limit is odd because an even number is
            // likely to be a convenient constant on ARM and other RISC architectures.
            pos.ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, offset, limit - 1)
        } else {
            pos.ins()
                .icmp_imm(IntCC::UnsignedGreaterThan, offset, limit)
        };
        pos.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
    }

    offset_addr(inst, heap, addr_ty, offset, offset_ty, pos.func);
}

/// Emit code for the base address computation of a `heap_addr` instruction.
///
///
fn offset_addr(
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
        offset = pos.ins().uextend(addr_ty, offset);
    }

    // Add the heap base address base
    match pos.func.heaps[heap].base {
        ir::HeapBase::ReservedReg => unimplemented!(),
        ir::HeapBase::GlobalVar(base_gv) => {
            let base_addr = pos.ins().global_addr(addr_ty, base_gv);
            let mut mflags = MemFlags::new();
            // The base address variable is requied to be accessible and aligned.
            mflags.set_notrap();
            mflags.set_aligned();
            let base = pos.ins().load(addr_ty, mflags, base_addr, 0);
            pos.func.dfg.replace(inst).iadd(base, offset);
        }
    }
}
