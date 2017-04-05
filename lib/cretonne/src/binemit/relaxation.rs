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
use entity_map::EntityMap;
use ir::{Function, DataFlowGraph, Cursor, Inst};
use isa::{TargetIsa, EncInfo, Encoding};

/// Relax branches and compute the final layout of EBB headers in `func`.
///
/// Fill in the `func.offsets` table so the function is ready for binary emission.
pub fn relax_branches(func: &mut Function, isa: &TargetIsa) {
    let encinfo = isa.encoding_info();

    // Clear all offsets so we can recognize EBBs that haven't been visited yet.
    func.offsets.clear();
    func.offsets.resize(func.dfg.num_ebbs());

    // The relaxation algorithm iterates to convergence.
    let mut go_again = true;
    while go_again {
        go_again = false;

        // Visit all instructions in layout order
        let mut offset = 0;
        let mut pos = Cursor::new(&mut func.layout);
        while let Some(ebb) = pos.next_ebb() {
            // Record the offset for `ebb` and make sure we iterate until offsets are stable.
            if func.offsets[ebb] != offset {
                assert!(func.offsets[ebb] < offset,
                        "Code shrinking during relaxation");
                func.offsets[ebb] = offset;
                go_again = true;
            }

            while let Some(inst) = pos.next_inst() {
                let enc = func.encodings.get(inst).cloned().unwrap_or_default();
                let size = encinfo.bytes(enc);

                // See if this might be a branch that is out of range.
                if let Some(range) = encinfo.branch_range(enc) {
                    if let Some(dest) = func.dfg[inst].branch_destination() {
                        let dest_offset = func.offsets[dest];
                        if !range.contains(offset, dest_offset) {
                            // This is an out-of-range branch.
                            // Relax it unless the destination offset has not been computed yet.
                            if dest_offset != 0 || Some(dest) == pos.layout.entry_block() {
                                offset += relax_branch(&mut func.dfg,
                                                       &mut func.encodings,
                                                       &encinfo,
                                                       &mut pos,
                                                       offset,
                                                       dest_offset);
                                continue;
                            }
                        }
                    }
                }

                offset += size;
            }
        }
    }
}

/// Relax the branch instruction at `pos` so it can cover the range `offset - dest_offset`.
///
/// Return the size of the replacement instructions up to and including the location where `pos` is
/// left.
fn relax_branch(dfg: &mut DataFlowGraph,
                encodings: &mut EntityMap<Inst, Encoding>,
                encinfo: &EncInfo,
                pos: &mut Cursor,
                offset: CodeOffset,
                dest_offset: CodeOffset)
                -> CodeOffset {
    let inst = pos.current_inst().unwrap();
    dbg!("Relaxing [{}] {} for {:#x}-{:#x} range",
         encinfo.display(encodings[inst]),
         dfg.display_inst(inst),
         offset,
         dest_offset);
    unimplemented!();
}
