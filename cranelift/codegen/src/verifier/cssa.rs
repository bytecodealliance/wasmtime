//! Verify conventional SSA form.

use crate::dbg::DisplayList;
use crate::dominator_tree::{DominatorTree, DominatorTreePreorder};
use crate::flowgraph::{BlockPredecessor, ControlFlowGraph};
use crate::ir::{ExpandedProgramPoint, Function};
use crate::timing;
use crate::verifier::{virtregs::VirtRegs, VerifierErrors, VerifierStepResult};

/// Verify conventional SSA form for `func`.
///
/// Conventional SSA form is represented in Cranelift with the help of virtual registers:
///
/// - Two values are said to be *PHI-related* if one is a block argument and the other is passed as
///   a branch argument in a location that matches the first value.
/// - PHI-related values must belong to the same virtual register.
/// - Two values in the same virtual register must not have overlapping live ranges.
///
/// Additionally, we verify this property of virtual registers:
///
/// - The values in a virtual register are topologically ordered w.r.t. dominance.
///
/// We don't verify that virtual registers are minimal. Minimal CSSA is not required.
pub fn verify_cssa(
    func: &Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    virtregs: &VirtRegs,
    errors: &mut VerifierErrors,
) -> VerifierStepResult<()> {
    let _tt = timing::verify_cssa();

    let mut preorder = DominatorTreePreorder::new();
    preorder.compute(domtree, &func.layout);

    let verifier = CssaVerifier {
        func,
        cfg,
        domtree,
        virtregs,
        preorder,
    };
    verifier.check_virtregs(errors)?;
    verifier.check_cssa(errors)?;
    Ok(())
}

struct CssaVerifier<'a> {
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
    domtree: &'a DominatorTree,
    virtregs: &'a VirtRegs,
    preorder: DominatorTreePreorder,
}

impl<'a> CssaVerifier<'a> {
    fn check_virtregs(&self, errors: &mut VerifierErrors) -> VerifierStepResult<()> {
        for vreg in self.virtregs.all_virtregs() {
            let values = self.virtregs.values(vreg);

            for (idx, &val) in values.iter().enumerate() {
                if !self.func.dfg.value_is_valid(val) {
                    return errors.fatal((val, format!("Invalid value in {}", vreg)));
                }
                if !self.func.dfg.value_is_attached(val) {
                    return errors.fatal((val, format!("Detached value in {}", vreg)));
                }

                // Check topological ordering with the previous values in the virtual register.
                let def: ExpandedProgramPoint = self.func.dfg.value_def(val).into();
                let def_block = self.func.layout.pp_block(def);
                for &prev_val in &values[0..idx] {
                    let prev_def: ExpandedProgramPoint = self.func.dfg.value_def(prev_val).into();
                    let prev_block = self.func.layout.pp_block(prev_def);

                    if prev_def == def {
                        return errors.fatal((
                            val,
                            format!(
                                "Values {} and {} in {} = {} defined at the same program point",
                                prev_val,
                                val,
                                vreg,
                                DisplayList(values)
                            ),
                        ));
                    }

                    // Enforce topological ordering of defs in the virtual register.
                    if self.preorder.dominates(def_block, prev_block)
                        && self.domtree.dominates(def, prev_def, &self.func.layout)
                    {
                        return errors.fatal((
                            val,
                            format!(
                                "Value in {} = {} def dominates previous {}",
                                vreg,
                                DisplayList(values),
                                prev_val
                            ),
                        ));
                    }
                }

                // Knowing that values are in topo order, we can check for interference this
                // way.
                // We only have to check against the nearest dominating value.
                for &prev_val in values[0..idx].iter().rev() {
                    let prev_def: ExpandedProgramPoint = self.func.dfg.value_def(prev_val).into();
                    let prev_block = self.func.layout.pp_block(prev_def);

                    if self.preorder.dominates(prev_block, def_block)
                        && self.domtree.dominates(prev_def, def, &self.func.layout)
                    {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn check_cssa(&self, errors: &mut VerifierErrors) -> VerifierStepResult<()> {
        for block in self.func.layout.blocks() {
            let block_params = self.func.dfg.block_params(block);
            for BlockPredecessor { inst: pred, .. } in self.cfg.pred_iter(block) {
                let pred_args = self.func.dfg.inst_variable_args(pred);
                // This should have been caught by an earlier verifier pass.
                assert_eq!(
                    block_params.len(),
                    pred_args.len(),
                    "Wrong arguments on branch."
                );

                for (&block_param, &pred_arg) in block_params.iter().zip(pred_args) {
                    if !self.virtregs.same_class(block_param, pred_arg) {
                        return errors.fatal((
                            pred,
                            format!(
                                "{} and {} must be in the same virtual register",
                                block_param, pred_arg
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
