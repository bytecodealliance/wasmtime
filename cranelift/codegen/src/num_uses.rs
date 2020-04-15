//! A pass that computes the number of uses of any given instruction.

use crate::entity::SecondaryMap;
use crate::ir::dfg::ValueDef;
use crate::ir::Value;
use crate::ir::{DataFlowGraph, Function, Inst};

/// Auxiliary data structure that counts the number of uses of any given
/// instruction in a Function. This is used during instruction selection
/// to essentially do incremental DCE: when an instruction is no longer
/// needed because its computation has been isel'd into another machine
/// instruction at every use site, we can skip it.
#[derive(Clone, Debug)]
pub struct NumUses {
    uses: SecondaryMap<Inst, u32>,
}

impl NumUses {
    fn new() -> NumUses {
        NumUses {
            uses: SecondaryMap::with_default(0),
        }
    }

    /// Compute the NumUses analysis result for a function.
    pub fn compute(func: &Function) -> NumUses {
        let mut uses = NumUses::new();
        for bb in func.layout.blocks() {
            for inst in func.layout.block_insts(bb) {
                for arg in func.dfg.inst_args(inst) {
                    let v = func.dfg.resolve_aliases(*arg);
                    uses.add_value(&func.dfg, v);
                }
            }
        }
        uses
    }

    fn add_value(&mut self, dfg: &DataFlowGraph, v: Value) {
        match dfg.value_def(v) {
            ValueDef::Result(inst, _) => {
                self.uses[inst] += 1;
            }
            _ => {}
        }
    }

    /// Take the complete uses map, consuming this analysis result.
    pub fn take_uses(self) -> SecondaryMap<Inst, u32> {
        self.uses
    }
}
