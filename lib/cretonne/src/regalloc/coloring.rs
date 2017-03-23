//! Register allocator coloring pass.
//!
//! The coloring pass assigns a physical register to every SSA value with a register affinity,
//! under the assumption that the register pressure has been lowered sufficiently by spilling and
//! splitting.
//!
//! # Preconditions
//!
//! The coloring pass doesn't work on arbitrary code. Certain preconditions must be satisfied:
//!
//! 1. All instructions must be legalized and assigned an encoding. The encoding recipe guides the
//!    register assignments and provides exact constraints.
//!
//! 2. Instructions with tied operands must be in a coloring-friendly state. Specifically, the
//!    values used by the tied operands must be killed by the instruction. This can be achieved by
//!    inserting a `copy` to a new value immediately before the two-address instruction.
//!
//! 3. The register pressure must be lowered sufficiently by inserting spill code. Register
//!    operands are allowed to read spilled values, but each such instance must be counted as using
//!    a register.
//!
//! # Iteration order
//!
//! The SSA property guarantees that whenever the live range of two values overlap, one of the
//! values will be live at the definition point of the other value. If we visit the instructions in
//! a topological order relative to the dominance relation, we can assign colors to the values
//! defined by the instruction and only consider the colors of other values that are live at the
//! instruction.
//!
//! The topological order of instructions inside an EBB is simply the layout order, starting from
//! the EBB header. A topological order of the EBBs can only visit an EBB once its immediate
//! dominator has been visited.
//!
//! There are many valid topological orders of the EBBs, and the specific order can affect which
//! coloring hints are satisfied and which are broken.
//!

use entity_map::EntityMap;
use dominator_tree::DominatorTree;
use ir::{Ebb, Inst, Value, Function, Cursor, ValueLoc, DataFlowGraph};
use isa::{TargetIsa, RegInfo, Encoding, RecipeConstraints, ConstraintKind};
use regalloc::affinity::Affinity;
use regalloc::allocatable_set::AllocatableSet;
use regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use regalloc::liveness::Liveness;
use sparse_map::SparseSet;


/// Data structures for the coloring pass.
///
/// These are scratch space data structures that can be reused between invocations.
pub struct Coloring {
    /// Set of visited EBBs.
    visited: SparseSet<Ebb>,

    /// Stack of EBBs to be visited next.
    stack: Vec<Ebb>,
}

/// Bundle of references that the coloring algorithm needs.
///
/// Some of the needed mutable references are passed around as explicit function arguments so we
/// can avoid many fights with the borrow checker over mutable borrows of `self`. This includes the
/// `Function` and `LiveValueTracker` references.
///
/// Immutable context information and mutable references that don't need to be borrowed across
/// method calls should go in this struct.
struct Context<'a> {
    // Cached ISA information.
    // We save it here to avoid frequent virtual function calls on the `TargetIsa` trait object.
    reginfo: RegInfo,
    recipe_constraints: &'a [RecipeConstraints],

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,

    // Pristine set of registers that the allocator can use.
    // This set remains immutable, we make clones.
    usable_regs: AllocatableSet,
}

impl Coloring {
    /// Allocate scratch space data structures for the coloring pass.
    pub fn new() -> Coloring {
        Coloring {
            visited: SparseSet::new(),
            stack: Vec::new(),
        }
    }

    /// Run the coloring algorithm over `func`.
    pub fn run(&mut self,
               isa: &TargetIsa,
               func: &mut Function,
               domtree: &DominatorTree,
               liveness: &mut Liveness,
               tracker: &mut LiveValueTracker) {
        let mut ctx = Context {
            reginfo: isa.register_info(),
            recipe_constraints: isa.recipe_constraints(),
            domtree: domtree,
            liveness: liveness,
            // TODO: Ask the target ISA about reserved registers etc.
            usable_regs: AllocatableSet::new(),
        };
        ctx.run(self, func, tracker)
    }
}

impl<'a> Context<'a> {
    /// Run the coloring algorithm.
    fn run(&mut self, data: &mut Coloring, func: &mut Function, tracker: &mut LiveValueTracker) {
        // Just visit blocks in layout order, letting `process_ebb` enforce a topological ordering.
        // TODO: Once we have a loop tree, we could visit hot blocks first.
        let mut next = func.layout.entry_block();
        while let Some(ebb) = next {
            self.process_ebb(ebb, data, func, tracker);
            next = func.layout.next_ebb(ebb);
        }
    }

