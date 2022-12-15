//! Implementation of Wasm to CLIF memory access translation.
//!
//! Given
//!
//! * a dynamic Wasm memory index operand,
//! * a static offset immediate, and
//! * a static access size,
//!
//! bounds check the memory access and translate it into a native memory access.

use crate::{FuncEnvironment, HeapData, HeapStyle};
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{self, condcodes::IntCC, InstBuilder, RelSourceLoc},
};
use cranelift_frontend::FunctionBuilder;
use wasmtime_types::WasmResult;

/// Helper used to emit bounds checks (as necessary) and compute the native
/// address of a heap access.
///
/// Returns the `ir::Value` holding the native address of the heap access, or
/// `None` if the heap access will unconditionally trap.
pub fn bounds_check_and_compute_addr<Env>(
    builder: &mut FunctionBuilder,
    env: &mut Env,
    heap: &HeapData,
    // Dynamic operand indexing into the heap.
    index: ir::Value,
    // Static immediate added to the index.
    offset: u32,
    // Static size of the heap access.
    access_size: u8,
) -> WasmResult<Option<ir::Value>>
where
    Env: FuncEnvironment + ?Sized,
{
    let index = cast_index_to_pointer_ty(
        index,
        heap.index_type,
        env.pointer_type(),
        &mut builder.cursor(),
    );
    let offset_and_size = offset_plus_size(offset, access_size);
    let spectre_mitigations_enabled = env.heap_access_spectre_mitigation();

    // We need to emit code that will trap (or compute an address that will trap
    // when accessed) if
    //
    //     index + offset + access_size > bound
    //
    // or if the `index + offset + access_size` addition overflows.
    //
    // Note that we ultimately want a 64-bit integer (we only target 64-bit
    // architectures at the moment) and that `offset` is a `u32` and
    // `access_size` is a `u8`. This means that we can add the latter together
    // as `u64`s without fear of overflow, and we only have to be concerned with
    // whether adding in `index` will overflow.
    //
    // Finally, the following right-hand sides of the matches do have a little
    // bit of duplicated code across them, but I think writing it this way is
    // worth it for readability and seeing very clearly each of our cases for
    // different bounds checks and optimizations of those bounds checks. It is
    // intentionally written in a straightforward case-matching style that will
    // hopefully make it easy to port to ISLE one day.
    Ok(match heap.style {
        // ====== Dynamic Memories ======
        //
        // 1. First special case for when `offset + access_size == 1`:
        //
        //            index + 1 > bound
        //        ==> index >= bound
        //
        //    1.a. When Spectre mitigations are enabled, avoid duplicating
        //         bounds checks between the mitigations and the regular bounds
        //         checks.
        HeapStyle::Dynamic { bound_gv } if offset_and_size == 1 && spectre_mitigations_enabled => {
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                Some(SpectreOobComparison {
                    cc: IntCC::UnsignedGreaterThanOrEqual,
                    lhs: index,
                    rhs: bound,
                }),
            ))
        }
        //    1.b. Emit explicit `index >= bound` bounds checks.
        HeapStyle::Dynamic { bound_gv } if offset_and_size == 1 => {
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            let oob = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, index, bound);
            builder.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                None,
            ))
        }

        // 2. Second special case for when `offset + access_size <= min_size`.
        //
        //    We know that `bound >= min_size`, so we can do the following
        //    comparison, without fear of the right-hand side wrapping around:
        //
        //            index + offset + access_size > bound
        //        ==> index > bound - (offset + access_size)
        //
        //    2.a. Dedupe bounds checks with Spectre mitigations.
        HeapStyle::Dynamic { bound_gv }
            if offset_and_size <= heap.min_size.into() && spectre_mitigations_enabled =>
        {
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            let adjusted_bound = builder.ins().iadd_imm(bound, -(offset_and_size as i64));
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                Some(SpectreOobComparison {
                    cc: IntCC::UnsignedGreaterThan,
                    lhs: index,
                    rhs: adjusted_bound,
                }),
            ))
        }
        //    2.b. Emit explicit `index > bound - (offset + access_size)` bounds
        //         checks.
        HeapStyle::Dynamic { bound_gv } if offset_and_size <= heap.min_size.into() => {
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            let adjusted_bound = builder.ins().iadd_imm(bound, -(offset_and_size as i64));
            let oob = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThan, index, adjusted_bound);
            builder.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                None,
            ))
        }

        // 3. General case for dynamic memories:
        //
        //        index + offset + access_size > bound
        //
        //    And we have to handle the overflow case in the left-hand side.
        //
        //    3.a. Dedupe bounds checks with Spectre mitigations.
        HeapStyle::Dynamic { bound_gv } if spectre_mitigations_enabled => {
            let access_size_val = builder
                .ins()
                .iconst(env.pointer_type(), offset_and_size as i64);
            let adjusted_index = builder.ins().uadd_overflow_trap(
                index,
                access_size_val,
                ir::TrapCode::HeapOutOfBounds,
            );
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                Some(SpectreOobComparison {
                    cc: IntCC::UnsignedGreaterThan,
                    lhs: adjusted_index,
                    rhs: bound,
                }),
            ))
        }
        //    3.b. Emit an explicit `index + offset + access_size > bound`
        //         check.
        HeapStyle::Dynamic { bound_gv } => {
            let access_size_val = builder
                .ins()
                .iconst(env.pointer_type(), offset_and_size as i64);
            let adjusted_index = builder.ins().uadd_overflow_trap(
                index,
                access_size_val,
                ir::TrapCode::HeapOutOfBounds,
            );
            let bound = builder.ins().global_value(env.pointer_type(), bound_gv);
            let oob = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThan, adjusted_index, bound);
            builder.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                None,
            ))
        }

        // ====== Static Memories ======
        //
        // With static memories we know the size of the heap bound at compile
        // time.
        //
        // 1. First special case: trap immediately if `offset + access_size >
        //    bound`, since we will end up being out-of-bounds regardless of the
        //    given `index`.
        HeapStyle::Static { bound } if offset_and_size > bound.into() => {
            env.before_unconditionally_trapping_memory_access(builder)?;
            builder.ins().trap(ir::TrapCode::HeapOutOfBounds);
            None
        }

        // 2. Second special case for when we can completely omit explicit
        //    bounds checks for 32-bit static memories.
        //
        //    First, let's rewrite our comparison to move all of the constants
        //    to one side:
        //
        //            index + offset + access_size > bound
        //        ==> index > bound - (offset + access_size)
        //
        //    We know the subtraction on the right-hand side won't wrap because
        //    we didn't hit the first special case.
        //
        //    Additionally, we add our guard pages (if any) to the right-hand
        //    side, since we can rely on the virtual memory subsystem at runtime
        //    to catch out-of-bound accesses within the range `bound .. bound +
        //    guard_size`. So now we are dealing with
        //
        //        index > bound + guard_size - (offset + access_size)
        //
        //    Note that `bound + guard_size` cannot overflow for
        //    correctly-configured heaps, as otherwise the heap wouldn't fit in
        //    a 64-bit memory space.
        //
        //    The complement of our should-this-trap comparison expression is
        //    the should-this-not-trap comparison expression:
        //
        //        index <= bound + guard_size - (offset + access_size)
        //
        //    If we know the right-hand side is greater than or equal to
        //    `u32::MAX`, then
        //
        //        index <= u32::MAX <= bound + guard_size - (offset + access_size)
        //
        //    This expression is always true when the heap is indexed with
        //    32-bit integers because `index` cannot be larger than
        //    `u32::MAX`. This means that `index` is always either in bounds or
        //    within the guard page region, neither of which require emitting an
        //    explicit bounds check.
        HeapStyle::Static { bound }
            if heap.index_type == ir::types::I32
                && u64::from(u32::MAX)
                    <= u64::from(bound) + u64::from(heap.offset_guard_size) - offset_and_size =>
        {
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                None,
            ))
        }

        // 3. General case for static memories.
        //
        //    We have to explicitly test whether
        //
        //        index > bound - (offset + access_size)
        //
        //    and trap if so.
        //
        //    Since we have to emit explicit bounds checks, we might as well be
        //    precise, not rely on the virtual memory subsystem at all, and not
        //    factor in the guard pages here.
        //
        //    3.a. Dedupe the Spectre mitigation and the explicit bounds check.
        HeapStyle::Static { bound } if spectre_mitigations_enabled => {
            // NB: this subtraction cannot wrap because we didn't hit the first
            // special case.
            let adjusted_bound = u64::from(bound) - offset_and_size;
            let adjusted_bound = builder
                .ins()
                .iconst(env.pointer_type(), adjusted_bound as i64);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                Some(SpectreOobComparison {
                    cc: IntCC::UnsignedGreaterThan,
                    lhs: index,
                    rhs: adjusted_bound,
                }),
            ))
        }
        //    3.b. Emit the explicit `index > bound - (offset + access_size)`
        //         check.
        HeapStyle::Static { bound } => {
            // See comment in 3.a. above.
            let adjusted_bound = u64::from(bound) - offset_and_size;
            let oob =
                builder
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThan, index, adjusted_bound as i64);
            builder.ins().trapnz(oob, ir::TrapCode::HeapOutOfBounds);
            Some(compute_addr(
                &mut builder.cursor(),
                heap,
                env.pointer_type(),
                index,
                offset,
                None,
            ))
        }
    })
}

