//! Function layout.
//!
//! The order of extended basic blocks in a function and the order of instructions in an EBB is
//! determined by the `Layout` data structure defined in this module.

use crate::entity::SecondaryMap;
use crate::ir::dfg::DataFlowGraph;
use crate::ir::progpoint::{ExpandedProgramPoint, ProgramOrder};
use crate::ir::{Ebb, Inst};
use crate::packed_option::PackedOption;
use crate::timing;
use core::cmp;
use core::iter::{IntoIterator, Iterator};
use log::debug;

/// The `Layout` struct determines the layout of EBBs and instructions in a function. It does not
/// contain definitions of instructions or EBBs, but depends on `Inst` and `Ebb` entity references
/// being defined elsewhere.
///
/// This data structure determines:
///
/// - The order of EBBs in the function.
/// - Which EBB contains a given instruction.
/// - The order of instructions with an EBB.
///
/// While data dependencies are not recorded, instruction ordering does affect control
/// dependencies, so part of the semantics of the program are determined by the layout.
///
#[derive(Clone)]
pub struct Layout {
    /// Linked list nodes for the layout order of EBBs Forms a doubly linked list, terminated in
    /// both ends by `None`.
    ebbs: SecondaryMap<Ebb, EbbNode>,

    /// Linked list nodes for the layout order of instructions. Forms a double linked list per EBB,
    /// terminated in both ends by `None`.
    insts: SecondaryMap<Inst, InstNode>,

    /// First EBB in the layout order, or `None` when no EBBs have been laid out.
    first_ebb: Option<Ebb>,

    /// Last EBB in the layout order, or `None` when no EBBs have been laid out.
    last_ebb: Option<Ebb>,
}

impl Layout {
    /// Create a new empty `Layout`.
    pub fn new() -> Self {
        Self {
            ebbs: SecondaryMap::new(),
            insts: SecondaryMap::new(),
            first_ebb: None,
            last_ebb: None,
        }
    }

    /// Clear the layout.
    pub fn clear(&mut self) {
        self.ebbs.clear();
        self.insts.clear();
        self.first_ebb = None;
        self.last_ebb = None;
    }

    /// Returns the capacity of the `EbbData` map.
    pub fn ebb_capacity(&self) -> usize {
        self.ebbs.capacity()
    }
}

/// Sequence numbers.
///
/// All instructions and EBBs are given a sequence number that can be used to quickly determine
/// their relative position in the layout. The sequence numbers are not contiguous, but are assigned
/// like line numbers in BASIC: 10, 20, 30, ...
///
/// The EBB sequence numbers are strictly increasing, and so are the instruction sequence numbers
/// within an EBB. The instruction sequence numbers are all between the sequence number of their
/// containing EBB and the following EBB.
///
/// The result is that sequence numbers work like BASIC line numbers for the textual form of the IR.
type SequenceNumber = u32;

/// Initial stride assigned to new sequence numbers.
const MAJOR_STRIDE: SequenceNumber = 10;

/// Secondary stride used when renumbering locally.
const MINOR_STRIDE: SequenceNumber = 2;

/// Limit on the sequence number range we'll renumber locally. If this limit is exceeded, we'll
/// switch to a full function renumbering.
const LOCAL_LIMIT: SequenceNumber = 100 * MINOR_STRIDE;

/// Compute the midpoint between `a` and `b`.
/// Return `None` if the midpoint would be equal to either.
fn midpoint(a: SequenceNumber, b: SequenceNumber) -> Option<SequenceNumber> {
    debug_assert!(a < b);
    // Avoid integer overflow.
    let m = a + (b - a) / 2;
    if m > a {
        Some(m)
    } else {
        None
    }
}

#[test]
fn test_midpoint() {
    assert_eq!(midpoint(0, 1), None);
    assert_eq!(midpoint(0, 2), Some(1));
    assert_eq!(midpoint(0, 3), Some(1));
    assert_eq!(midpoint(0, 4), Some(2));
    assert_eq!(midpoint(1, 4), Some(2));
    assert_eq!(midpoint(2, 4), Some(3));
    assert_eq!(midpoint(3, 4), None);
    assert_eq!(midpoint(3, 4), None);
}

impl ProgramOrder for Layout {
    fn cmp<A, B>(&self, a: A, b: B) -> cmp::Ordering
    where
        A: Into<ExpandedProgramPoint>,
        B: Into<ExpandedProgramPoint>,
    {
        let a_seq = self.seq(a);
        let b_seq = self.seq(b);
        a_seq.cmp(&b_seq)
    }

    fn is_ebb_gap(&self, inst: Inst, ebb: Ebb) -> bool {
        let i = &self.insts[inst];
        let e = &self.ebbs[ebb];

        i.next.is_none() && i.ebb == e.prev
    }
}

