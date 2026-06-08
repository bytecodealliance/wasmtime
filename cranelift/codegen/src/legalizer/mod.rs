//! Legalize instructions.
//!
//! A legal instruction is one that can be mapped directly to a machine code instruction for the
//! target ISA. The `legalize_function()` function takes as input any function and transforms it
//! into an equivalent function using only legal instructions.
//!
//! The characteristics of legal instructions depend on the target ISA, so any given instruction
//! can be legal for one ISA and illegal for another.
//!
//! Besides transforming instructions, the legalizer also fills out the `function.encodings` map
//! which provides a legal encoding recipe for every instruction.
//!
//! The legalizer does not deal with register allocation constraints. These constraints are derived
//! from the encoding recipes, and solved later by the register allocator.

use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{self, InstBuilder, InstructionData};
use crate::isa::TargetIsa;
use crate::trace;
use cranelift_entity::EntitySet;
use smallvec::SmallVec;

mod branch_to_trap;
mod globalvalue;

use self::branch_to_trap::BranchToTrap;
use self::globalvalue::expand_global_value;

/// A command describing how the walk over instructions should proceed.
enum WalkCommand {
    /// Continue walking to the next instruction, if any.
    Continue,
    /// Revisit the current instruction (presumably because it was legalized
    /// into a new instruction that may also require further legalization).
    Revisit,
}

/// A simple, naive backwards walk over every instruction in every block in the
/// function's layout.
///
/// This does not guarantee any kind of reverse post-order visitation or
/// anything like that, it is just iterating over blocks in reverse layout
/// order, not any kind of control-flow graph visitation order.
///
/// The `f` visitor closure controls how the walk proceeds via its `WalkCommand`
/// result.
fn backward_walk(
    func: &mut ir::Function,
    mut f: impl FnMut(&mut ir::Function, ir::Block, ir::Inst) -> WalkCommand,
) {
    let mut pos = FuncCursor::new(func);
    while let Some(block) = pos.prev_block() {
        let mut prev_pos;
        while let Some(inst) = {
            prev_pos = pos.position();
            pos.prev_inst()
        } {
            match f(pos.func, block, inst) {
                WalkCommand::Continue => continue,
                WalkCommand::Revisit => pos.set_position(prev_pos),
            }
        }
    }
}

/// Perform a simple legalization by expansion of the function, without
/// platform-specific transforms.
pub fn simple_legalize(func: &mut ir::Function, isa: &dyn TargetIsa) {
    trace!("Pre-legalization function:\n{}", func.display());

    let mut branch_to_trap = BranchToTrap::default();

    // We walk the IR backwards because in practice, given the way that
    // frontends tend to produce CLIF, this means we will visit in roughly
    // reverse post order, which is helpful for getting the most optimizations
    // out of the `branch-to-trap` pass that we can (it must analyze trapping
    // blocks before it can rewrite branches to them) but the order does not
    // actually affect correctness.
    backward_walk(func, |func, block, inst| match func.dfg.insts[inst] {
        InstructionData::Trap {
            opcode: ir::Opcode::Trap,
            code: _,
        } => {
            branch_to_trap.analyze_trapping_block(func, block);
            WalkCommand::Continue
        }
        InstructionData::Brif {
            opcode: ir::Opcode::Brif,
            arg,
            blocks,
        } => {
            branch_to_trap.process_brif(func, inst, arg, blocks);
            WalkCommand::Continue
        }

        InstructionData::UnaryGlobalValue {
            opcode: ir::Opcode::GlobalValue,
            global_value,
        } => expand_global_value(inst, func, isa, global_value),

        _ => WalkCommand::Continue,
    });

    trace!("Post-legalization function:\n{}", func.display());
}
