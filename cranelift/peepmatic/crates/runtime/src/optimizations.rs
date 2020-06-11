//! Compiled peephole optimizations.

use crate::error::Result;
use crate::instruction_set::InstructionSet;
use crate::integer_interner::IntegerInterner;
use crate::linear::{Action, MatchOp, MatchResult};
use crate::optimizer::PeepholeOptimizer;
use crate::paths::PathInterner;
use peepmatic_automata::Automaton;
use serde::{Deserialize, Serialize};

#[cfg(feature = "construct")]
use std::fs;
#[cfg(feature = "construct")]
use std::path::Path;

/// A compiled set of peephole optimizations.
///
/// This is the compilation result of the `peepmatic` crate, after its taken a
/// bunch of optimizations written in the DSL and lowered and combined them.
#[derive(Debug, Serialize, Deserialize)]
pub struct PeepholeOptimizations {
    /// The instruction paths referenced by the peephole optimizations.
    pub paths: PathInterner,

    /// Not all integers we're matching on fit in the `u32` that we use as the
    /// result of match operations. So we intern them and refer to them by id.
    pub integers: IntegerInterner,

    /// The underlying automata for matching optimizations' left-hand sides, and
    /// building up the corresponding right-hand side.
    pub automata: Automaton<MatchResult, MatchOp, Box<[Action]>>,
}

impl PeepholeOptimizations {
    /// Deserialize a `PeepholeOptimizations` from bytes.
    pub fn deserialize(serialized: &[u8]) -> Result<Self> {
        let peep_opt: Self = bincode::deserialize(serialized)?;
        Ok(peep_opt)
    }

    /// Serialize these peephole optimizations out to the file at the given path.
    ///
    /// Requires that the `"construct"` cargo feature is enabled.
    #[cfg(feature = "construct")]
    pub fn serialize_to_file(&self, path: &Path) -> Result<()> {
        let file = fs::File::create(path)?;
        bincode::serialize_into(file, self)?;
        Ok(())
    }

    /// Create a new peephole optimizer instance from this set of peephole
    /// optimizations.
    ///
    /// The peephole optimizer instance can be used to apply these peephole
    /// optimizations. When checking multiple instructions for whether they can
    /// be optimized, it is more performant to reuse a single peephole optimizer
    /// instance, rather than create a new one for each instruction. Reusing the
    /// peephole optimizer instance allows the reuse of a few internal
    /// allocations.
    pub fn optimizer<'peep, 'ctx, I>(&'peep self, instr_set: I) -> PeepholeOptimizer<'peep, 'ctx, I>
    where
        I: InstructionSet<'ctx>,
    {
        PeepholeOptimizer {
            peep_opt: self,
            instr_set,
            right_hand_sides: vec![],
            actions: vec![],
            backtracking_states: vec![],
        }
    }
}