// Private methods for dealing with sequence numbers.
impl Layout {
    /// Get the sequence number of a program point that must correspond to an entity in the layout.
    fn seq<PP: Into<ExpandedProgramPoint>>(&self, pp: PP) -> SequenceNumber {
        // When `PP = Inst` or `PP = Ebb`, we expect this dynamic type check to be optimized out.
        match pp.into() {
            ExpandedProgramPoint::Ebb(ebb) => self.ebbs[ebb].seq,
            ExpandedProgramPoint::Inst(inst) => self.insts[inst].seq,
        }
    }

    /// Get the last sequence number in `ebb`.
    fn last_ebb_seq(&self, ebb: Ebb) -> SequenceNumber {
        // Get the seq of the last instruction if it exists, otherwise use the EBB header seq.
        self.ebbs[ebb]
            .last_inst
            .map(|inst| self.insts[inst].seq)
            .unwrap_or(self.ebbs[ebb].seq)
    }

    /// Assign a valid sequence number to `ebb` such that the numbers are still monotonic. This may
    /// require renumbering.
    fn assign_ebb_seq(&mut self, ebb: Ebb) {
        debug_assert!(self.is_ebb_inserted(ebb));

        // Get the sequence number immediately before `ebb`, or 0.
        let prev_seq = self.ebbs[ebb]
            .prev
            .map(|prev_ebb| self.last_ebb_seq(prev_ebb))
            .unwrap_or(0);

        // Get the sequence number immediately following `ebb`.
        let next_seq = if let Some(inst) = self.ebbs[ebb].first_inst.expand() {
            self.insts[inst].seq
        } else if let Some(next_ebb) = self.ebbs[ebb].next.expand() {
            self.ebbs[next_ebb].seq
        } else {
            // There is nothing after `ebb`. We can just use a major stride.
            self.ebbs[ebb].seq = prev_seq + MAJOR_STRIDE;
            return;
        };

        // Check if there is room between these sequence numbers.
        if let Some(seq) = midpoint(prev_seq, next_seq) {
            self.ebbs[ebb].seq = seq;
        } else {
            // No available integers between `prev_seq` and `next_seq`. We have to renumber.
            self.renumber_from_ebb(ebb, prev_seq + MINOR_STRIDE, prev_seq + LOCAL_LIMIT);
        }
    }

    /// Assign a valid sequence number to `inst` such that the numbers are still monotonic. This may
    /// require renumbering.
    fn assign_inst_seq(&mut self, inst: Inst) {
        let ebb = self
            .inst_ebb(inst)
            .expect("inst must be inserted before assigning an seq");

        // Get the sequence number immediately before `inst`.
        let prev_seq = match self.insts[inst].prev.expand() {
            Some(prev_inst) => self.insts[prev_inst].seq,
            None => self.ebbs[ebb].seq,
        };

        // Get the sequence number immediately following `inst`.
        let next_seq = if let Some(next_inst) = self.insts[inst].next.expand() {
            self.insts[next_inst].seq
        } else if let Some(next_ebb) = self.ebbs[ebb].next.expand() {
            self.ebbs[next_ebb].seq
        } else {
            // There is nothing after `inst`. We can just use a major stride.
            self.insts[inst].seq = prev_seq + MAJOR_STRIDE;
            return;
        };

        // Check if there is room between these sequence numbers.
        if let Some(seq) = midpoint(prev_seq, next_seq) {
            self.insts[inst].seq = seq;
        } else {
            // No available integers between `prev_seq` and `next_seq`. We have to renumber.
            self.renumber_from_inst(inst, prev_seq + MINOR_STRIDE, prev_seq + LOCAL_LIMIT);
        }
    }

    /// Renumber instructions starting from `inst` until the end of the EBB or until numbers catch
    /// up.
    ///
    /// Return `None` if renumbering has caught up and the sequence is monotonic again. Otherwise
    /// return the last used sequence number.
    ///
    /// If sequence numbers exceed `limit`, switch to a full function renumbering and return `None`.
    fn renumber_insts(
        &mut self,
        inst: Inst,
        seq: SequenceNumber,
        limit: SequenceNumber,
    ) -> Option<SequenceNumber> {
        let mut inst = inst;
        let mut seq = seq;

        loop {
            self.insts[inst].seq = seq;

            // Next instruction.
            inst = match self.insts[inst].next.expand() {
                None => return Some(seq),
                Some(next) => next,
            };

            if seq < self.insts[inst].seq {
                // Sequence caught up.
                return None;
            }

            if seq > limit {
                // We're pushing too many instructions in front of us.
                // Switch to a full function renumbering to make some space.
                self.full_renumber();
                return None;
            }

            seq += MINOR_STRIDE;
        }
    }

    /// Renumber starting from `ebb` to `seq` and continuing until the sequence numbers are
    /// monotonic again.
    fn renumber_from_ebb(&mut self, ebb: Ebb, first_seq: SequenceNumber, limit: SequenceNumber) {
        let mut ebb = ebb;
        let mut seq = first_seq;

        loop {
            self.ebbs[ebb].seq = seq;

            // Renumber instructions in `ebb`. Stop when the numbers catch up.
            if let Some(inst) = self.ebbs[ebb].first_inst.expand() {
                seq = match self.renumber_insts(inst, seq + MINOR_STRIDE, limit) {
                    Some(s) => s,
                    None => return,
                }
            }

            // Advance to the next EBB.
            ebb = match self.ebbs[ebb].next.expand() {
                Some(next) => next,
                None => return,
            };

            // Stop renumbering once the numbers catch up.
            if seq < self.ebbs[ebb].seq {
                return;
            }

            seq += MINOR_STRIDE;
        }
    }