    /// Process `ebb`, but only after ensuring that the immediate dominator has been processed.
    ///
    /// This method can be called with the most desired order of visiting the EBBs. It will convert
    /// that order into a valid topological order by visiting dominators first.
    fn process_ebb(&mut self,
                   mut ebb: Ebb,
                   data: &mut Coloring,
                   func: &mut Function,
                   tracker: &mut LiveValueTracker) {
        // The stack is just a scratch space for this algorithm. We leave it empty when returning.
        assert!(data.stack.is_empty());

        // Trace up the dominator tree until we reach a dominator that has already been visited.
        while data.visited.insert(ebb).is_none() {
            data.stack.push(ebb);
            match self.domtree.idom(ebb) {
                Some(idom) => ebb = func.layout.inst_ebb(idom).expect("idom not in layout"),
                None => break,
            }
        }

        // Pop off blocks in topological order.
        while let Some(ebb) = data.stack.pop() {
            self.visit_ebb(ebb, func, tracker);
        }
    }

    /// Visit `ebb`, assuming that the immediate dominator has already been visited.
    fn visit_ebb(&mut self, ebb: Ebb, func: &mut Function, tracker: &mut LiveValueTracker) {
        let mut regs = self.visit_ebb_header(ebb, func, tracker);

        // Now go through the instructions in `ebb` and color the values they define.
        let mut pos = Cursor::new(&mut func.layout);
        pos.goto_top(ebb);
        while let Some(inst) = pos.next_inst() {
            let encoding = func.encodings[inst];
            assert!(encoding.is_legal(), "Illegal: {}", func.dfg[inst].opcode());
            self.visit_inst(inst,
                            encoding,
                            &mut pos,
                            &mut func.dfg,
                            tracker,
                            &mut regs,
                            &mut func.locations);
            tracker.drop_dead(inst);
        }

    }

    /// Visit the `ebb` header.
    ///
    /// Initialize the set of live registers and color the arguments to `ebb`.
    fn visit_ebb_header(&self,
                        ebb: Ebb,
                        func: &mut Function,
                        tracker: &mut LiveValueTracker)
                        -> AllocatableSet {
        // Reposition the live value tracker and deal with the EBB arguments.
        let (liveins, args) =
            tracker.ebb_top(ebb, &func.dfg, self.liveness, &func.layout, self.domtree);

        // The live-ins have already been assigned a register. Reconstruct the allocatable set.
        let mut regs = self.livein_regs(liveins, func);

        // TODO: Arguments to the entry block are pre-colored by the ABI. We should probably call
        // a whole other function for that case.
        self.color_args(args, &mut regs, &mut func.locations);

        regs
    }

    /// Initialize a set of allocatable registers from the values that are live-in to a block.
    /// These values must already be colored when the dominating blocks were processed.
    fn livein_regs(&self, liveins: &[LiveValue], func: &Function) -> AllocatableSet {
        // Start from the registers that are actually usable. We don't want to include any reserved
        // registers in the set.
        let mut regs = self.usable_regs.clone();

        for lv in liveins {
            let value = lv.value;
            let affinity = self.liveness
                .get(value)
                .expect("No live range for live-in")
                .affinity;
            if let Affinity::Reg(rc_index) = affinity {
                let regclass = self.reginfo.rc(rc_index);
                match func.locations[value] {
                    ValueLoc::Reg(regunit) => regs.take(regclass, regunit),
                    ValueLoc::Unassigned => panic!("Live-in {} wasn't assigned", value),
                    ValueLoc::Stack(ss) => {
                        panic!("Live-in {} is in {}, should be register", value, ss)
                    }
                }
            }
        }

        regs
    }

    /// Color the live arguments to the current block.
    ///
    /// It is assumed that any live-in register values have already been taken out of the register
    /// set.
    fn color_args(&self,
                  args: &[LiveValue],
                  regs: &mut AllocatableSet,
                  locations: &mut EntityMap<Value, ValueLoc>) {
        for lv in args {
            // Only look at the register arguments.
            if let Affinity::Reg(rc_index) = lv.affinity {
                let regclass = self.reginfo.rc(rc_index);
                // TODO: Fall back to a top-level super-class. Sub-classes are only hints.
                let regunit = regs.iter(regclass).next().expect("Out of registers for arguments");
                regs.take(regclass, regunit);
                *locations.ensure(lv.value) = ValueLoc::Reg(regunit);
            }
        }
    }

