//! Cretonne compilation context and main entry point.
//!
//! When compiling many small functions, it is important to avoid repeatedly allocating and
//! deallocating the data structures needed for compilation. The `Context` struct is used to hold
//! on to memory allocations between function compilations.

use cfg::ControlFlowGraph;
use dominator_tree::DominatorTree;
use ir::Function;

/// Persistent data structures and compilation pipeline.
pub struct Context {
    /// The function we're compiling.
    pub func: Function,

    /// The control flow graph of `func`.
    pub cfg: ControlFlowGraph,

    /// Dominator tree for `func`.
    pub domtree: DominatorTree,
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
        }
    }

    /// Recompute the control flow graph and dominator tree.
    pub fn flowgraph(&mut self) {
        self.cfg.compute(&self.func);
        self.domtree.compute(&self.func, &self.cfg);
    }
}