    /// Renumber starting from `inst` to `seq` and continuing until the sequence numbers are
    /// monotonic again.
    fn renumber_from_inst(&mut self, inst: Inst, first_seq: SequenceNumber, limit: SequenceNumber) {
        if let Some(seq) = self.renumber_insts(inst, first_seq, limit) {
            // Renumbering spills over into next EBB.
            if let Some(next_ebb) = self.ebbs[self.inst_ebb(inst).unwrap()].next.expand() {
                self.renumber_from_ebb(next_ebb, seq + MINOR_STRIDE, limit);
            }
        }
    }

    /// Renumber all EBBs and instructions in the layout.
    ///
    /// This doesn't affect the position of anything, but it gives more room in the internal
    /// sequence numbers for inserting instructions later.
    fn full_renumber(&mut self) {
        let _tt = timing::layout_renumber();
        let mut seq = 0;
        let mut next_ebb = self.first_ebb;
        while let Some(ebb) = next_ebb {
            self.ebbs[ebb].seq = seq;
            seq += MAJOR_STRIDE;
            next_ebb = self.ebbs[ebb].next.expand();

            let mut next_inst = self.ebbs[ebb].first_inst.expand();
            while let Some(inst) = next_inst {
                self.insts[inst].seq = seq;
                seq += MAJOR_STRIDE;
                next_inst = self.insts[inst].next.expand();
            }
        }
        debug!("Renumbered {} program points", seq / MAJOR_STRIDE);
    }
}

/// Methods for laying out EBBs.
///
/// An unknown EBB starts out as *not inserted* in the EBB layout. The layout is a linear order of
/// inserted EBBs. Once an EBB has been inserted in the layout, instructions can be added. An EBB
/// can only be removed from the layout when it is empty.
///
/// Since every EBB must end with a terminator instruction which cannot fall through, the layout of
/// EBBs do not affect the semantics of the program.
///
impl Layout {
    /// Is `ebb` currently part of the layout?
    pub fn is_ebb_inserted(&self, ebb: Ebb) -> bool {
        Some(ebb) == self.first_ebb || self.ebbs[ebb].prev.is_some()
    }

    /// Insert `ebb` as the last EBB in the layout.
    pub fn append_ebb(&mut self, ebb: Ebb) {
        debug_assert!(
            !self.is_ebb_inserted(ebb),
            "Cannot append EBB that is already in the layout"
        );
        {
            let node = &mut self.ebbs[ebb];
            debug_assert!(node.first_inst.is_none() && node.last_inst.is_none());
            node.prev = self.last_ebb.into();
            node.next = None.into();
        }
        if let Some(last) = self.last_ebb {
            self.ebbs[last].next = ebb.into();
        } else {
            self.first_ebb = Some(ebb);
        }
        self.last_ebb = Some(ebb);
        self.assign_ebb_seq(ebb);
    }

    /// Insert `ebb` in the layout before the existing EBB `before`.
    pub fn insert_ebb(&mut self, ebb: Ebb, before: Ebb) {
        debug_assert!(
            !self.is_ebb_inserted(ebb),
            "Cannot insert EBB that is already in the layout"
        );
        debug_assert!(
            self.is_ebb_inserted(before),
            "EBB Insertion point not in the layout"
        );
        let after = self.ebbs[before].prev;
        {
            let node = &mut self.ebbs[ebb];
            node.next = before.into();
            node.prev = after;
        }
        self.ebbs[before].prev = ebb.into();
        match after.expand() {
            None => self.first_ebb = Some(ebb),
            Some(a) => self.ebbs[a].next = ebb.into(),
        }
        self.assign_ebb_seq(ebb);
    }

    /// Insert `ebb` in the layout *after* the existing EBB `after`.
    pub fn insert_ebb_after(&mut self, ebb: Ebb, after: Ebb) {
        debug_assert!(
            !self.is_ebb_inserted(ebb),
            "Cannot insert EBB that is already in the layout"
        );
        debug_assert!(
            self.is_ebb_inserted(after),
            "EBB Insertion point not in the layout"
        );
        let before = self.ebbs[after].next;
        {
            let node = &mut self.ebbs[ebb];
            node.next = before;
            node.prev = after.into();
        }
        self.ebbs[after].next = ebb.into();
        match before.expand() {
            None => self.last_ebb = Some(ebb),
            Some(b) => self.ebbs[b].prev = ebb.into(),
        }
        self.assign_ebb_seq(ebb);
    }

