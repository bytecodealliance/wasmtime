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
//! For example, Intel branches can have either an 8-bit or a 32-bit displacement.
//!
//! On RISC architectures, it can happen that conditional branches have a shorter range than
//! unconditional branches:
//!
//! ```cton
//!     brz v1, ebb17
//! ```
//!
//! can be transformed into:
//!
//! ```cton
//!     brnz v1, ebb23
//!     jump ebb17
//! ebb23:
//! ```

use binemit::CodeOffset;
use cursor::{Cursor, FuncCursor};
use ir::{Function, InstructionData, Opcode};
use isa::{TargetIsa, EncInfo};
use iterators::IteratorExtras;
use result::CtonError;

/// Relax branches and compute the final layout of EBB headers in `func`.
///
/// Fill in the `func.offsets` table so the function is ready for binary emission.
pub fn relax_branches(func: &mut Function, isa: &TargetIsa) -> Result<CodeOffset, CtonError> {
    let encinfo = isa.encoding_info();

    // Clear all offsets so we can recognize EBBs that haven't been visited yet.
    func.offsets.clear();
    func.offsets.resize(func.dfg.num_ebbs());

    // Start by inserting fall through instructions.
    fallthroughs(func);

    let mut offset = 0;

    // The relaxation algorithm iterates to convergence.
    let mut go_again = true;
    while go_again {
        go_again = false;
        offset = 0;

        // Visit all instructions in layout order
        let mut cur = FuncCursor::new(func);
        while let Some(ebb) = cur.next_ebb() {
            // Record the offset for `ebb` and make sure we iterate until offsets are stable.
            if cur.func.offsets[ebb] != offset {
                assert!(
                    cur.func.offsets[ebb] < offset,
                    "Code shrinking during relaxation"
                );
                cur.func.offsets[ebb] = offset;
                go_again = true;
            }

            while let Some(inst) = cur.next_inst() {
                let enc = cur.func.encodings[inst];
                let size = encinfo.bytes(enc);

                // See if this might be a branch that is out of range.
                if let Some(range) = encinfo.branch_range(enc) {
                    if let Some(dest) = cur.func.dfg[inst].branch_destination() {
                        let dest_offset = cur.func.offsets[dest];
                        if !range.contains(offset, dest_offset) {
                            // This is an out-of-range branch.
                            // Relax it unless the destination offset has not been computed yet.
                            if dest_offset != 0 || Some(dest) == cur.func.layout.entry_block() {
                                offset +=
                                    relax_branch(&mut cur, offset, dest_offset, &encinfo, isa);
                                continue;
                            }
                        }
                    }
                }

                offset += size;
            }
        }
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
                    assert_eq!(destination, succ, "Illegal fall-through in {}", ebb)
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

/// Relax the branch instruction at `pos` so it can cover the range `offset - dest_offset`.
///
/// Return the size of the replacement instructions up to and including the location where `pos` is
/// left.
fn relax_branch(
    cur: &mut FuncCursor,
    offset: CodeOffset,
    dest_offset: CodeOffset,
    encinfo: &EncInfo,
    isa: &TargetIsa,
) -> CodeOffset {
    let inst = cur.current_inst().unwrap();
    dbg!(
        "Relaxing [{}] {} for {:#x}-{:#x} range",
        encinfo.display(cur.func.encodings[inst]),
        cur.func.dfg.display_inst(inst, isa),
        offset,
        dest_offset
    );

    // Pick the first encoding that can handle the branch range.
    let dfg = &cur.func.dfg;
    let ctrl_type = dfg.ctrl_typevar(inst);
    if let Some(enc) = isa.legal_encodings(dfg, &dfg[inst], ctrl_type).find(
        |&enc| {
            let range = encinfo.branch_range(enc).expect("Branch with no range");
            let in_range = range.contains(offset, dest_offset);
            dbg!(
                "  trying [{}]: {}",
                encinfo.display(enc),
                if in_range { "OK" } else { "out of range" }
            );
            in_range
        },
    )
    {
        cur.func.encodings[inst] = enc;
        return encinfo.bytes(enc);
    }

    unimplemented!();
}
