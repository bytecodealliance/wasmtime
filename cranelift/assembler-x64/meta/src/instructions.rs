//! Defines x64 instructions using the DSL.

mod add;
mod and;
mod avg;
mod bitmanip;
mod cvt;
mod div;
mod lanes;
mod max;
mod min;
mod mov;
mod mul;
mod neg;
mod or;
mod round;
mod shift;
mod sqrt;
mod sub;
mod unpack;
mod xor;

use crate::dsl::{Feature, Inst, Mutability, OperandKind};
use std::collections::HashMap;

#[must_use]
pub fn list() -> Vec<Inst> {
    let mut all = vec![];
    all.extend(add::list());
    all.extend(and::list());
    all.extend(avg::list());
    all.extend(bitmanip::list());
    all.extend(cvt::list());
    all.extend(div::list());
    all.extend(lanes::list());
    all.extend(max::list());
    all.extend(min::list());
    all.extend(mov::list());
    all.extend(mul::list());
    all.extend(neg::list());
    all.extend(or::list());
    all.extend(round::list());
    all.extend(shift::list());
    all.extend(sqrt::list());
    all.extend(sub::list());
    all.extend(xor::list());
    all.extend(unpack::list());

    // Automatically assign AVX alternates to SSE instructions (see
    // `Inst::alternate`). This allows later code generation to
    // instruction-select between the AVX and SSE versions of an instruction.
    assign_avx_alternates(&mut all);

    all
}

/// Assigns AVX alternates to SSE instructions.
///
/// This works by:
/// - finding the mnemonics of all AVX instructions
/// - finding the mnemonics of all SSE* instructions
/// - looking up each AVX mnemonic, minus its 'v' prefix, to see if an SSE
///   version exists
/// - checking that the SSE and AVX instructions have the same opcode
/// - assigning the AVX instruction name as the alternate for the SSE
///   instruction
fn assign_avx_alternates(all: &mut [Inst]) {
    let sse = map_mnemonic_to_index(&all, is_sse);
    let avx = map_mnemonic_to_index(&all, is_avx);

    for (avx_mnemonic, avx_index) in avx {
        let sse_mnemonic = &avx_mnemonic[1..]; // Remove the 'v' prefix.
        if let Some(sse_index) = sse.get(sse_mnemonic) {
            if let Some(avx_name) = sse_matches_avx(*sse_index, avx_index, all) {
                let sse_inst = &mut all[*sse_index];
                sse_inst.alternate = Some(avx_name);
            }
        }
    }
}

/// Check if `inst` is an SSE instructions (any SSE-based feature).
fn is_sse((_, inst): &(usize, &Inst)) -> bool {
    inst.features.contains(Feature::sse)
        || inst.features.contains(Feature::sse2)
        || inst.features.contains(Feature::ssse3)
        || inst.features.contains(Feature::sse41)
}

/// Check if `inst` is an AVX instructions.
fn is_avx((_, inst): &(usize, &Inst)) -> bool {
    inst.features.contains(Feature::avx)
}

/// Create a map of an instruction mnemonic to its index in `insts`; this speeds
/// up lookups and avoids borrowing from a `Vec` we are attempting to modify.
fn map_mnemonic_to_index(
    insts: &[Inst],
    filter: impl Fn(&(usize, &Inst)) -> bool,
) -> HashMap<String, usize> {
    insts
        .iter()
        .enumerate()
        .filter(filter)
        .map(|(index, inst)| (inst.mnemonic.clone(), index))
        .collect()
}

/// Checks if the SSE instruction at `sse_index` matches the AVX instruction at
/// `avx_index` in terms of operands and opcode, and returns the AVX mnemonic if
/// they match.
fn sse_matches_avx(sse_index: usize, avx_index: usize, insts: &[Inst]) -> Option<String> {
    use crate::dsl::{Mutability::*, OperandKind::*};

    let sse_inst = &insts[sse_index];
    let avx_inst = &insts[avx_index];

    // Just to double-check:
    debug_assert_eq!(&format!("v{}", sse_inst.mnemonic), &avx_inst.mnemonic);

    if sse_inst.encoding.opcode() != avx_inst.encoding.opcode() {
        panic!("the following instructions should have the same opcode:\n{sse_inst}\n{avx_inst}");
    }

    match (list_ops(sse_inst).as_slice(), list_ops(avx_inst).as_slice()) {
        // For now, we only really want to tie together SSE instructions that
        // look like `rw(xmm), r(xmm_m*)` with their AVX counterpart that looks
        // like `w(xmm), r(xmm), r(xmm_m*)`. This is because the relationship
        // between these kinds of instructions is quite regular. Other formats
        // may have slightly different operand semantics (e.g., `roundss` ->
        // `vroundss`) and we want to be careful about matching too freely.
        (
            [(ReadWrite, Reg(_)), (Read, RegMem(_))],
            [(Write, Reg(_)), (Read, Reg(_)), (Read, RegMem(_))],
        ) => Some(avx_inst.name()),
        // We ignore other formats for now.
        _ => None,
    }
}

/// Collect the mutability and kind of each operand in an instruction.
fn list_ops(inst: &Inst) -> Vec<(Mutability, OperandKind)> {
    inst.format
        .operands
        .iter()
        .map(|o| (o.mutability, o.location.kind()))
        .collect()
}