    /// Remove `ebb` from the layout.
    pub fn remove_ebb(&mut self, ebb: Ebb) {
        debug_assert!(self.is_ebb_inserted(ebb), "EBB not in the layout");
        debug_assert!(self.first_inst(ebb).is_none(), "EBB must be empty.");

        // Clear the `ebb` node and extract links.
        let prev;
        let next;
        {
            let n = &mut self.ebbs[ebb];
            prev = n.prev;
            next = n.next;
            n.prev = None.into();
            n.next = None.into();
        }
        // Fix up links to `ebb`.
        match prev.expand() {
            None => self.first_ebb = next.expand(),
            Some(p) => self.ebbs[p].next = next,
        }
        match next.expand() {
            None => self.last_ebb = prev.expand(),
            Some(n) => self.ebbs[n].prev = prev,
        }
    }

    /// Return an iterator over all EBBs in layout order.
    pub fn ebbs(&self) -> Ebbs {
        Ebbs {
            layout: self,
            next: self.first_ebb,
        }
    }

    /// Get the function's entry block.
    /// This is simply the first EBB in the layout order.
    pub fn entry_block(&self) -> Option<Ebb> {
        self.first_ebb
    }

    /// Get the last EBB in the layout.
    pub fn last_ebb(&self) -> Option<Ebb> {
        self.last_ebb
    }

    /// Get the block preceding `ebb` in the layout order.
    pub fn prev_ebb(&self, ebb: Ebb) -> Option<Ebb> {
        self.ebbs[ebb].prev.expand()
    }

    /// Get the block following `ebb` in the layout order.
    pub fn next_ebb(&self, ebb: Ebb) -> Option<Ebb> {
        self.ebbs[ebb].next.expand()
    }
}

#[derive(Clone, Debug, Default)]
struct EbbNode {
    prev: PackedOption<Ebb>,
    next: PackedOption<Ebb>,
    first_inst: PackedOption<Inst>,
    last_inst: PackedOption<Inst>,
    seq: SequenceNumber,
}

/// Iterate over EBBs in layout order. See `Layout::ebbs()`.
pub struct Ebbs<'f> {
    layout: &'f Layout,
    next: Option<Ebb>,
}

impl<'f> Iterator for Ebbs<'f> {
    type Item = Ebb;

    fn next(&mut self) -> Option<Ebb> {
        match self.next {
            Some(ebb) => {
                self.next = self.layout.next_ebb(ebb);
                Some(ebb)
            }
            None => None,
        }
    }
}

/// Use a layout reference in a for loop.
impl<'f> IntoIterator for &'f Layout {
    type Item = Ebb;
    type IntoIter = Ebbs<'f>;

    fn into_iter(self) -> Ebbs<'f> {
        self.ebbs()
    }
}

/// Methods for arranging instructions.
///
/// An instruction starts out as *not inserted* in the layout. An instruction can be inserted into
/// an EBB at a given position.
impl Layout {
    /// Get the EBB containing `inst`, or `None` if `inst` is not inserted in the layout.
    pub fn inst_ebb(&self, inst: Inst) -> Option<Ebb> {
        self.insts[inst].ebb.into()
    }

    /// Get the EBB containing the program point `pp`. Panic if `pp` is not in the layout.
    pub fn pp_ebb<PP>(&self, pp: PP) -> Ebb
    where
        PP: Into<ExpandedProgramPoint>,
    {
        match pp.into() {
            ExpandedProgramPoint::Ebb(ebb) => ebb,
            ExpandedProgramPoint::Inst(inst) => {
                self.inst_ebb(inst).expect("Program point not in layout")
            }
        }
    }

    /// Append `inst` to the end of `ebb`.
    pub fn append_inst(&mut self, inst: Inst, ebb: Ebb) {
        debug_assert_eq!(self.inst_ebb(inst), None);
        debug_assert!(
            self.is_ebb_inserted(ebb),
            "Cannot append instructions to EBB not in layout"
        );
        {
            let ebb_node = &mut self.ebbs[ebb];
            {
                let inst_node = &mut self.insts[inst];
                inst_node.ebb = ebb.into();
                inst_node.prev = ebb_node.last_inst;
                debug_assert!(inst_node.next.is_none());
            }
            if ebb_node.first_inst.is_none() {
                ebb_node.first_inst = inst.into();
            } else {
                self.insts[ebb_node.last_inst.unwrap()].next = inst.into();
            }
            ebb_node.last_inst = inst.into();
        }
        self.assign_inst_seq(inst);
    }

    /// Fetch an ebb's first instruction.
    pub fn first_inst(&self, ebb: Ebb) -> Option<Inst> {
        self.ebbs[ebb].first_inst.into()
    }

    /// Fetch an ebb's last instruction.
    pub fn last_inst(&self, ebb: Ebb) -> Option<Inst> {
        self.ebbs[ebb].last_inst.into()
    }

    /// Fetch the instruction following `inst`.
    pub fn next_inst(&self, inst: Inst) -> Option<Inst> {
        self.insts[inst].next.expand()
    }

    /// Fetch the instruction preceding `inst`.
    pub fn prev_inst(&self, inst: Inst) -> Option<Inst> {
        self.insts[inst].prev.expand()
    }

