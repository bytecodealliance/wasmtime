//! Liveness analysis for SSA values.
//!
//! This module computes the live range of all the SSA values in a function and produces a
//! `LiveRange` instance for each.
//!
//!
//! # Liveness consumers
//!
//! The primary consumer of the liveness analysis is the SSA coloring pass which goes through each
//! EBB and assigns a register to the defined values. This algorithm needs to maintain a set of the
//! currently live values as it is iterating down the instructions in the EBB. It asks the
//! following questions:
//!
//! - What is the set of live values at the entry to the EBB?
//! - When moving past a use of a value, is that value still alive in the EBB, or was that the last
//!   use?
//! - When moving past a branch, which of the live values are still live below the branch?
//!
//! The set of `LiveRange` instances can answer these questions through their `def_local_end` and
//! `livein_local_end` queries. The coloring algorithm visits EBBs in a topological order of the
//! dominator tree, so it can compute the set of live values at the beginning of an EBB by starting
//! from the set of live values at the dominating branch instruction and filtering it with
//! `livein_local_end`. These sets do not need to be stored in the liveness analysis.
//!
//! The secondary consumer of the liveness analysis is the spilling pass which needs to count the
//! number of live values at every program point and insert spill code until the number of
//! registers needed is small enough.
//!
//!
//! # Alternative algorithms
//!
//! A number of different liveness analysis algorithms exist, so it is worthwhile to look at a few
//! alternatives.
//!
//! ## Data-flow equations
//!
//! The classic *live variables analysis* that you will find in all compiler books from the
//! previous century does not depend on SSA form. It is typically implemented by iteratively
//! solving data-flow equations on bit-vectors of variables. The result is a live-out bit-vector of
//! variables for every basic block in the program.
//!
//! This algorithm has some disadvantages that makes us look elsewhere:
//!
//! - Quadratic memory use. We need a bit per variable per basic block in the function.
//! - Dense representation of sparse data. In practice, the majority of SSA values never leave
//!   their basic block, and those that do span basic blocks rarely span a large number of basic
//!   blocks. This makes the data stored in the bitvectors quite sparse.
//! - Traditionally, the data-flow equations were solved for real program *variables* which does
//!   not include temporaries used in evaluating expressions. We have an SSA form program which
//!   blurs the distinction between temporaries and variables. This makes the quadratic memory
//!   problem worse because there are many more SSA values than there was variables in the original
//!   program, and we don't know a priori which SSA values leave their basic block.
//! - Missing last-use information. For values that are not live-out of a basic block, we would
//!   need to store information about the last use in the block somewhere. LLVM stores this
//!   information as a 'kill bit' on the last use in the IR. Maintaining these kill bits has been a
//!   source of problems for LLVM's register allocator.
//!
//! Data-flow equations can detect when a variable is used uninitialized, and they can handle
//! multiple definitions of the same variable. We don't need this generality since we already have
//! a program in SSA form.
//!
//! ## LLVM's liveness analysis
//!
//! LLVM's register allocator computes liveness per *virtual register*, where a virtual register is
//! a disjoint union of related SSA values that should be assigned to the same physical register.
//! It uses a compact data structure very similar to our `LiveRange`. The important difference is
//! that Cranelift's `LiveRange` only describes a single SSA value, while LLVM's `LiveInterval`
//! describes the live range of a virtual register *and* which one of the related SSA values is
//! live at any given program point.
//!
//! LLVM computes the live range of each virtual register independently by using the use-def chains
//! that are baked into its IR. The algorithm for a single virtual register is:
//!
//! 1. Initialize the live range with a single-instruction snippet of liveness at each def, using
//!    the def-chain. This does not include any phi-values.
//! 2. Go through the virtual register's use chain and perform the following steps at each use:
//! 3. Perform an exhaustive depth-first traversal up the CFG from the use. Look for basic blocks
//!    that already contain some liveness and extend the last live SSA value in the block to be
//!    live-out. Also build a list of new basic blocks where the register needs to be live-in.
//! 4. Iteratively propagate live-out SSA values to the new live-in blocks. This may require new
//!    PHI values to be created when different SSA values can reach the same block.
//!
//! The iterative SSA form reconstruction can be skipped if the depth-first search only encountered
//! one SSA value.
//!
//! This algorithm has some advantages compared to the data-flow equations:
//!
//! - The live ranges of local virtual registers are computed very quickly without ever traversing
//!   the CFG. The memory needed to store these live ranges is independent of the number of basic
//!   blocks in the program.
//! - The time to compute the live range of a global virtual register is proportional to the number
//!   of basic blocks covered. Many virtual registers only cover a few blocks, even in very large
//!   functions.
//! - A single live range can be recomputed after making modifications to the IR. No global
//!   algorithm is necessary. This feature depends on having use-def chains for virtual registers
//!   which Cranelift doesn't.
//!
//! Cranelift uses a very similar data structures and algorithms to LLVM, with the important
//! difference that live ranges are computed per SSA value instead of per virtual register, and the
//! uses in Cranelift IR refers to SSA values instead of virtual registers. This means that
//! Cranelift can skip the last step of reconstructing SSA form for the virtual register uses.
//!
//! ## Fast Liveness Checking for SSA-Form Programs
//!
//! A liveness analysis that is often brought up in the context of SSA-based register allocation
//! was presented at CGO 2008:
//!
//! > Boissinot, B., Hack, S., Grund, D., de Dinechin, B. D., & Rastello, F. (2008). *Fast Liveness
//! Checking for SSA-Form Programs.* CGO.
//!
//! This analysis uses a global pre-computation that only depends on the CFG of the function. It
//! then allows liveness queries for any (value, program point) pair. Each query traverses the use
//! chain of the value and performs lookups in the precomputed bit-vectors.
//!
//! I did not seriously consider this analysis for Cranelift because:
//!
//! - It depends critically on use chains which Cranelift doesn't have.
//! - Popular variables like the `this` pointer in a C++ method can have very large use chains.
//!   Traversing such a long use chain on every liveness lookup has the potential for some nasty
//!   quadratic behavior in unfortunate cases.
//! - It says "fast" in the title, but the paper only claims to be 16% faster than a data-flow
//!   based approach, which isn't that impressive.
//!
//! Nevertheless, the property of only depending in the CFG structure is very useful. If Cranelift
//! gains use chains, this approach would be worth a proper evaluation.
//!
//!
//! # Cranelift's liveness analysis
//!
//! The algorithm implemented in this module is similar to LLVM's with these differences:
//!
//! - The `LiveRange` data structure describes the liveness of a single SSA value, not a virtual
//!   register.
//! - Instructions in Cranelift IR contains references to SSA values, not virtual registers.
//! - All live ranges are computed in one traversal of the program. Cranelift doesn't have use
//!   chains, so it is not possible to compute the live range for a single SSA value independently.
//!
//! The liveness computation visits all instructions in the program. The order is not important for
//! the algorithm to be correct. At each instruction, the used values are examined.
//!
//! - The first time a value is encountered, its live range is constructed as a dead live range
//!   containing only the defining program point.
//! - The local interval of the value's live range is extended so it reaches the use. This may
//!   require creating a new live-in local interval for the EBB.
//! - If the live range became live-in to the EBB, add the EBB to a work-list.
//! - While the work-list is non-empty pop a live-in EBB and repeat the two steps above, using each
//!   of the live-in EBB's CFG predecessor instructions as a 'use'.
//!
//! The effect of this algorithm is to extend the live range of each to reach uses as they are
//! visited. No data about each value beyond the live range is needed between visiting uses, so
//! nothing is lost by computing the live range of all values simultaneously.
//!
//! ## Cache efficiency of Cranelift vs LLVM
//!
//! Since LLVM computes the complete live range of a virtual register in one go, it can keep the
//! whole `LiveInterval` for the register in L1 cache. Since it is visiting the instructions in use
//! chain order, some cache thrashing can occur as a result of pulling instructions into cache
//! somewhat chaotically.
//!
//! Cranelift uses a transposed algorithm, visiting instructions in order. This means that each
//! instruction is brought into cache only once, and it is likely that the other instructions on
//! the same cache line will be visited before the line is evicted.
//!
//! Cranelift's problem is that the `LiveRange` structs are visited many times and not always
//! regularly. We should strive to make the `LiveRange` struct as small as possible such that
//! multiple related values can live on the same cache line.
//!
//! - Local values should fit in a 16-byte `LiveRange` struct or smaller. The current
//!   implementation contains a 24-byte `Vec` object and a redundant `value` member pushing the
//!   size to 32 bytes.
//! - Related values should be stored on the same cache line. The current sparse set implementation
//!   does a decent job of that.
//! - For global values, the list of live-in intervals is very likely to fit on a single cache
//!   line. These lists are very likely to be found in L2 cache at least.
//!
//! There is some room for improvement.

