//! Branch relaxation and offset computation.
//!
//! # EBB header offsets
//!
//! Before we can generate binary machine code for branch instructions, we need to know the final
//! offsets of all the EBB headers in the function. This information is encoded in the
//! `func.offsets` table.
//!
//! # Branch relaxation
//!
//! Branch relaxation is the process of ensuring that all branches in the function have enough
//! range to encode their destination. It is common to have multiple branch encodings in an ISA.
//! For example, x86 branches can have either an 8-bit or a 32-bit displacement.
//!
//! On RISC architectures, it can happen that conditional branches have a shorter range than
//! unconditional branches:
//!
//! ```clif
//!     brz v1, ebb17
//! ```
//!
//! can be transformed into:
//!
//! ```clif
//!     brnz v1, ebb23
//!     jump ebb17
//! ebb23:
//! ```

use crate::binemit::CodeOffset;
use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{Function, InstructionData, Opcode};
use crate::isa::{EncInfo, TargetIsa};
use crate::iterators::IteratorExtras;
use crate::regalloc::RegDiversions;
use crate::timing;
use crate::CodegenResult;
use log::debug;

/// Relax branches and compute the final layout of EBB headers in `func`.
///
/// Fill in the `func.offsets` table so the function is ready for binary emission.
pub fn relax_branches(func: &mut Function, isa: &TargetIsa) -> CodegenResult<CodeOffset> {
    let _tt = timing::relax_branches();

    let encinfo = isa.encoding_info();

    // Clear all offsets so we can recognize EBBs that haven't been visited yet.
    func.offsets.clear();
    func.offsets.resize(func.dfg.num_ebbs());

    // Start by inserting fall through instructions.
    fallthroughs(func);

    let mut offset = 0;
    let mut divert = RegDiversions::new();

    // First, compute initial offsets for every EBB.
    {
        let mut cur = FuncCursor::new(func);
        while let Some(ebb) = cur.next_ebb() {
            divert.clear();
            cur.func.offsets[ebb] = offset;
            while let Some(inst) = cur.next_inst() {
                divert.apply(&cur.func.dfg[inst]);
                let enc = cur.func.encodings[inst];
                offset += encinfo.byte_size(enc, inst, &divert, &cur.func);
            }
        }
    }

    // Then, run the relaxation algorithm until it converges.
    let mut go_again = true;
    while go_again {
        go_again = false;
        offset = 0;

        // Visit all instructions in layout order.
        let mut cur = FuncCursor::new(func);
        while let Some(ebb) = cur.next_ebb() {
            divert.clear();

            // Record the offset for `ebb` and make sure we iterate until offsets are stable.
            if cur.func.offsets[ebb] != offset {
                cur.func.offsets[ebb] = offset;
                go_again = true;
            }

            while let Some(inst) = cur.next_inst() {
                divert.apply(&cur.func.dfg[inst]);

                let enc = cur.func.encodings[inst];

                // See if this is a branch has a range and a destination, and if the target is in
                // range.
                if let Some(range) = encinfo.branch_range(enc) {
                    if let Some(dest) = cur.func.dfg[inst].branch_destination() {
                        let dest_offset = cur.func.offsets[dest];
                        if !range.contains(offset, dest_offset) {
                            offset +=
                                relax_branch(&mut cur, &divert, offset, dest_offset, &encinfo, isa);
                            continue;
                        }
                    }
                }

                offset += encinfo.byte_size(enc, inst, &divert, &cur.func);
            }
        }
    }

    for (jt, jt_data) in func.jump_tables.iter() {
        func.jt_offsets[jt] = offset;
        // TODO: this should be computed based on the min size needed to hold
        //        the furthest branch.
        offset += jt_data.len() as u32 * 4;
    }

    Ok(offset)
}