    /// Fetch the first instruction in an ebb's terminal branch group.
    pub fn canonical_branch_inst(&self, dfg: &DataFlowGraph, ebb: Ebb) -> Option<Inst> {
        // Basic blocks permit at most two terminal branch instructions.
        // If two, the former is conditional and the latter is unconditional.
        let last = self.last_inst(ebb)?;
        if let Some(prev) = self.prev_inst(last) {
            if dfg[prev].opcode().is_branch() {
                return Some(prev);
            }
        }
        Some(last)
    }

    /// Insert `inst` before the instruction `before` in the same EBB.
    pub fn insert_inst(&mut self, inst: Inst, before: Inst) {
        debug_assert_eq!(self.inst_ebb(inst), None);
        let ebb = self
            .inst_ebb(before)
            .expect("Instruction before insertion point not in the layout");
        let after = self.insts[before].prev;
        {
            let inst_node = &mut self.insts[inst];
            inst_node.ebb = ebb.into();
            inst_node.next = before.into();
            inst_node.prev = after;
        }
        self.insts[before].prev = inst.into();
        match after.expand() {
            None => self.ebbs[ebb].first_inst = inst.into(),
            Some(a) => self.insts[a].next = inst.into(),
        }
        self.assign_inst_seq(inst);
    }

    /// Remove `inst` from the layout.
    pub fn remove_inst(&mut self, inst: Inst) {
        let ebb = self.inst_ebb(inst).expect("Instruction already removed.");
        // Clear the `inst` node and extract links.
        let prev;
        let next;
        {
            let n = &mut self.insts[inst];
            prev = n.prev;
            next = n.next;
            n.ebb = None.into();
            n.prev = None.into();
            n.next = None.into();
        }
        // Fix up links to `inst`.
        match prev.expand() {
            None => self.ebbs[ebb].first_inst = next,
            Some(p) => self.insts[p].next = next,
        }
        match next.expand() {
            None => self.ebbs[ebb].last_inst = prev,
            Some(n) => self.insts[n].prev = prev,
        }
    }

    /// Iterate over the instructions in `ebb` in layout order.
    pub fn ebb_insts(&self, ebb: Ebb) -> Insts {
        Insts {
            layout: self,
            head: self.ebbs[ebb].first_inst.into(),
            tail: self.ebbs[ebb].last_inst.into(),
        }
    }

    /// Split the EBB containing `before` in two.
    ///
    /// Insert `new_ebb` after the old EBB and move `before` and the following instructions to
    /// `new_ebb`:
    ///
    /// ```text
    /// old_ebb:
    ///     i1
    ///     i2
    ///     i3 << before
    ///     i4
    /// ```
    /// becomes:
    ///
    /// ```text
    /// old_ebb:
    ///     i1
    ///     i2
    /// new_ebb:
    ///     i3 << before
    ///     i4
    /// ```
    pub fn split_ebb(&mut self, new_ebb: Ebb, before: Inst) {
        let old_ebb = self
            .inst_ebb(before)
            .expect("The `before` instruction must be in the layout");
        debug_assert!(!self.is_ebb_inserted(new_ebb));

        // Insert new_ebb after old_ebb.
        let next_ebb = self.ebbs[old_ebb].next;
        let last_inst = self.ebbs[old_ebb].last_inst;
        {
            let node = &mut self.ebbs[new_ebb];
            node.prev = old_ebb.into();
            node.next = next_ebb;
            node.first_inst = before.into();
            node.last_inst = last_inst;
        }
        self.ebbs[old_ebb].next = new_ebb.into();

        // Fix backwards link.
        if Some(old_ebb) == self.last_ebb {
            self.last_ebb = Some(new_ebb);
        } else {
            self.ebbs[next_ebb.unwrap()].prev = new_ebb.into();
        }

        // Disconnect the instruction links.
        let prev_inst = self.insts[before].prev;
        self.insts[before].prev = None.into();
        self.ebbs[old_ebb].last_inst = prev_inst;
        match prev_inst.expand() {
            None => self.ebbs[old_ebb].first_inst = None.into(),
            Some(pi) => self.insts[pi].next = None.into(),
        }

        // Fix the instruction -> ebb pointers.
        let mut opt_i = Some(before);
        while let Some(i) = opt_i {
            debug_assert_eq!(self.insts[i].ebb.expand(), Some(old_ebb));
            self.insts[i].ebb = new_ebb.into();
            opt_i = self.insts[i].next.into();
        }

        self.assign_ebb_seq(new_ebb);
    }
}

#[derive(Clone, Debug, Default)]
struct InstNode {
    /// The Ebb containing this instruction, or `None` if the instruction is not yet inserted.
    ebb: PackedOption<Ebb>,
    prev: PackedOption<Inst>,
    next: PackedOption<Inst>,
    seq: SequenceNumber,
}

/// Iterate over instructions in an EBB in layout order. See `Layout::ebb_insts()`.
pub struct Insts<'f> {
    layout: &'f Layout,
    head: Option<Inst>,
    tail: Option<Inst>,
}