use entity::SparseMap;
use flowgraph::{BasicBlock, ControlFlowGraph};
use ir::dfg::ValueDef;
use ir::{Ebb, Function, Inst, Layout, ProgramPoint, Value};
use isa::{EncInfo, OperandConstraint, TargetIsa};
use regalloc::affinity::Affinity;
use regalloc::liverange::{LiveRange, LiveRangeContext, LiveRangeForest};
use std::mem;
use std::ops::Index;
use std::vec::Vec;
use timing;

/// A set of live ranges, indexed by value number.
type LiveRangeSet = SparseMap<Value, LiveRange>;

/// Get a mutable reference to the live range for `value`.
/// Create it if necessary.
fn get_or_create<'a>(
    lrset: &'a mut LiveRangeSet,
    value: Value,
    isa: &TargetIsa,
    func: &Function,
    encinfo: &EncInfo,
) -> &'a mut LiveRange {
    // It would be better to use `get_mut()` here, but that leads to borrow checker fighting
    // which can probably only be resolved by non-lexical lifetimes.
    // https://github.com/rust-lang/rfcs/issues/811
    if lrset.get(value).is_none() {
        // Create a live range for value. We need the program point that defines it.
        let def;
        let affinity;
        match func.dfg.value_def(value) {
            ValueDef::Result(inst, rnum) => {
                def = inst.into();
                // Initialize the affinity from the defining instruction's result constraints.
                // Don't do this for call return values which are always tied to a single register.
                affinity = encinfo
                    .operand_constraints(func.encodings[inst])
                    .and_then(|rc| rc.outs.get(rnum))
                    .map(Affinity::new)
                    .or_else(|| {
                        // If this is a call, get the return value affinity.
                        func.dfg
                            .call_signature(inst)
                            .map(|sig| Affinity::abi(&func.dfg.signatures[sig].returns[rnum], isa))
                    })
                    .unwrap_or_default();
            }
            ValueDef::Param(ebb, num) => {
                def = ebb.into();
                if func.layout.entry_block() == Some(ebb) {
                    // The affinity for entry block parameters can be inferred from the function
                    // signature.
                    affinity = Affinity::abi(&func.signature.params[num], isa);
                } else {
                    // Give normal EBB parameters a register affinity matching their type.
                    let rc = isa.regclass_for_abi_type(func.dfg.value_type(value));
                    affinity = Affinity::Reg(rc.into());
                }
            }
        };
        lrset.insert(LiveRange::new(value, def, affinity));
    }
    lrset.get_mut(value).unwrap()
}

