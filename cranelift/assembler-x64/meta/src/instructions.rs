//! Defines x64 instructions using the DSL.

mod add;
mod and;
mod avg;
mod bitmanip;
mod cmp;
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
mod stack;
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
    all.extend(cmp::list());
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
    all.extend(stack::list());
    all.extend(sub::list());
    all.extend(xor::list());
    all.extend(unpack::list());

    check_avx_alternates(&mut all);

    all
}

/// Checks that assigned AVX alternates are correctly applied to SSE
/// instructions.
///
/// # Panics
///
/// Expects that each AVX alternate to be of an SSE instruction (currently).
fn check_avx_alternates(all: &mut [Inst]) {
    let name_to_index: HashMap<String, usize> = all
        .iter()
        .enumerate()
        .map(|(index, inst)| (inst.name().clone(), index))
        .collect();
    for inst in all.iter().filter(|inst| inst.alternate.is_some()) {
        assert!(inst.features.is_sse());
        let alternate = inst.alternate.as_ref().unwrap();
        assert_eq!(alternate.feature, Feature::avx);
        let avx_index = name_to_index.get(&alternate.name).expect(&format!(
            "invalid alternate name: {} (did you use the full `<mnemonic>_<format>` form?)",
            alternate.name
        ));
        check_sse_matches_avx(inst, &all[*avx_index]);
    }
}

/// Checks if the SSE instruction `sse_inst` matches the AVX instruction
/// `avx_inst` in terms of operands and opcode.
///
/// # Panics
///
/// Panics for any condition indicating that the SSE and AVX instructions do not
/// match:
/// - the AVX instruction does not have a 'v' prefix
/// - the SSE and AVX instructions do not have the same opcode
/// - the operand formats do not match the expected patterns
fn check_sse_matches_avx(sse_inst: &Inst, avx_inst: &Inst) {
    use crate::dsl::{Mutability::*, OperandKind::*};

    debug_assert_eq!(
        &format!("v{}", sse_inst.mnemonic),
        &avx_inst.mnemonic,
        "an alternate AVX instruction should have a 'v' prefix: {avx_inst}"
    );

    if sse_inst.encoding.opcode() != avx_inst.encoding.opcode() {
        panic!("alternate instructions should have the same opcode:\n{sse_inst}\n{avx_inst}");
    }

    match (list_ops(sse_inst).as_slice(), list_ops(avx_inst).as_slice()) {
        // For now, we only really want to tie together SSE instructions that
        // look like `rw(xmm), r(xmm_m*)` with their AVX counterpart that looks
        // like `w(xmm), r(xmm), r(xmm_m*)`. This is because the relationship
        // between these kinds of instructions is quite regular. Other formats
        // may have slightly different operand semantics (e.g., `roundss` ->
        // `vroundss`) and we want to be careful about matching too freely.
        (
            [(ReadWrite, Reg(_)), (Read, Reg(_) | RegMem(_) | Mem(_))],
            [
                (Write, Reg(_)),
                (Read, Reg(_)),
                (Read, Reg(_) | RegMem(_) | Mem(_)),
            ],
        ) => {}
        // We panic on other formats for now; feel free to add more patterns to
        // avoid this.
        _ => panic!("unmatched formats for SSE-to-AVX alternate:\n{sse_inst}\n{avx_inst}"),
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
