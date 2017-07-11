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

use binemit::{CodeOffset, relax_branches};
use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use loop_analysis::LoopAnalysis;
use isa::TargetIsa;
use legalize_function;
use regalloc;
use result::{CtonError, CtonResult};
use verifier;
use simple_gvn::do_simple_gvn;
use licm::do_licm;

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

    /// Loop analysis of `func`.
    pub loop_analysis: LoopAnalysis,
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
            loop_analysis: LoopAnalysis::new(),
        }
    }

    /// Compile the function.
    ///
    /// Run the function through all the passes necessary to generate code for the target ISA
    /// represented by `isa`. This does not include the final step of emitting machine code into a
    /// code sink.
    ///
    /// Returns the size of the function's code.
    pub fn compile(&mut self, isa: &TargetIsa) -> Result<CodeOffset, CtonError> {
        self.flowgraph();
        self.verify_if(isa)?;

        self.legalize(isa)?;
        self.regalloc(isa)?;
        self.relax_branches(isa)
    }

    /// Run the verifier on the function.
    ///
    /// Also check that the dominator tree and control flow graph are consistent with the function.
    ///
    /// The `isa` argument is currently unused, but the verifier will soon be able to also
    /// check ISA-dependent constraints.
    pub fn verify(&self, isa: Option<&TargetIsa>) -> verifier::Result {
        verifier::verify_context(&self.func, &self.cfg, &self.domtree, isa)
    }

    /// Run the verifier only if the `enable_verifier` setting is true.
    pub fn verify_if(&self, isa: &TargetIsa) -> CtonResult {
        if isa.flags().enable_verifier() {
            self.verify(Some(isa)).map_err(Into::into)
        } else {
            Ok(())
        }
    }

    /// Run the legalizer for `isa` on the function.
    pub fn legalize(&mut self, isa: &TargetIsa) -> CtonResult {
        legalize_function(&mut self.func, &mut self.cfg, &self.domtree, isa);
        self.verify_if(isa)
    }

    /// Recompute the control flow graph and dominator tree.
    pub fn flowgraph(&mut self) {
        self.cfg.compute(&self.func);
        self.domtree.compute(&self.func, &self.cfg);
    }

    /// Perform simple GVN on the function.
    pub fn simple_gvn(&mut self) -> CtonResult {
        do_simple_gvn(&mut self.func, &mut self.cfg);
        // TODO: Factor things such that we can get a Flags and test
        // enable_verifier().
        self.verify(None).map_err(Into::into)
    }

    /// Perform LICM on the function.
    pub fn licm(&mut self) -> CtonResult {
        do_licm(&mut self.func,
                &mut self.cfg,
                &mut self.domtree,
                &mut self.loop_analysis);
        self.verify(None).map_err(Into::into)
    }

    /// Run the register allocator.
    pub fn regalloc(&mut self, isa: &TargetIsa) -> CtonResult {
        self.regalloc
            .run(isa, &mut self.func, &self.cfg, &self.domtree)
    }

    /// Run the branch relaxation pass and return the final code size.
    pub fn relax_branches(&mut self, isa: &TargetIsa) -> Result<CodeOffset, CtonError> {
        let code_size = relax_branches(&mut self.func, isa)?;
        self.verify_if(isa)?;

        Ok(code_size)
    }
}
