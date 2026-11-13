//! Placing a pure instruction early where it ends one of its
//! operands' live ranges.
//!
//! Elaboration places a pure instruction just before its first use in
//! the skeleton, which keeps every operand live until then. A running
//! sum, `acc2 = fadd acc1, v2`, whose only skeleton use is a return,
//! would keep each `v` live from its call to the return, and register
//! allocation would then cost the square of the calls. Moved to just
//! after its latest operand, such an instruction ends the range of the
//! operand that it kills and begins its own, so no range grows.

use super::Stats;
use crate::FxHashMap;
use crate::dominator_tree::{DomTreeGraph, DominatorTree};
use crate::flowgraph::ControlFlowGraph;
use crate::inst_predicates::is_pure_for_egraph;
use crate::ir::{Block, Function, Inst, ProgramPoint, Value, ValueDef};
use crate::loop_analysis::LoopAnalysis;
use cranelift_entity::{EntitySet, SecondaryMap};
use smallvec::SmallVec;

/// Move each pure instruction that is the last use of one of its
/// non-constant operands up to just after its latest operand's
/// definition.
///
/// An instruction moves only past a skeleton instruction, which a
/// range that it ends would otherwise span. A move never makes an
/// instruction run where it did not:
/// - within its block it always may move;
/// - into another block it moves only where its own block
///   post-dominates that one and lies in every loop containing it;
/// - a cold block is treated as never reached, so the guards that
///   leave to one do not stop a move.
///
/// A constant operand does not constrain where an instruction moves.
/// It moves with its one user, or else is copied before the user once
/// for each block where its definition no longer dominates the user.
pub(crate) fn place_early(
    func: &mut Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    loops: &LoopAnalysis,
    stats: &mut Stats,
) {
    let mut uses: SecondaryMap<Value, u32> = SecondaryMap::new();
    let mut in_skeleton: SecondaryMap<Value, bool> = SecondaryMap::new();
    uses.resize(func.dfg.num_values());
    in_skeleton.resize(func.dfg.num_values());
    let mut pure: EntitySet<Inst> = EntitySet::new();
    // The skeleton instructions before each instruction in its block.
    let mut effects_before: SecondaryMap<Inst, u32> = SecondaryMap::new();
    effects_before.resize(func.dfg.num_insts());
    for block in func.layout.blocks() {
        let mut effects = 0;
        for inst in func.layout.block_insts(block) {
            let is_pure = is_pure_for_egraph(func, inst);
            if is_pure {
                pure.insert(inst);
            }
            effects_before[inst] = effects;
            effects += u32::from(!is_pure);
            for value in func.dfg.inst_values(inst) {
                uses[value] += 1;
                in_skeleton[value] |= !is_pure;
            }
        }
    }
    let mut postdom: Option<WarmPostdom> = None;
    let is_constant = |func: &Function, value: Value| match func.dfg.value_def(value) {
        ValueDef::Result(def, _) => {
            pure.contains(def) && func.dfg.inst_values(def).next().is_none()
        }
        _ => false,
    };
    let mut args: SmallVec<[Value; 8]> = SmallVec::new();
    let mut copies: FxHashMap<(Block, Value), Value> = FxHashMap::default();
    for &block in domtree.cfg_rpo() {
        if !func.layout.is_block_inserted(block) {
            continue;
        }
        let mut next = func.layout.first_inst(block);
        while let Some(inst) = next {
            next = func.layout.next_inst(inst);
            // Elaboration placed a value that the skeleton uses just before
            // that use, where lowering may merge the two.
            if !pure.contains(inst) || in_skeleton[func.dfg.first_result(inst)] {
                continue;
            }
            args.clear();
            args.extend(func.dfg.inst_values(inst));
            let mut latest: Option<ProgramPoint> = None;
            let mut kills = false;
            for &arg in &args {
                if is_constant(func, arg) {
                    continue;
                }
                let here = args.iter().filter(|&&a| a == arg).count() as u32;
                kills |= uses[arg] == here;
                let def = match func.dfg.value_def(arg) {
                    ValueDef::Result(def, _) => ProgramPoint::Inst(def),
                    ValueDef::Param(block, _) => ProgramPoint::Block(block),
                    ValueDef::Union(..) => unreachable!("no union survives elaboration"),
                };
                if latest.is_none_or(|l| domtree.dominates(l, def, &func.layout)) {
                    latest = Some(def);
                }
            }
            let before = match latest {
                Some(ProgramPoint::Inst(def)) => func.layout.next_inst(def),
                Some(ProgramPoint::Block(block)) => func.layout.first_inst(block),
                None => None,
            };
            let Some(before) = before.filter(|&b| kills && b != inst) else {
                continue;
            };
            let to = func.layout.inst_block(before).unwrap();
            // A range that ends before the next skeleton instruction costs
            // nothing to keep.
            if to == block && effects_before[before] == effects_before[inst] {
                continue;
            }
            if to != block {
                let postdom =
                    postdom.get_or_insert_with(|| WarmPostdom::compute(func, cfg, domtree));
                if !postdom.postdominates(block, to) {
                    continue;
                }
                if let Some(lp) = loops.innermost_loop(to) {
                    if !loops.is_in_loop(block, lp) {
                        continue;
                    }
                }
            }
            func.layout.remove_inst(inst);
            func.layout.insert_inst(inst, before);
            effects_before[inst] = effects_before[before];
            stats.elaborate_place_early += 1;
            for i in 0..args.len() {
                let arg = args[i];
                let ValueDef::Result(def, _) = func.dfg.value_def(arg) else {
                    continue;
                };
                if !is_constant(func, arg) || domtree.dominates(def, inst, &func.layout) {
                    continue;
                }
                if uses[arg] == args.iter().filter(|&&a| a == arg).count() as u32 {
                    func.layout.remove_inst(def);
                    func.layout.insert_inst(def, inst);
                    effects_before[def] = effects_before[inst];
                    continue;
                }
                // One copy for each block, as elaboration rematerializes a
                // constant, rather than one for each moved instruction.
                let shared = copies.get(&(to, arg)).copied().filter(|&copy| {
                    let ValueDef::Result(copy, _) = func.dfg.value_def(copy) else {
                        unreachable!("a copy is an instruction's result")
                    };
                    domtree.dominates(copy, inst, &func.layout)
                });
                args[i] = shared.unwrap_or_else(|| {
                    let copy = func.dfg.clone_inst(def);
                    func.layout.insert_inst(copy, inst);
                    effects_before[copy] = effects_before[inst];
                    let value = func.dfg.first_result(copy);
                    copies.insert((to, arg), value);
                    value
                });
            }
            func.dfg.overwrite_inst_values(inst, args.iter().copied());
        }
    }
}