/// Extend the live range for `value` so it reaches `to` which must live in `ebb`.
fn extend_to_use(
    lr: &mut LiveRange,
    ebb: Ebb,
    to: Inst,
    worklist: &mut Vec<Ebb>,
    func: &Function,
    cfg: &ControlFlowGraph,
    forest: &mut LiveRangeForest,
) {
    // This is our scratch working space, and we'll leave it empty when we return.
    debug_assert!(worklist.is_empty());

    // Extend the range locally in `ebb`.
    // If there already was a live interval in that block, we're done.
    if lr.extend_in_ebb(ebb, to, &func.layout, forest) {
        worklist.push(ebb);
    }

    // The work list contains those EBBs where we have learned that the value needs to be
    // live-in.
    //
    // This algorithm becomes a depth-first traversal up the CFG, enumerating all paths through the
    // CFG from the existing live range to `ebb`.
    //
    // Extend the live range as we go. The live range itself also serves as a visited set since
    // `extend_in_ebb` will never return true twice for the same EBB.
    //
    while let Some(livein) = worklist.pop() {
        // We've learned that the value needs to be live-in to the `livein` EBB.
        // Make sure it is also live at all predecessor branches to `livein`.
        for BasicBlock {
            ebb: pred,
            inst: branch,
        } in cfg.pred_iter(livein)
        {
            if lr.extend_in_ebb(pred, branch, &func.layout, forest) {
                // This predecessor EBB also became live-in. We need to process it later.
                worklist.push(pred);
            }
        }
    }
}

/// Liveness analysis for a function.
///
/// Compute a live range for every SSA value used in the function.
pub struct Liveness {
    /// The live ranges that have been computed so far.
    ranges: LiveRangeSet,

    /// Memory pool for the live ranges.
    forest: LiveRangeForest,

    /// Working space for the `extend_to_use` algorithm.
    /// This vector is always empty, except for inside that function.
    /// It lives here to avoid repeated allocation of scratch memory.
    worklist: Vec<Ebb>,
}

impl Liveness {
    /// Create a new empty liveness analysis.
    ///
    /// The memory allocated for this analysis can be reused for multiple functions. Use the
    /// `compute` method to actually runs the analysis for a function.
    pub fn new() -> Self {
        Self {
            ranges: LiveRangeSet::new(),
            forest: LiveRangeForest::new(),
            worklist: Vec::new(),
        }
    }

    /// Get a context needed for working with a `LiveRange`.
    pub fn context<'a>(&'a self, layout: &'a Layout) -> LiveRangeContext<'a, Layout> {
        LiveRangeContext::new(layout, &self.forest)
    }

    /// Clear all data structures in this liveness analysis.
    pub fn clear(&mut self) {
        self.ranges.clear();
        self.forest.clear();
        self.worklist.clear();
    }

    /// Get the live range for `value`, if it exists.
    pub fn get(&self, value: Value) -> Option<&LiveRange> {
        self.ranges.get(value)
    }