impl<'f> Iterator for Insts<'f> {
    type Item = Inst;

    fn next(&mut self) -> Option<Inst> {
        let rval = self.head;
        if let Some(inst) = rval {
            if self.head == self.tail {
                self.head = None;
                self.tail = None;
            } else {
                self.head = self.layout.insts[inst].next.into();
            }
        }
        rval
    }
}

impl<'f> DoubleEndedIterator for Insts<'f> {
    fn next_back(&mut self) -> Option<Inst> {
        let rval = self.tail;
        if let Some(inst) = rval {
            if self.head == self.tail {
                self.head = None;
                self.tail = None;
            } else {
                self.tail = self.layout.insts[inst].prev.into();
            }
        }
        rval
    }
}

#[cfg(test)]
mod tests {
    use super::Layout;
    use crate::cursor::{Cursor, CursorPosition};
    use crate::entity::EntityRef;
    use crate::ir::{Ebb, Inst, ProgramOrder, SourceLoc};
    use alloc::vec::Vec;
    use core::cmp::Ordering;

    struct LayoutCursor<'f> {
        /// Borrowed function layout. Public so it can be re-borrowed from this cursor.
        pub layout: &'f mut Layout,
        pos: CursorPosition,
    }

    impl<'f> Cursor for LayoutCursor<'f> {
        fn position(&self) -> CursorPosition {
            self.pos
        }

        fn set_position(&mut self, pos: CursorPosition) {
            self.pos = pos;
        }

        fn srcloc(&self) -> SourceLoc {
            unimplemented!()
        }

        fn set_srcloc(&mut self, _srcloc: SourceLoc) {
            unimplemented!()
        }

        fn layout(&self) -> &Layout {
            self.layout
        }

        fn layout_mut(&mut self) -> &mut Layout {
            self.layout
        }
    }

    impl<'f> LayoutCursor<'f> {
        /// Create a new `LayoutCursor` for `layout`.
        /// The cursor holds a mutable reference to `layout` for its entire lifetime.
        pub fn new(layout: &'f mut Layout) -> Self {
            Self {
                layout,
                pos: CursorPosition::Nowhere,
            }
        }
    }

    fn verify(layout: &mut Layout, ebbs: &[(Ebb, &[Inst])]) {
        // Check that EBBs are inserted and instructions belong the right places.
        // Check forward linkage with iterators.
        // Check that layout sequence numbers are strictly monotonic.
        {
            let mut seq = 0;
            let mut ebb_iter = layout.ebbs();
            for &(ebb, insts) in ebbs {
                assert!(layout.is_ebb_inserted(ebb));
                assert_eq!(ebb_iter.next(), Some(ebb));
                assert!(layout.ebbs[ebb].seq > seq);
                seq = layout.ebbs[ebb].seq;

                let mut inst_iter = layout.ebb_insts(ebb);
                for &inst in insts {
                    assert_eq!(layout.inst_ebb(inst), Some(ebb));
                    assert_eq!(inst_iter.next(), Some(inst));
                    assert!(layout.insts[inst].seq > seq);
                    seq = layout.insts[inst].seq;
                }
                assert_eq!(inst_iter.next(), None);
            }
            assert_eq!(ebb_iter.next(), None);
        }

        // Check backwards linkage with a cursor.
        let mut cur = LayoutCursor::new(layout);
        for &(ebb, insts) in ebbs.into_iter().rev() {
            assert_eq!(cur.prev_ebb(), Some(ebb));
            for &inst in insts.into_iter().rev() {
                assert_eq!(cur.prev_inst(), Some(inst));
            }
            assert_eq!(cur.prev_inst(), None);
        }
        assert_eq!(cur.prev_ebb(), None);
    }

    #[test]
    fn append_ebb() {
        let mut layout = Layout::new();
        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);
        let e2 = Ebb::new(2);

        {
            let imm = &layout;
            assert!(!imm.is_ebb_inserted(e0));
            assert!(!imm.is_ebb_inserted(e1));
        }
        verify(&mut layout, &[]);

        layout.append_ebb(e1);
        assert!(!layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(!layout.is_ebb_inserted(e2));
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e1]);

        layout.append_ebb(e2);
        assert!(!layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e1, e2]);

        layout.append_ebb(e0);
        assert!(layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e1, e2, e0]);

        {
            let imm = &layout;
            let mut v = Vec::new();
            for e in imm {
                v.push(e);
            }
            assert_eq!(v, [e1, e2, e0]);
        }

        // Test cursor positioning.
        let mut cur = LayoutCursor::new(&mut layout);
        assert_eq!(cur.position(), CursorPosition::Nowhere);
        assert_eq!(cur.next_inst(), None);
        assert_eq!(cur.position(), CursorPosition::Nowhere);
        assert_eq!(cur.prev_inst(), None);
        assert_eq!(cur.position(), CursorPosition::Nowhere);

        assert_eq!(cur.next_ebb(), Some(e1));
        assert_eq!(cur.position(), CursorPosition::Before(e1));
        assert_eq!(cur.next_inst(), None);
        assert_eq!(cur.position(), CursorPosition::After(e1));
        assert_eq!(cur.next_inst(), None);
        assert_eq!(cur.position(), CursorPosition::After(e1));
        assert_eq!(cur.next_ebb(), Some(e2));
        assert_eq!(cur.prev_inst(), None);
        assert_eq!(cur.position(), CursorPosition::Before(e2));
        assert_eq!(cur.next_ebb(), Some(e0));
        assert_eq!(cur.next_ebb(), None);
        assert_eq!(cur.position(), CursorPosition::Nowhere);

        // Backwards through the EBBs.
        assert_eq!(cur.prev_ebb(), Some(e0));
        assert_eq!(cur.position(), CursorPosition::After(e0));
        assert_eq!(cur.prev_ebb(), Some(e2));
        assert_eq!(cur.prev_ebb(), Some(e1));
        assert_eq!(cur.prev_ebb(), None);
        assert_eq!(cur.position(), CursorPosition::Nowhere);
    }

    #[test]
    fn insert_ebb() {
        let mut layout = Layout::new();
        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);
        let e2 = Ebb::new(2);

        {
            let imm = &layout;
            assert!(!imm.is_ebb_inserted(e0));
            assert!(!imm.is_ebb_inserted(e1));

            let v: Vec<Ebb> = layout.ebbs().collect();
            assert_eq!(v, []);
        }

        layout.append_ebb(e1);
        assert!(!layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(!layout.is_ebb_inserted(e2));
        verify(&mut layout, &[(e1, &[])]);

        layout.insert_ebb(e2, e1);
        assert!(!layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        verify(&mut layout, &[(e2, &[]), (e1, &[])]);

        layout.insert_ebb(e0, e1);
        assert!(layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        verify(&mut layout, &[(e2, &[]), (e0, &[]), (e1, &[])]);
    }

    #[test]
    fn insert_ebb_after() {
        let mut layout = Layout::new();
        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);
        let e2 = Ebb::new(2);

        layout.append_ebb(e1);
        layout.insert_ebb_after(e2, e1);
        verify(&mut layout, &[(e1, &[]), (e2, &[])]);

        layout.insert_ebb_after(e0, e1);
        verify(&mut layout, &[(e1, &[]), (e0, &[]), (e2, &[])]);
    }

    #[test]
    fn append_inst() {
        let mut layout = Layout::new();
        let e1 = Ebb::new(1);

        layout.append_ebb(e1);
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, []);

        let i0 = Inst::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);

        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), None);
        assert_eq!(layout.inst_ebb(i2), None);

        layout.append_inst(i1, e1);
        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), None);
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i1]);

        layout.append_inst(i2, e1);
        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), Some(e1));
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i1, i2]);

        // Test double-ended instruction iterator.
        let v: Vec<Inst> = layout.ebb_insts(e1).rev().collect();
        assert_eq!(v, [i2, i1]);

        layout.append_inst(i0, e1);
        verify(&mut layout, &[(e1, &[i1, i2, i0])]);

        // Test cursor positioning.
        let mut cur = LayoutCursor::new(&mut layout).at_top(e1);
        assert_eq!(cur.position(), CursorPosition::Before(e1));
        assert_eq!(cur.prev_inst(), None);
        assert_eq!(cur.position(), CursorPosition::Before(e1));
        assert_eq!(cur.next_inst(), Some(i1));
        assert_eq!(cur.position(), CursorPosition::At(i1));
        assert_eq!(cur.next_inst(), Some(i2));
        assert_eq!(cur.next_inst(), Some(i0));
        assert_eq!(cur.prev_inst(), Some(i2));
        assert_eq!(cur.position(), CursorPosition::At(i2));
        assert_eq!(cur.next_inst(), Some(i0));
        assert_eq!(cur.position(), CursorPosition::At(i0));
        assert_eq!(cur.next_inst(), None);
        assert_eq!(cur.position(), CursorPosition::After(e1));
        assert_eq!(cur.next_inst(), None);
        assert_eq!(cur.position(), CursorPosition::After(e1));
        assert_eq!(cur.prev_inst(), Some(i0));
        assert_eq!(cur.prev_inst(), Some(i2));
        assert_eq!(cur.prev_inst(), Some(i1));
        assert_eq!(cur.prev_inst(), None);
        assert_eq!(cur.position(), CursorPosition::Before(e1));

        // Test remove_inst.
        cur.goto_inst(i2);
        assert_eq!(cur.remove_inst(), i2);
        verify(cur.layout, &[(e1, &[i1, i0])]);
        assert_eq!(cur.layout.inst_ebb(i2), None);
        assert_eq!(cur.remove_inst(), i0);
        verify(cur.layout, &[(e1, &[i1])]);
        assert_eq!(cur.layout.inst_ebb(i0), None);
        assert_eq!(cur.position(), CursorPosition::After(e1));
        cur.layout.remove_inst(i1);
        verify(cur.layout, &[(e1, &[])]);
        assert_eq!(cur.layout.inst_ebb(i1), None);
    }

    #[test]
    fn insert_inst() {
        let mut layout = Layout::new();
        let e1 = Ebb::new(1);

        layout.append_ebb(e1);
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, []);

        let i0 = Inst::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);

        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), None);
        assert_eq!(layout.inst_ebb(i2), None);

        layout.append_inst(i1, e1);
        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), None);
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i1]);

        layout.insert_inst(i2, i1);
        assert_eq!(layout.inst_ebb(i0), None);
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), Some(e1));
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i2, i1]);

        layout.insert_inst(i0, i1);
        verify(&mut layout, &[(e1, &[i2, i0, i1])]);
    }

    #[test]
    fn multiple_ebbs() {
        let mut layout = Layout::new();

        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);

        assert_eq!(layout.entry_block(), None);
        layout.append_ebb(e0);
        assert_eq!(layout.entry_block(), Some(e0));
        layout.append_ebb(e1);
        assert_eq!(layout.entry_block(), Some(e0));

        let i0 = Inst::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);
        let i3 = Inst::new(3);

        layout.append_inst(i0, e0);
        layout.append_inst(i1, e0);
        layout.append_inst(i2, e1);
        layout.append_inst(i3, e1);

        let v0: Vec<Inst> = layout.ebb_insts(e0).collect();
        let v1: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v0, [i0, i1]);
        assert_eq!(v1, [i2, i3]);
    }

    #[test]
    fn split_ebb() {
        let mut layout = Layout::new();

        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);
        let e2 = Ebb::new(2);

        let i0 = Inst::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);
        let i3 = Inst::new(3);

        layout.append_ebb(e0);
        layout.append_inst(i0, e0);
        assert_eq!(layout.inst_ebb(i0), Some(e0));
        layout.split_ebb(e1, i0);
        assert_eq!(layout.inst_ebb(i0), Some(e1));

        {
            let mut cur = LayoutCursor::new(&mut layout);
            assert_eq!(cur.next_ebb(), Some(e0));
            assert_eq!(cur.next_inst(), None);
            assert_eq!(cur.next_ebb(), Some(e1));
            assert_eq!(cur.next_inst(), Some(i0));
            assert_eq!(cur.next_inst(), None);
            assert_eq!(cur.next_ebb(), None);

            // Check backwards links.
            assert_eq!(cur.prev_ebb(), Some(e1));
            assert_eq!(cur.prev_inst(), Some(i0));
            assert_eq!(cur.prev_inst(), None);
            assert_eq!(cur.prev_ebb(), Some(e0));
            assert_eq!(cur.prev_inst(), None);
            assert_eq!(cur.prev_ebb(), None);
        }

        layout.append_inst(i1, e0);
        layout.append_inst(i2, e0);
        layout.append_inst(i3, e0);
        layout.split_ebb(e2, i2);

        assert_eq!(layout.inst_ebb(i0), Some(e1));
        assert_eq!(layout.inst_ebb(i1), Some(e0));
        assert_eq!(layout.inst_ebb(i2), Some(e2));
        assert_eq!(layout.inst_ebb(i3), Some(e2));

        {
            let mut cur = LayoutCursor::new(&mut layout);
            assert_eq!(cur.next_ebb(), Some(e0));
            assert_eq!(cur.next_inst(), Some(i1));
            assert_eq!(cur.next_inst(), None);
            assert_eq!(cur.next_ebb(), Some(e2));
            assert_eq!(cur.next_inst(), Some(i2));
            assert_eq!(cur.next_inst(), Some(i3));
            assert_eq!(cur.next_inst(), None);
            assert_eq!(cur.next_ebb(), Some(e1));
            assert_eq!(cur.next_inst(), Some(i0));
            assert_eq!(cur.next_inst(), None);
            assert_eq!(cur.next_ebb(), None);

            assert_eq!(cur.prev_ebb(), Some(e1));
            assert_eq!(cur.prev_inst(), Some(i0));
            assert_eq!(cur.prev_inst(), None);
            assert_eq!(cur.prev_ebb(), Some(e2));
            assert_eq!(cur.prev_inst(), Some(i3));
            assert_eq!(cur.prev_inst(), Some(i2));
            assert_eq!(cur.prev_inst(), None);
            assert_eq!(cur.prev_ebb(), Some(e0));
            assert_eq!(cur.prev_inst(), Some(i1));
            assert_eq!(cur.prev_inst(), None);
            assert_eq!(cur.prev_ebb(), None);
        }

        // Check `ProgramOrder`.
        assert_eq!(layout.cmp(e2, e2), Ordering::Equal);
        assert_eq!(layout.cmp(e2, i2), Ordering::Less);
        assert_eq!(layout.cmp(i3, i2), Ordering::Greater);

        assert_eq!(layout.is_ebb_gap(i1, e2), true);
        assert_eq!(layout.is_ebb_gap(i3, e1), true);
        assert_eq!(layout.is_ebb_gap(i1, e1), false);
        assert_eq!(layout.is_ebb_gap(i2, e1), false);
    }
}