    /// Color the values defined by `inst` and insert any necessary shuffle code to satisfy
    /// instruction constraints.
    ///
    /// Update `regs` to reflect the allocated registers after `inst`, including removing any dead
    /// or killed values from the set.
    fn visit_inst(&self,
                  inst: Inst,
                  encoding: Encoding,
                  _pos: &mut Cursor,
                  dfg: &mut DataFlowGraph,
                  tracker: &mut LiveValueTracker,
                  regs: &mut AllocatableSet,
                  locations: &mut EntityMap<Value, ValueLoc>) {
        // First update the live value tracker with this instruction.
        // Get lists of values that are killed and defined by `inst`.
        let (kills, defs) = tracker.process_inst(inst, dfg, self.liveness);

        // Get the operand constraints for `inst` that we are trying to satisfy.
        let constraints = self.recipe_constraints[encoding.recipe()].clone();

        // Get rid of the killed values.
        for lv in kills {
            if let Affinity::Reg(rc_index) = lv.affinity {
                let regclass = self.reginfo.rc(rc_index);
                if let ValueLoc::Reg(regunit) = locations[lv.value] {
                    regs.free(regclass, regunit);
                }
            }
        }

        // Process the defined values with fixed constraints.
        // TODO: Handle constraints on call return values.
        assert_eq!(defs.len(),
                   constraints.outs.len(),
                   "Can't handle variable results");
        for (lv, opcst) in defs.iter().zip(constraints.outs) {
            match lv.affinity {
                // This value should go in a register.
                Affinity::Reg(rc_index) => {
                    // The preferred register class is not a requirement.
                    let pref_rc = self.reginfo.rc(rc_index);
                    match opcst.kind {
                        ConstraintKind::Reg => {
                            // This is a standard register constraint. The preferred register class
                            // should have been computed as a subclass of the hard constraint of
                            // the def.
                            assert!(opcst.regclass.has_subclass(rc_index),
                                    "{} preference {} is not compatible with the definition \
                                     constraint {}",
                                    lv.value,
                                    pref_rc.name,
                                    opcst.regclass.name);
                            // Try to grab a register from the preferred class, but fall back to
                            // the actual constraint if we have to.
                            let regunit = regs.iter(pref_rc)
                                .next()
                                .or_else(|| regs.iter(opcst.regclass).next())
                                .expect("Ran out of registers");
                            regs.take(opcst.regclass, regunit);
                            *locations.ensure(lv.value) = ValueLoc::Reg(regunit);
                        }
                        ConstraintKind::Tied(arg_index) => {
                            // This def must use the same register as a fixed instruction argument.
                            let arg = dfg.inst_args(inst)[arg_index as usize];
                            let loc = locations[arg];
                            *locations.ensure(lv.value) = loc;
                            // Mark the reused register. It's not really clear if we support tied
                            // stack operands. We could do that for some Intel read-modify-write
                            // encodings.
                            if let ValueLoc::Reg(regunit) = loc {
                                // This is going to assert out unless the incoming value at
                                // `arg_index` was killed. Tied operands must be fixed to
                                // ensure that before running the coloring pass.
                                regs.take(opcst.regclass, regunit);
                            }
                        }
                        ConstraintKind::FixedReg(_regunit) => unimplemented!(),
                        ConstraintKind::Stack => {
                            panic!("{}:{} should be a stack value", lv.value, pref_rc.name)
                        }
                    }
                }
                Affinity::Stack => unimplemented!(),
                Affinity::Any => unimplemented!(),
            }
        }

        // Get rid of the dead defs.
        for lv in defs {
            if lv.endpoint == inst {
                if let Affinity::Reg(rc_index) = lv.affinity {
                    let regclass = self.reginfo.rc(rc_index);
                    if let ValueLoc::Reg(regunit) = locations[lv.value] {
                        regs.free(regclass, regunit);
                    }
                }
            }
        }
    }
}
