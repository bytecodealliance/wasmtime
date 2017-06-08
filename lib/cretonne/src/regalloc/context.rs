//! Register allocator context.
//!
//! The `Context` struct contains data structures that should be preserved across invocations of
//! the register allocator algorithm. This doesn't preserve any data between functions, but it
//! avoids allocating data structures independently for each function begin compiled.

use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use isa::TargetIsa;
use regalloc::coloring::Coloring;
use regalloc::live_value_tracker::LiveValueTracker;
use regalloc::liveness::Liveness;
use regalloc::reload::Reload;
use regalloc::spilling::Spilling;
use result::CtonResult;
use topo_order::TopoOrder;
use verifier::{verify_context, verify_liveness};

/// Persistent memory allocations for register allocation.
pub struct Context {
    liveness: Liveness,
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
    pub fn new() -> Context {
        Context {
            liveness: Liveness::new(),
            topo: TopoOrder::new(),
            tracker: LiveValueTracker::new(),
            spilling: Spilling::new(),
            reload: Reload::new(),
            coloring: Coloring::new(),
        }
    }

    /// Allocate registers in `func`.
    ///
    /// After register allocation, all values in `func` have been assigned to a register or stack
    /// location that is consistent with instruction encoding constraints.
    pub fn run(&mut self,
               isa: &TargetIsa,
               func: &mut Function,
               cfg: &ControlFlowGraph,
               domtree: &DominatorTree)
               -> CtonResult {
        // `Liveness` and `Coloring` are self-clearing.
        // Tracker state (dominator live sets) is actually reused between the spilling and coloring
        // phases.
        self.tracker.clear();

        // First pass: Liveness analysis.
        self.liveness.compute(isa, func, cfg);

        if isa.flags().enable_verifier() {
            verify_liveness(isa, func, cfg, &self.liveness)?;
        }

        // Second pass: Spilling.
        self.spilling
            .run(isa,
                 func,
                 domtree,
                 &mut self.liveness,
                 &mut self.topo,
                 &mut self.tracker);

        if isa.flags().enable_verifier() {
            verify_context(func, cfg, domtree, Some(isa))?;
            verify_liveness(isa, func, cfg, &self.liveness)?;
        }

        // Third pass: Reload.
        self.reload
            .run(isa,
                 func,
                 domtree,
                 &mut self.liveness,
                 &mut self.topo,
                 &mut self.tracker);

        if isa.flags().enable_verifier() {
            verify_context(func, cfg, domtree, Some(isa))?;
            verify_liveness(isa, func, cfg, &self.liveness)?;
        }

        // Fourth pass: Coloring.
        self.coloring
            .run(isa,
                 func,
                 domtree,
                 &mut self.liveness,
                 &mut self.topo,
                 &mut self.tracker);

        if isa.flags().enable_verifier() {
            verify_context(func, cfg, domtree, Some(isa))?;
            verify_liveness(isa, func, cfg, &self.liveness)?;
        }
        Ok(())
    }
}
