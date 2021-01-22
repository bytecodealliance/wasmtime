//! Debug info analysis: computes value-label ranges from value-label markers in
//! generated VCode.
//!
//! We "reverse-engineer" debug info like this because it is far more reliable
//! than generating it while emitting code and keeping it in sync.
//!
//! This works by (i) observing "value-label marker" instructions, which are
//! semantically just an assignment from a register to a "value label" (which
//! one can think of as another register; they represent, e.g., Wasm locals) at
//! a certain point in the code, and (ii) observing loads and stores to the
//! stack and register moves.
//!
//! We track, at every program point, the correspondence between each value
//! label and *all* locations in which it resides. E.g., if it is stored to the
//! stack, we remember that it is in both a register and the stack slot; but if
//! the register is later overwritten, then we have it just in the stack slot.
//! This allows us to avoid false-positives observing loads/stores that we think
//! are spillslots but really aren't.
//!
//! We do a standard forward dataflow analysis to compute this info.

use crate::ir::ValueLabel;
use crate::machinst::*;
use crate::value_label::{LabelValueLoc, ValueLabelsRanges, ValueLocRange};
use log::trace;
use regalloc::{Reg, RegUsageCollector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ValueLoc {
    Reg(Reg),
    /// Nominal-SP offset.
    Stack(i64),
}

impl From<ValueLoc> for LabelValueLoc {
    fn from(v: ValueLoc) -> Self {
        match v {
            ValueLoc::Reg(r) => LabelValueLoc::Reg(r),
            ValueLoc::Stack(off) => LabelValueLoc::SPOffset(off),
        }
    }
}

impl ValueLoc {
    fn is_reg(self) -> bool {
        match self {
            ValueLoc::Reg(_) => true,
            _ => false,
        }
    }
    fn is_stack(self) -> bool {
        match self {
            ValueLoc::Stack(_) => true,
            _ => false,
        }
    }
}

/// Mappings at one program point.
#[derive(Clone, Debug)]
struct AnalysisInfo {
    // nominal SP relative to real SP.
    nominal_sp_offset: Option<i64>,
    label_to_locs: HashMap<ValueLabel, HashSet<ValueLoc>>,
    reg_to_label: HashMap<Reg, ValueLabel>,
    stack_to_label: HashMap<i64, ValueLabel>,
}

fn get_inst_writes<M: MachInst>(m: &M) -> Vec<Reg> {
    // TODO: expose this part of regalloc.rs's interface publicly.
    let mut vecs = RegUsageCollector::get_empty_reg_vecs_test_framework_only(false);
    let mut coll = RegUsageCollector::new(&mut vecs);
    m.get_regs(&mut coll);
    vecs.defs.extend(vecs.mods.into_iter());
    vecs.defs
}

impl AnalysisInfo {
    fn new() -> Self {
        AnalysisInfo {
            nominal_sp_offset: Some(0),
            label_to_locs: HashMap::new(),
            reg_to_label: HashMap::new(),
            stack_to_label: HashMap::new(),
        }
    }

    fn clear_label(&mut self, label: ValueLabel) {
        if let Some(locs) = self.label_to_locs.remove(&label) {
            for loc in locs {
                match loc {
                    ValueLoc::Reg(r) => {
                        self.reg_to_label.remove(&r);
                    }
                    ValueLoc::Stack(off) => {
                        self.stack_to_label.remove(&off);
                    }
                }
            }
        }
    }
    fn clear_reg(&mut self, reg: Reg) {
        if let Some(label) = self.reg_to_label.remove(&reg) {
            if let Some(locs) = self.label_to_locs.get_mut(&label) {
                locs.remove(&ValueLoc::Reg(reg));
            }
        }
    }
    fn clear_stack_off(&mut self, off: i64) {
        if let Some(label) = self.stack_to_label.remove(&off) {
            if let Some(locs) = self.label_to_locs.get_mut(&label) {
                locs.remove(&ValueLoc::Stack(off));
            }
        }
    }
    fn def_label_at_reg(&mut self, label: ValueLabel, reg: Reg) {
        self.clear_label(label);
        self.label_to_locs
            .entry(label)
            .or_insert_with(|| HashSet::new())
            .insert(ValueLoc::Reg(reg));
        self.reg_to_label.insert(reg, label);
    }
    fn store_reg(&mut self, reg: Reg, off: i64) {
        self.clear_stack_off(off);
        if let Some(label) = self.reg_to_label.get(&reg) {
            if let Some(locs) = self.label_to_locs.get_mut(label) {
                locs.insert(ValueLoc::Stack(off));
            }
            self.stack_to_label.insert(off, *label);
        }
    }
    fn load_reg(&mut self, reg: Reg, off: i64) {
        self.clear_reg(reg);
        if let Some(&label) = self.stack_to_label.get(&off) {
            if let Some(locs) = self.label_to_locs.get_mut(&label) {
                locs.insert(ValueLoc::Reg(reg));
            }
            self.reg_to_label.insert(reg, label);
        }
    }
    fn move_reg(&mut self, to: Reg, from: Reg) {
        self.clear_reg(to);
        if let Some(&label) = self.reg_to_label.get(&from) {
            if let Some(locs) = self.label_to_locs.get_mut(&label) {
                locs.insert(ValueLoc::Reg(to));
            }
            self.reg_to_label.insert(to, label);
        }
    }

    fn step<M: MachInst>(&mut self, inst: &M) {
        for write in get_inst_writes(inst) {
            self.clear_reg(write);
        }
        if let Some((label, reg)) = inst.defines_value_label() {
            self.def_label_at_reg(label, reg);
        }
        match inst.stack_op_info() {
            Some(MachInstStackOpInfo::LoadNomSPOff(reg, offset)) => {
                self.load_reg(reg, offset + self.nominal_sp_offset.unwrap());
            }
            Some(MachInstStackOpInfo::StoreNomSPOff(reg, offset)) => {
                self.store_reg(reg, offset + self.nominal_sp_offset.unwrap());
            }
            Some(MachInstStackOpInfo::NomSPAdj(offset)) => {
                if self.nominal_sp_offset.is_some() {
                    self.nominal_sp_offset = Some(self.nominal_sp_offset.unwrap() + offset);
                }
            }
            _ => {}
        }
        if let Some((to, from)) = inst.is_move() {
            let to = to.to_reg();
            self.move_reg(to, from);
        }
    }
}

trait IntersectFrom {
    fn intersect_from(&mut self, other: &Self) -> IntersectResult;
}
struct IntersectResult {
    changed: bool,
    remove: bool,
}

impl IntersectFrom for AnalysisInfo {
    fn intersect_from(&mut self, other: &Self) -> IntersectResult {
        let mut changed = false;
        changed |= self
            .nominal_sp_offset
            .intersect_from(&other.nominal_sp_offset)
            .changed;
        changed |= self
            .label_to_locs
            .intersect_from(&other.label_to_locs)
            .changed;
        changed |= self
            .reg_to_label
            .intersect_from(&other.reg_to_label)
            .changed;
        changed |= self
            .stack_to_label
            .intersect_from(&other.stack_to_label)
            .changed;
        IntersectResult {
            changed,
            remove: false,
        }
    }
}

impl<K, V> IntersectFrom for HashMap<K, V>
where
    K: Copy + Eq + Hash,
    V: IntersectFrom,
{
    fn intersect_from(&mut self, other: &Self) -> IntersectResult {
        let mut changed = false;
        let mut remove_keys = vec![];
        for k in self.keys() {
            if !other.contains_key(k) {
                remove_keys.push(*k);
            }
        }
        for k in remove_keys {
            changed = true;
            self.remove(&k);
        }

        let mut remove_keys = vec![];
        for k in other.keys() {
            if let Some(v) = self.get_mut(k) {
                let result = v.intersect_from(other.get(k).unwrap());
                changed |= result.changed;
                if result.remove {
                    remove_keys.push(*k);
                }
            }
        }
        for k in remove_keys {
            changed = true;
            self.remove(&k);
        }

        IntersectResult {
            changed,
            remove: self.len() == 0,
        }
    }
}
impl<T> IntersectFrom for HashSet<T>
where
    T: Copy + Eq + Hash,
{
    fn intersect_from(&mut self, other: &Self) -> IntersectResult {
        let mut changed = false;
        let mut remove = vec![];
        for val in self.iter() {
            if !other.contains(val) {
                remove.push(*val);
            }
        }
        for val in remove {
            changed = true;
            self.remove(&val);
        }

        IntersectResult {
            changed,
            remove: self.len() == 0,
        }
    }
}
impl IntersectFrom for ValueLabel {
    fn intersect_from(&mut self, other: &Self) -> IntersectResult {
        // Remove if not equal (simple top -> values -> bottom lattice)
        IntersectResult {
            changed: false,
            remove: *self != *other,
        }
    }
}
impl<T> IntersectFrom for Option<T>
where
    T: Copy + Eq,
{
    fn intersect_from(&mut self, other: &Self) -> IntersectResult {
        let mut changed = false;
        if !(self.is_some() && other.is_some() && self == other) {
            changed = true;
            *self = None;
        }
        IntersectResult {
            changed,
            remove: self.is_none(),
        }
    }
}

pub(crate) fn compute<I: VCodeInst>(
    insts: &[I],
    inst_ends: &[u32],
    label_insn_iix: &[u32],
) -> ValueLabelsRanges {
    let inst_start = |idx: usize| if idx == 0 { 0 } else { inst_ends[idx - 1] };

    trace!("compute: insts =");
    for i in 0..insts.len() {
        trace!(" #{} end: {} -> {:?}", i, inst_ends[i], insts[i]);
    }
    trace!("label_insn_iix: {:?}", label_insn_iix);

    // Info at each block head, indexed by label.
    let mut block_starts: HashMap<u32, AnalysisInfo> = HashMap::new();

    // Initialize state at entry.
    block_starts.insert(0, AnalysisInfo::new());

    // Worklist: label indices for basic blocks.
    let mut worklist = VecDeque::new();
    let mut worklist_set = HashSet::new();
    worklist.push_back(0);
    worklist_set.insert(0);

    while !worklist.is_empty() {
        let block = worklist.pop_front().unwrap();
        worklist_set.remove(&block);

        let mut state = block_starts.get(&block).unwrap().clone();
        trace!("at block {} -> state: {:?}", block, state);
        // Iterate for each instruction in the block (we break at the first
        // terminator we see).
        let mut iix = label_insn_iix[block as usize];
        while iix < insts.len() as u32 {
            state.step(&insts[iix as usize]);
            trace!(" -> inst #{}: {:?}", iix, insts[iix as usize]);
            trace!("    --> state: {:?}", state);

            let term = insts[iix as usize].is_term();
            if term.is_term() {
                for succ in term.get_succs() {
                    trace!("    SUCCESSOR block {}", succ.get());
                    if let Some(succ_state) = block_starts.get_mut(&succ.get()) {
                        trace!("       orig state: {:?}", succ_state);
                        if succ_state.intersect_from(&state).changed {
                            if worklist_set.insert(succ.get()) {
                                worklist.push_back(succ.get());
                            }
                            trace!("        (changed)");
                        }
                        trace!("       new state: {:?}", succ_state);
                    } else {
                        // First time seeing this block
                        block_starts.insert(succ.get(), state.clone());
                        worklist.push_back(succ.get());
                        worklist_set.insert(succ.get());
                    }
                }
                break;
            }

            iix += 1;
        }
    }

    // Now iterate over blocks one last time, collecting
    // value-label locations.

    let mut value_labels_ranges: ValueLabelsRanges = HashMap::new();
    for block in 0..label_insn_iix.len() {
        let start_iix = label_insn_iix[block];
        let end_iix = if block == label_insn_iix.len() - 1 {
            insts.len() as u32
        } else {
            label_insn_iix[block + 1]
        };
        let block = block as u32;
        let mut state = block_starts.get(&block).unwrap().clone();
        for iix in start_iix..end_iix {
            let offset = inst_start(iix as usize);
            let end = inst_ends[iix as usize];
            state.step(&insts[iix as usize]);

            for (label, locs) in &state.label_to_locs {
                trace!("   inst {} has label {:?} -> locs {:?}", iix, label, locs);
                // Find an appropriate loc: a register if possible, otherwise pick the first stack
                // loc.
                let reg = locs.iter().cloned().find(|l| l.is_reg());
                let stack = locs.iter().cloned().find(|l| l.is_stack());
                if let Some(loc) = reg.or(stack) {
                    let loc = LabelValueLoc::from(loc);
                    let list = value_labels_ranges.entry(*label).or_insert_with(|| vec![]);
                    if list.is_empty()
                        || list.last().unwrap().end <= offset
                        || list.last().unwrap().loc != loc
                    {
                        list.push(ValueLocRange {
                            loc,
                            start: end,
                            end: end + 1,
                        });
                    } else {
                        list.last_mut().unwrap().end = end + 1;
                    }
                }
            }
        }
    }

    trace!("ret: {:?}", value_labels_ranges);
    value_labels_ranges
}