/// Convert `jump` instructions to `fallthrough` instructions where possible and verify that any
/// existing `fallthrough` instructions are correct.
fn fallthroughs(func: &mut Function) {
    for (ebb, succ) in func.layout.ebbs().adjacent_pairs() {
        let term = func.layout.last_inst(ebb).expect("EBB has no terminator.");
        if let InstructionData::Jump {
            ref mut opcode,
            destination,
            ..
        } = func.dfg[term]
        {
            match *opcode {
                Opcode::Fallthrough => {
                    // Somebody used a fall-through instruction before the branch relaxation pass.
                    // Make sure it is correct, i.e. the destination is the layout successor.
                    debug_assert_eq!(destination, succ, "Illegal fall-through in {}", ebb)
                }
                Opcode::Jump => {
                    // If this is a jump to the successor EBB, change it to a fall-through.
                    if destination == succ {
                        *opcode = Opcode::Fallthrough;
                        func.encodings[term] = Default::default();
                    }
                }
                _ => {}
            }
        }
    }
}

/// Relax the branch instruction at `cur` so it can cover the range `offset - dest_offset`.
///
/// Return the size of the replacement instructions up to and including the location where `cur` is
/// left.
fn relax_branch(
    cur: &mut FuncCursor,
    divert: &RegDiversions,
    offset: CodeOffset,
    dest_offset: CodeOffset,
    encinfo: &EncInfo,
    isa: &TargetIsa,
) -> CodeOffset {
    let inst = cur.current_inst().unwrap();
    debug!(
        "Relaxing [{}] {} for {:#x}-{:#x} range",
        encinfo.display(cur.func.encodings[inst]),
        cur.func.dfg.display_inst(inst, isa),
        offset,
        dest_offset
    );

    // Pick the smallest encoding that can handle the branch range.
    let dfg = &cur.func.dfg;
    let ctrl_type = dfg.ctrl_typevar(inst);
    if let Some(enc) = isa
        .legal_encodings(cur.func, &dfg[inst], ctrl_type)
        .filter(|&enc| {
            let range = encinfo.branch_range(enc).expect("Branch with no range");
            if !range.contains(offset, dest_offset) {
                debug!("  trying [{}]: out of range", encinfo.display(enc));
                false
            } else if encinfo.operand_constraints(enc)
                != encinfo.operand_constraints(cur.func.encodings[inst])
            {
                // Conservatively give up if the encoding has different constraints
                // than the original, so that we don't risk picking a new encoding
                // which the existing operands don't satisfy. We can't check for
                // validity directly because we don't have a RegDiversions active so
                // we don't know which registers are actually in use.
                debug!("  trying [{}]: constraints differ", encinfo.display(enc));
                false
            } else {
                debug!("  trying [{}]: OK", encinfo.display(enc));
                true
            }
        })
        .min_by_key(|&enc| encinfo.byte_size(enc, inst, &divert, &cur.func))
    {
        debug_assert!(enc != cur.func.encodings[inst]);
        cur.func.encodings[inst] = enc;
        return encinfo.byte_size(enc, inst, &divert, &cur.func);
    }

    // Note: On some RISC ISAs, conditional branches have shorter range than unconditional
    // branches, so one way of extending the range of a conditional branch is to invert its
    // condition and make it branch over an unconditional jump which has the larger range.
    //
    // Splitting the EBB is problematic this late because there may be register diversions in
    // effect across the conditional branch, and they can't survive the control flow edge to a new
    // EBB. We have two options for handling that:
    //
    // 1. Set a flag on the new EBB that indicates it wants the preserve the register diversions of
    //    its layout predecessor, or
    // 2. Use an encoding macro for the branch-over-jump pattern so we don't need to split the EBB.
    //
    // It seems that 1. would allow us to share code among RISC ISAs that need this.
    //
    // We can't allow register diversions to survive from the layout predecessor because the layout
    // predecessor could contain kill points for some values that are live in this EBB, and
    // diversions are not automatically cancelled when the live range of a value ends.

    // This assumes solution 2. above:
    panic!("No branch in range for {:#x}-{:#x}", offset, dest_offset);
}
