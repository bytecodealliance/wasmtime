//! Cretonne compilation context and main entry point.
//!
//! When compiling many small functions, it is important to avoid repeatedly allocating and
//! deallocating the data structures needed for compilation. The `Context` struct is used to hold
//! on to memory allocations between function compilations.
//!
//! The context does not hold a `TargetIsa` instance which has to be provided as an argument
//! instead. This is because an ISA instance is immutable and can be used by multiple compilation
//! contexts concurrently. Typically, you would have one context per compilation thread and only a
//! single ISA instance.

use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use isa::TargetIsa;
use legalize_function;
use regalloc;

/// Persistent data structures and compilation pipeline.
pub struct Context {
    /// The function we're compiling.
    pub func: Function,

    /// The control flow graph of `func`.
    pub cfg: ControlFlowGraph,

    /// Dominator tree for `func`.
    pub domtree: DominatorTree,

    /// Register allocation context.
    pub regalloc: regalloc::Context,
}

impl Context {
    /// Allocate a new compilation context.
    ///
    /// The returned instance should be reused for compiling multiple functions in order to avoid
    /// needless allocator thrashing.
    pub fn new() -> Context {
        Context {
            func: Function::new(),
            cfg: ControlFlowGraph::new(),
            domtree: DominatorTree::new(),
            regalloc: regalloc::Context::new(),
        }
    }

    /// Run the legalizer for `isa` on the function.
    pub fn legalize(&mut self, isa: &TargetIsa) {
        legalize_function(&mut self.func, isa);
    }

    /// Recompute the control flow graph and dominator tree.
    pub fn flowgraph(&mut self) {
        self.cfg.compute(&self.func);
        self.domtree.compute(&self.func, &self.cfg);
    }

    /// Run the register allocator.
    pub fn regalloc(&mut self, isa: &TargetIsa) {
        self.regalloc.run(isa, &mut self.func, &self.cfg, &self.domtree);
    }
}