    /// Create a new live range for `value`.
    ///
    /// The new live range will be defined at `def` with no extent, like a dead value.
    ///
    /// This asserts that `value` does not have an existing live range.
    pub fn create_dead<PP>(&mut self, value: Value, def: PP, affinity: Affinity)
    where
        PP: Into<ProgramPoint>,
    {
        let old = self
            .ranges
            .insert(LiveRange::new(value, def.into(), affinity));
        debug_assert!(old.is_none(), "{} already has a live range", value);
    }

    /// Move the definition of `value` to `def`.
    ///
    /// The old and new def points must be in the same EBB, and before the end of the live range.
    pub fn move_def_locally<PP>(&mut self, value: Value, def: PP)
    where
        PP: Into<ProgramPoint>,
    {
        let lr = self.ranges.get_mut(value).expect("Value has no live range");
        lr.move_def_locally(def.into());
    }

    /// Locally extend the live range for `value` to reach `user`.
    ///
    /// It is assumed the `value` is already live before `user` in `ebb`.
    ///
    /// Returns a mutable reference to the value's affinity in case that also needs to be updated.
    pub fn extend_locally(
        &mut self,
        value: Value,
        ebb: Ebb,
        user: Inst,
        layout: &Layout,
    ) -> &mut Affinity {
        debug_assert_eq!(Some(ebb), layout.inst_ebb(user));
        let lr = self.ranges.get_mut(value).expect("Value has no live range");
        let livein = lr.extend_in_ebb(ebb, user, layout, &mut self.forest);
        debug_assert!(!livein, "{} should already be live in {}", value, ebb);
        &mut lr.affinity
    }

    /// Change the affinity of `value` to `Stack` and return the previous affinity.
    pub fn spill(&mut self, value: Value) -> Affinity {
        let lr = self.ranges.get_mut(value).expect("Value has no live range");
        mem::replace(&mut lr.affinity, Affinity::Stack)
    }

    /// Compute the live ranges of all SSA values used in `func`.
    /// This clears out any existing analysis stored in this data structure.
    pub fn compute(&mut self, isa: &TargetIsa, func: &mut Function, cfg: &ControlFlowGraph) {
        let _tt = timing::ra_liveness();
        self.ranges.clear();

        // Get ISA data structures used for computing live range affinities.
        let encinfo = isa.encoding_info();
        let reginfo = isa.register_info();

        // The liveness computation needs to visit all uses, but the order doesn't matter.
        // TODO: Perhaps this traversal of the function could be combined with a dead code
        // elimination pass if we visit a post-order of the dominator tree?
        // TODO: Resolve value aliases while we're visiting instructions?
        for ebb in func.layout.ebbs() {
            // Make sure we have created live ranges for dead EBB parameters.
            // TODO: If these parameters are really dead, we could remove them, except for the
            // entry block which must match the function signature.
            for &arg in func.dfg.ebb_params(ebb) {
                get_or_create(&mut self.ranges, arg, isa, func, &encinfo);
            }

            for inst in func.layout.ebb_insts(ebb) {
                // Eliminate all value aliases, they would confuse the register allocator.
                func.dfg.resolve_aliases_in_arguments(inst);

                // Make sure we have created live ranges for dead defs.
                // TODO: When we implement DCE, we can use the absence of a live range to indicate
                // an unused value.
                for &def in func.dfg.inst_results(inst) {
                    get_or_create(&mut self.ranges, def, isa, func, &encinfo);
                }

                // Iterator of constraints, one per value operand.
                let encoding = func.encodings[inst];
                let operand_constraint_slice: &[OperandConstraint] =
                    encinfo.operand_constraints(encoding).map_or(&[], |c| c.ins);
                let mut operand_constraints = operand_constraint_slice.iter();

                for &arg in func.dfg.inst_args(inst) {
                    // Get the live range, create it as a dead range if necessary.
                    let lr = get_or_create(&mut self.ranges, arg, isa, func, &encinfo);

                    // Extend the live range to reach this use.
                    extend_to_use(
                        lr,
                        ebb,
                        inst,
                        &mut self.worklist,
                        func,
                        cfg,
                        &mut self.forest,
                    );

                    // Apply operand constraint, ignoring any variable arguments after the fixed
                    // operands described by `operand_constraints`. Variable arguments are either
                    // EBB arguments or call/return ABI arguments.
                    if let Some(constraint) = operand_constraints.next() {
                        lr.affinity.merge(constraint, &reginfo);
                    }
                }
            }
        }
    }
}

impl Index<Value> for Liveness {
    type Output = LiveRange;

    fn index(&self, index: Value) -> &LiveRange {
        match self.ranges.get(index) {
            Some(lr) => lr,
            None => panic!("{} has no live range", index),
        }
    }
}
