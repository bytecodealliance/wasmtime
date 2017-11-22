//! Verify conventional SSA form.

use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use regalloc::liveness::Liveness;
use regalloc::virtregs::VirtRegs;
use std::cmp::Ordering;
use verifier::Result;

/// Verify conventional SSA form for `func`.
///
/// Conventional SSA form is represented in Cretonne with the help of virtual registers:
///
/// - Two values are said to be *PHI-related* if one is an EBB argument and the other is passed as
///   a branch argument in a location that matches the first value.
/// - PHI-related values must belong to the same virtual register.
/// - Two values in the same virtual register must not have overlapping live ranges.
///
/// Additionally, we verify this property of virtual registers:
///
/// - The values in a virtual register are ordered according to the dominator tree's `rpo_cmp()`.
///
/// We don't verify that virtual registers are minimal. Minimal CSSA is not required.
pub fn verify_cssa(
    func: &Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    liveness: &Liveness,
    virtregs: &VirtRegs,
) -> Result {
    let verifier = CssaVerifier {
        func,
        cfg,
        domtree,
        virtregs,
        liveness,
    };
    verifier.check_virtregs()?;
    verifier.check_cssa()?;
    Ok(())
}

struct CssaVerifier<'a> {
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
    domtree: &'a DominatorTree,
    virtregs: &'a VirtRegs,
    liveness: &'a Liveness,
}

impl<'a> CssaVerifier<'a> {
    fn check_virtregs(&self) -> Result {
        for vreg in self.virtregs.all_virtregs() {
            let values = self.virtregs.values(vreg);

            for (idx, &val) in values.iter().enumerate() {
                if !self.func.dfg.value_is_valid(val) {
                    return err!(val, "Invalid value in {}", vreg);
                }
                if !self.func.dfg.value_is_attached(val) {
                    return err!(val, "Detached value in {}", vreg);
                }
                if self.liveness.get(val).is_none() {
                    return err!(val, "Value in {} has no live range", vreg);
                };

                // Check RPO ordering with the previous values in the virtual register.
                let def = self.func.dfg.value_def(val).into();
                let def_ebb = self.func.layout.pp_ebb(def);
                for &prev_val in &values[0..idx] {
                    let prev_def = self.func.dfg.value_def(prev_val);

                    // Enforce RPO of defs in the virtual register.
                    match self.domtree.rpo_cmp(prev_def, def, &self.func.layout) {
                        Ordering::Less => {}
                        Ordering::Equal => {
                            return err!(val, "Value in {} has same def as {}", vreg, prev_val);
                        }
                        Ordering::Greater => {
                            return err!(
                                val,
                                "Value in {} in wrong order relative to {}",
                                vreg,
                                prev_val
                            );
                        }
                    }

                    // Knowing that values are in RPO order, we can check for interference this
                    // way.
                    if self.liveness[prev_val].overlaps_def(def, def_ebb, &self.func.layout) {
                        return err!(val, "Value def in {} interferes with {}", vreg, prev_val);
                    }
                }
            }
        }

        Ok(())
    }

    fn check_cssa(&self) -> Result {
        for ebb in self.func.layout.ebbs() {
            let ebb_params = self.func.dfg.ebb_params(ebb);
            for (_, pred) in self.cfg.pred_iter(ebb) {
                let pred_args = self.func.dfg.inst_variable_args(pred);
                // This should have been caught by an earlier verifier pass.
                assert_eq!(
                    ebb_params.len(),
                    pred_args.len(),
                    "Wrong arguments on branch."
                );

                for (&ebb_param, &pred_arg) in ebb_params.iter().zip(pred_args) {
                    if !self.virtregs.same_class(ebb_param, pred_arg) {
                        return err!(
                            pred,
                            "{} and {} must be in the same virtual register",
                            ebb_param,
                            pred_arg
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
