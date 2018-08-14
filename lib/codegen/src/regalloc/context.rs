//! Register allocator context.
//!
//! The `Context` struct contains data structures that should be preserved across invocations of
//! the register allocator algorithm. This doesn't preserve any data between functions, but it
//! avoids allocating data structures independently for each function begin compiled.

use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use isa::TargetIsa;
use regalloc::coalescing::Coalescing;
use regalloc::coloring::Coloring;
use regalloc::live_value_tracker::LiveValueTracker;
use regalloc::liveness::Liveness;
use regalloc::reload::Reload;
use regalloc::spilling::Spilling;
use regalloc::virtregs::VirtRegs;
use result::CodegenResult;
use timing;
use topo_order::TopoOrder;
use verifier::{verify_context, verify_cssa, verify_liveness, verify_locations, VerifierErrors};

/// Persistent memory allocations for register allocation.
pub struct Context {
    liveness: Liveness,
    virtregs: VirtRegs,
    coalescing: Coalescing,
    topo: TopoOrder,
    tracker: LiveValueTracker,
    spilling: Spilling,
    reload: Reload,
    coloring: Coloring,
}

impl Context {
    /// Create a new context for register allocation.
    ///
    /// This context should be reused for multiple functions in order to avoid repeated memory
    /// allocations.
    pub fn new() -> Self {
        Self {
            liveness: Liveness::new(),
            virtregs: VirtRegs::new(),
            coalescing: Coalescing::new(),
            topo: TopoOrder::new(),
            tracker: LiveValueTracker::new(),
            spilling: Spilling::new(),
            reload: Reload::new(),
            coloring: Coloring::new(),
        }
    }

    /// Clear all data structures in this context.
    pub fn clear(&mut self) {
        self.liveness.clear();
        self.virtregs.clear();
        self.coalescing.clear();
        self.topo.clear();
        self.tracker.clear();
        self.spilling.clear();
        self.reload.clear();
        self.coloring.clear();
    }

    /// Allocate registers in `func`.
    ///
    /// After register allocation, all values in `func` have been assigned to a register or stack
    /// location that is consistent with instruction encoding constraints.
    pub fn run(
        &mut self,
        isa: &TargetIsa,
        func: &mut Function,
        cfg: &ControlFlowGraph,
        domtree: &mut DominatorTree,
    ) -> CodegenResult<()> {
        let _tt = timing::regalloc();
        debug_assert!(domtree.is_valid());

        let mut errors = VerifierErrors::default();

        // `Liveness` and `Coloring` are self-clearing.
        self.virtregs.clear();

        // Tracker state (dominator live sets) is actually reused between the spilling and coloring
        // phases.
        self.tracker.clear();

        // Pass: Liveness analysis.
        self.liveness.compute(isa, func, cfg);

        if isa.flags().enable_verifier() {
            let ok = verify_liveness(isa, func, cfg, &self.liveness, &mut errors).is_ok();

            if !ok {
                return Err(errors.into());
            }
        }

        // Pass: Coalesce and create Conventional SSA form.
        self.coalescing.conventional_ssa(
            isa,
            func,
            cfg,
            domtree,
            &mut self.liveness,
            &mut self.virtregs,
        );

        if isa.flags().enable_verifier() {
            let ok = verify_context(func, cfg, domtree, isa, &mut errors).is_ok()
                && verify_liveness(isa, func, cfg, &self.liveness, &mut errors).is_ok()
                && verify_cssa(
                    func,
                    cfg,
                    domtree,
                    &self.liveness,
                    &self.virtregs,
                    &mut errors,
                ).is_ok();

            if !ok {
                return Err(errors.into());
            }
        }

        // Pass: Spilling.
        self.spilling.run(
            isa,
            func,
            domtree,
            &mut self.liveness,
            &self.virtregs,
            &mut self.topo,
            &mut self.tracker,
        );

        if isa.flags().enable_verifier() {
            let ok = verify_context(func, cfg, domtree, isa, &mut errors).is_ok()
                && verify_liveness(isa, func, cfg, &self.liveness, &mut errors).is_ok()
                && verify_cssa(
                    func,
                    cfg,
                    domtree,
                    &self.liveness,
                    &self.virtregs,
                    &mut errors,
                ).is_ok();

            if !ok {
                return Err(errors.into());
            }
        }

        // Pass: Reload.
        self.reload.run(
            isa,
            func,
            domtree,
            &mut self.liveness,
            &mut self.topo,
            &mut self.tracker,
        );

        if isa.flags().enable_verifier() {
            let ok = verify_context(func, cfg, domtree, isa, &mut errors).is_ok()
                && verify_liveness(isa, func, cfg, &self.liveness, &mut errors).is_ok()
                && verify_cssa(
                    func,
                    cfg,
                    domtree,
                    &self.liveness,
                    &self.virtregs,
                    &mut errors,
                ).is_ok();

            if !ok {
                return Err(errors.into());
            }
        }

        // Pass: Coloring.
        self.coloring
            .run(isa, func, domtree, &mut self.liveness, &mut self.tracker);

        if isa.flags().enable_verifier() {
            let ok = verify_context(func, cfg, domtree, isa, &mut errors).is_ok()
                && verify_liveness(isa, func, cfg, &self.liveness, &mut errors).is_ok()
                && verify_locations(isa, func, Some(&self.liveness), &mut errors).is_ok()
                && verify_cssa(
                    func,
                    cfg,
                    domtree,
                    &self.liveness,
                    &self.virtregs,
                    &mut errors,
                ).is_ok();

            if !ok {
                return Err(errors.into());
            }
        }

        // Even if we arrive here, (non-fatal) errors might have been reported, so we
        // must make sure absolutely nothing is wrong
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }
}
