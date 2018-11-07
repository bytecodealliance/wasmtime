//! Cranelift compilation context and main entry point.
//!
//! When compiling many small functions, it is important to avoid repeatedly allocating and
//! deallocating the data structures needed for compilation. The `Context` struct is used to hold
//! on to memory allocations between function compilations.
//!
//! The context does not hold a `TargetIsa` instance which has to be provided as an argument
//! instead. This is because an ISA instance is immutable and can be used by multiple compilation
//! contexts concurrently. Typically, you would have one context per compilation thread and only a
//! single ISA instance.

use binemit::{
    relax_branches, shrink_instructions, CodeOffset, MemoryCodeSink, RelocSink, TrapSink,
};
use dce::do_dce;
use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::Function;
use isa::TargetIsa;
use legalize_function;
use licm::do_licm;
use loop_analysis::LoopAnalysis;
use nan_canonicalization::do_nan_canonicalization;
use postopt::do_postopt;
use regalloc;
use result::CodegenResult;
use settings::{FlagsOrIsa, OptLevel};
use simple_gvn::do_simple_gvn;
use simple_preopt::do_preopt;
use std::vec::Vec;
use timing;
use unreachable_code::eliminate_unreachable_code;
use verifier::{verify_context, verify_locations, VerifierErrors, VerifierResult};

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
    pub fn new() -> Self {
        Self::for_function(Function::new())
    }

    /// Allocate a new compilation context with an existing Function.
    ///
    /// The returned instance should be reused for compiling multiple functions in order to avoid
    /// needless allocator thrashing.
    pub fn for_function(func: Function) -> Self {
        Self {
            func,
            cfg: ControlFlowGraph::new(),
            domtree: DominatorTree::new(),
            regalloc: regalloc::Context::new(),
            loop_analysis: LoopAnalysis::new(),
        }
    }

    /// Clear all data structures in this context.
    pub fn clear(&mut self) {
        self.func.clear();
        self.cfg.clear();
        self.domtree.clear();
        self.regalloc.clear();
        self.loop_analysis.clear();
    }

    /// Compile the function, and emit machine code into a `Vec<u8>`.
    ///
    /// Run the function through all the passes necessary to generate code for the target ISA
    /// represented by `isa`, as well as the final step of emitting machine code into a
    /// `Vec<u8>`. The machine code is not relocated. Instead, any relocations are emitted
    /// into `relocs`.
    ///
    /// This function calls `compile` and `emit_to_memory`, taking care to resize `mem` as
    /// needed, so it provides a safe interface.
    pub fn compile_and_emit(
        &mut self,
        isa: &TargetIsa,
        mem: &mut Vec<u8>,
        relocs: &mut RelocSink,
        traps: &mut TrapSink,
    ) -> CodegenResult<()> {
        let code_size = self.compile(isa)?;
        let old_len = mem.len();
        mem.resize(old_len + code_size as usize, 0);
        unsafe { self.emit_to_memory(isa, mem.as_mut_ptr().add(old_len), relocs, traps) };
        Ok(())
    }

    /// Compile the function.
    ///
    /// Run the function through all the passes necessary to generate code for the target ISA
    /// represented by `isa`. This does not include the final step of emitting machine code into a
    /// code sink.
    ///
    /// Returns the size of the function's code.
    pub fn compile(&mut self, isa: &TargetIsa) -> CodegenResult<CodeOffset> {
        let _tt = timing::compile();
        self.verify_if(isa)?;

        self.compute_cfg();
        if isa.flags().opt_level() != OptLevel::Fastest {
            self.preopt(isa)?;
        }
        if isa.flags().enable_nan_canonicalization() {
            self.canonicalize_nans(isa)?;
        }
        self.legalize(isa)?;
        if isa.flags().opt_level() != OptLevel::Fastest {
            self.postopt(isa)?;
        }
        if isa.flags().opt_level() == OptLevel::Best {
            self.compute_domtree();
            self.compute_loop_analysis();
            self.licm(isa)?;
            self.simple_gvn(isa)?;
        }
        self.compute_domtree();
        self.eliminate_unreachable_code(isa)?;
        if isa.flags().opt_level() != OptLevel::Fastest {
            self.dce(isa)?;
        }
        self.regalloc(isa)?;
        self.prologue_epilogue(isa)?;
        if isa.flags().opt_level() == OptLevel::Best {
            self.shrink_instructions(isa)?;
        }
        self.relax_branches(isa)
    }

    /// Emit machine code directly into raw memory.
    ///
    /// Write all of the function's machine code to the memory at `mem`. The size of the machine
    /// code is returned by `compile` above.
    ///
    /// The machine code is not relocated. Instead, any relocations are emitted into `relocs`.
    ///
    /// This function is unsafe since it does not perform bounds checking on the memory buffer,
    /// and it can't guarantee that the `mem` pointer is valid.
    pub unsafe fn emit_to_memory(
        &self,
        isa: &TargetIsa,
        mem: *mut u8,
        relocs: &mut RelocSink,
        traps: &mut TrapSink,
    ) {
        let _tt = timing::binemit();
        isa.emit_function_to_memory(&self.func, &mut MemoryCodeSink::new(mem, relocs, traps));
    }

    /// Run the verifier on the function.
    ///
    /// Also check that the dominator tree and control flow graph are consistent with the function.
    pub fn verify<'a, FOI: Into<FlagsOrIsa<'a>>>(&self, fisa: FOI) -> VerifierResult<()> {
        let mut errors = VerifierErrors::default();
        let _ = verify_context(&self.func, &self.cfg, &self.domtree, fisa, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Run the verifier only if the `enable_verifier` setting is true.
    pub fn verify_if<'a, FOI: Into<FlagsOrIsa<'a>>>(&self, fisa: FOI) -> CodegenResult<()> {
        let fisa = fisa.into();
        if fisa.flags.enable_verifier() {
            self.verify(fisa)?;
        }
        Ok(())
    }

    /// Run the locations verifier on the function.
    pub fn verify_locations(&self, isa: &TargetIsa) -> VerifierResult<()> {
        let mut errors = VerifierErrors::default();
        let _ = verify_locations(isa, &self.func, None, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Run the locations verifier only if the `enable_verifier` setting is true.
    pub fn verify_locations_if(&self, isa: &TargetIsa) -> CodegenResult<()> {
        if isa.flags().enable_verifier() {
            self.verify_locations(isa)?;
        }
        Ok(())
    }

    /// Perform dead-code elimination on the function.
    pub fn dce<'a, FOI: Into<FlagsOrIsa<'a>>>(&mut self, fisa: FOI) -> CodegenResult<()> {
        do_dce(&mut self.func, &mut self.domtree);
        self.verify_if(fisa)?;
        Ok(())
    }

    /// Perform pre-legalization rewrites on the function.
    pub fn preopt(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        do_preopt(&mut self.func);
        self.verify_if(isa)?;
        Ok(())
    }

    /// Perform NaN canonicalizing rewrites on the function.
    pub fn canonicalize_nans(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        do_nan_canonicalization(&mut self.func);
        self.verify_if(isa)
    }

    /// Run the legalizer for `isa` on the function.
    pub fn legalize(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        // Legalization invalidates the domtree and loop_analysis by mutating the CFG.
        // TODO: Avoid doing this when legalization doesn't actually mutate the CFG.
        self.domtree.clear();
        self.loop_analysis.clear();
        legalize_function(&mut self.func, &mut self.cfg, isa);
        self.verify_if(isa)
    }

    /// Perform post-legalization rewrites on the function.
    pub fn postopt(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        do_postopt(&mut self.func, isa);
        self.verify_if(isa)?;
        Ok(())
    }

    /// Compute the control flow graph.
    pub fn compute_cfg(&mut self) {
        self.cfg.compute(&self.func)
    }

    /// Compute dominator tree.
    pub fn compute_domtree(&mut self) {
        self.domtree.compute(&self.func, &self.cfg)
    }

    /// Compute the loop analysis.
    pub fn compute_loop_analysis(&mut self) {
        self.loop_analysis
            .compute(&self.func, &self.cfg, &self.domtree)
    }

    /// Compute the control flow graph and dominator tree.
    pub fn flowgraph(&mut self) {
        self.compute_cfg();
        self.compute_domtree()
    }

    /// Perform simple GVN on the function.
    pub fn simple_gvn<'a, FOI: Into<FlagsOrIsa<'a>>>(&mut self, fisa: FOI) -> CodegenResult<()> {
        do_simple_gvn(&mut self.func, &mut self.domtree);
        self.verify_if(fisa)
    }

    /// Perform LICM on the function.
    pub fn licm<'a, FOI: Into<FlagsOrIsa<'a>>>(&mut self, fisa: FOI) -> CodegenResult<()> {
        do_licm(
            &mut self.func,
            &mut self.cfg,
            &mut self.domtree,
            &mut self.loop_analysis,
        );
        self.verify_if(fisa)
    }

    /// Perform unreachable code elimination.
    pub fn eliminate_unreachable_code<'a, FOI>(&mut self, fisa: FOI) -> CodegenResult<()>
    where
        FOI: Into<FlagsOrIsa<'a>>,
    {
        eliminate_unreachable_code(&mut self.func, &mut self.cfg, &self.domtree);
        self.verify_if(fisa)
    }

    /// Run the register allocator.
    pub fn regalloc(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        self.regalloc
            .run(isa, &mut self.func, &self.cfg, &mut self.domtree)
    }

    /// Insert prologue and epilogues after computing the stack frame layout.
    pub fn prologue_epilogue(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        isa.prologue_epilogue(&mut self.func)?;
        self.verify_if(isa)?;
        self.verify_locations_if(isa)?;
        Ok(())
    }

    /// Run the instruction shrinking pass.
    pub fn shrink_instructions(&mut self, isa: &TargetIsa) -> CodegenResult<()> {
        shrink_instructions(&mut self.func, isa);
        self.verify_if(isa)?;
        self.verify_locations_if(isa)?;
        Ok(())
    }

    /// Run the branch relaxation pass and return the final code size.
    pub fn relax_branches(&mut self, isa: &TargetIsa) -> CodegenResult<CodeOffset> {
        let code_size = relax_branches(&mut self.func, isa)?;
        self.verify_if(isa)?;
        self.verify_locations_if(isa)?;
        Ok(code_size)
    }
}
