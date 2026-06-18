//! Analysis for rewriting branch-to-unconditional-trap into conditional trap
//! instructions.
//!
//! Given this instruction:
//!
//! ```clif
//! brif v0, block1, block2
//! ```
//!
//! If we know that `block1` does nothing but immediately trap then we can
//! rewrite that `brif` into the following:
//!
//! ```clif
//! trapnz v0, <trapcode>
//! jump block2
//! ```
//!
//! (And we can do the equivalent with `trapz` if `block2` immediately traps).
//!
//! This transformation allows for the conditional trap instructions to be GVN'd
//! and for our egraphs mid-end to generally better optimize the program. We
//! additionally have better codegen in our backends for `trapz`/`trapnz` than
//! branches to unconditional traps.
//!
//! This module only provides the *analysis* of which blocks are "just trap"
//! blocks; the actual rewrite is performed by `simplify_skeleton` ISLE rules in
//! the egraph pass, which consult this analysis via the `just_trap_block` ISLE
//! constructor.

use crate::FxHashMap;
use crate::inst_predicates::is_pure_for_egraph;
use crate::ir::{self, InstructionData, Opcode};
use cranelift_entity::EntitySet;

/// On-demand, memoized analysis of which blocks are "just trap" blocks.
///
/// A block is a "just trap" block when its terminator is an unconditional
/// `trap` and all of its other instructions in the block are pure.
///
/// Results are memoized so that it does not matter in which order blocks are
/// analyzed or instructions are processed; callers can ask for the analysis
/// result of any block at any time.
#[derive(Default)]
pub struct BranchToTrapAnalysis {
    /// The set of blocks we have already analyzed.
    analyzed_blocks: EntitySet<ir::Block>,

    /// Given that we have already analyzed a block and found it to be a
    /// just-trap block, what is its trap code?
    just_trap_block_codes: FxHashMap<ir::Block, ir::TrapCode>,
}

impl BranchToTrapAnalysis {
    /// Determine whether `block` is a "just trap" block and, if so, return the
    /// trap code of its terminating `trap` instruction.
    pub fn analyze_block(&mut self, func: &ir::Function, block: ir::Block) -> Option<ir::TrapCode> {
        if self.analyzed_blocks.insert(block) {
            let code = Self::analyze_block_impl(func, block);
            if let Some(code) = code {
                let old_entry = self.just_trap_block_codes.insert(block, code);
                debug_assert!(old_entry.is_none());
            }
            code
        } else {
            self.just_trap_block_codes.get(&block).copied()
        }
    }

    fn analyze_block_impl(func: &ir::Function, block: ir::Block) -> Option<ir::TrapCode> {
        let last = func.layout.last_inst(block)?;
        let code = match func.dfg.insts[last] {
            InstructionData::Trap {
                opcode: Opcode::Trap,
                code,
            } => code,
            _ => return None,
        };

        for inst in func.layout.block_insts(block) {
            if inst != last && !is_pure_for_egraph(func, inst) {
                return None;
            }
        }

        Some(code)
    }
}