fn cast_index_to_pointer_ty(
    index: ir::Value,
    index_ty: ir::Type,
    pointer_ty: ir::Type,
    pos: &mut FuncCursor,
) -> ir::Value {
    if index_ty == pointer_ty {
        return index;
    }
    // Note that using 64-bit heaps on a 32-bit host is not currently supported,
    // would require at least a bounds check here to ensure that the truncation
    // from 64-to-32 bits doesn't lose any upper bits. For now though we're
    // mostly interested in the 32-bit-heaps-on-64-bit-hosts cast.
    assert!(index_ty.bits() < pointer_ty.bits());

    // Convert `index` to `addr_ty`.
    let extended_index = pos.ins().uextend(pointer_ty, index);

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

/// Emit code for the base address computation of a `heap_addr` instruction,
/// without any bounds checks (other than optional Spectre mitigations).
fn compute_addr(
    pos: &mut FuncCursor,
    heap: &HeapData,
    addr_ty: ir::Type,
    index: ir::Value,
    offset: u32,
    // If we are performing Spectre mitigation with conditional selects, the
    // values to compare and the condition code that indicates an out-of bounds
    // condition; on this condition, the conditional move will choose a
    // speculatively safe address (a zero / null pointer) instead.
    spectre_oob_comparison: Option<SpectreOobComparison>,
) -> ir::Value {
    debug_assert_eq!(pos.func.dfg.value_type(index), addr_ty);

    // Add the heap base address base
    let base = pos.ins().global_value(addr_ty, heap.base);

    let final_base = pos.ins().iadd(base, index);
    let final_addr = if offset == 0 {
        final_base
    } else {
        // NB: The addition of the offset immediate must happen *before* the
        // `select_spectre_guard`. If it happens after, then we potentially are
        // letting speculative execution read the whole first 4GiB of memory.
        pos.ins().iadd_imm(final_base, offset as i64)
    };

    if let Some(SpectreOobComparison { cc, lhs, rhs }) = spectre_oob_comparison {
        let null = pos.ins().iconst(addr_ty, 0);
        let cmp = pos.ins().icmp(cc, lhs, rhs);
        pos.ins().select_spectre_guard(cmp, null, final_addr)
    } else {
        final_addr
    }
}

#[inline]
fn offset_plus_size(offset: u32, size: u8) -> u64 {
    // Cannot overflow because we are widening to `u64`.
    offset as u64 + size as u64
}
