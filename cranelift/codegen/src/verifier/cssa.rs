//! Verify conventional SSA form.

use crate::dbg::DisplayList;
use crate::dominator_tree::{DominatorTree, DominatorTreePreorder};
use crate::flowgraph::{BasicBlock, ControlFlowGraph};
use crate::ir::{ExpandedProgramPoint, Function};
use crate::regalloc::liveness::Liveness;
use crate::regalloc::virtregs::VirtRegs;
use crate::timing;
use crate::verifier::{VerifierErrors, VerifierStepResult};

/// Verify conventional SSA form for `func`.
///
/// Conventional SSA form is represented in Cranelift with the help of virtual registers:
///
/// - Two values are said to be *PHI-related* if one is an EBB argument and the other is passed as
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
    liveness: &Liveness,
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
        liveness,
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
    liveness: &'a Liveness,
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
                if self.liveness.get(val).is_none() {
                    return errors.fatal((val, format!("Value in {} has no live range", vreg)));
                };

                // Check topological ordering with the previous values in the virtual register.
                let def: ExpandedProgramPoint = self.func.dfg.value_def(val).into();
                let def_ebb = self.func.layout.pp_ebb(def);
                for &prev_val in &values[0..idx] {
                    let prev_def: ExpandedProgramPoint = self.func.dfg.value_def(prev_val).into();
                    let prev_ebb = self.func.layout.pp_ebb(prev_def);

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
                    if self.preorder.dominates(def_ebb, prev_ebb)
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
                    let prev_ebb = self.func.layout.pp_ebb(prev_def);

                    if self.preorder.dominates(prev_ebb, def_ebb)
                        && self.domtree.dominates(prev_def, def, &self.func.layout)
                    {
                        if self.liveness[prev_val].overlaps_def(def, def_ebb, &self.func.layout) {
                            return errors.fatal((
                                val,
                                format!(
                                    "Value def in {} = {} interferes with {}",
                                    vreg,
                                    DisplayList(values),
                                    prev_val
                                ),
                            ));
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_cssa(&self, errors: &mut VerifierErrors) -> VerifierStepResult<()> {
        for ebb in self.func.layout.ebbs() {
            let ebb_params = self.func.dfg.ebb_params(ebb);
            for BasicBlock { inst: pred, .. } in self.cfg.pred_iter(ebb) {
                let pred_args = self.func.dfg.inst_variable_args(pred);
                // This should have been caught by an earlier verifier pass.
                assert_eq!(
                    ebb_params.len(),
                    pred_args.len(),
                    "Wrong arguments on branch."
                );

                for (&ebb_param, &pred_arg) in ebb_params.iter().zip(pred_args) {
                    if !self.virtregs.same_class(ebb_param, pred_arg) {
                        return errors.fatal((
                            pred,
                            format!(
                                "{} and {} must be in the same virtual register",
                                ebb_param, pred_arg
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