/// Reverse only warm, reachable CFG edges, as in `PostDominatorTree`, so
/// cold error exits do not prevent early placement on the normal path.
struct WarmReverseGraph<'a> {
    func: &'a Function,
    cfg: &'a ControlFlowGraph,
    domtree: &'a DominatorTree,
}

impl WarmReverseGraph<'_> {
    fn warm(&self, block: Block) -> bool {
        self.func.layout.is_block_inserted(block)
            && self.domtree.is_reachable(block)
            && !self.func.layout.is_cold(block)
    }
}

impl DomTreeGraph for WarmReverseGraph<'_> {
    fn num_blocks(&self) -> usize {
        self.func.dfg.num_blocks()
    }

    fn roots(&self) -> impl Iterator<Item = Block> {
        self.domtree
            .cfg_rpo()
            .copied()
            .filter(|&block| self.warm(block) && self.predecessors(block).next().is_none())
    }

    fn successors(&self, block: Block) -> impl Iterator<Item = Block> {
        self.cfg
            .pred_iter(block)
            .map(|pred| pred.block)
            .filter(|&pred| self.warm(pred))
    }

    fn predecessors(&self, block: Block) -> impl Iterator<Item = Block> {
        self.cfg.succ_iter(block).filter(|&succ| self.warm(succ))
    }
}

/// The post-dominator tree of the warm blocks that reach an exit.
/// Reuse the ordinary dominator-tree machinery on the reversed graph.
struct WarmPostdom(DominatorTree);

impl WarmPostdom {
    fn compute(func: &Function, cfg: &ControlFlowGraph, domtree: &DominatorTree) -> Self {
        let mut tree = DominatorTree::new();
        tree.compute_from_graph(&WarmReverseGraph { func, cfg, domtree });
        Self(tree)
    }

    /// Whether every warm path from `b` to an exit passes through `a`.
    /// Blocks that cannot reach a warm exit do not participate in the tree.
    fn postdominates(&self, a: Block, b: Block) -> bool {
        self.0.is_reachable(a) && self.0.is_reachable(b) && self.0.block_dominates(a, b)
    }
}
