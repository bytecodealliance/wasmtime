//! Function layout.
//!
//! The order of extended basic blocks in a function and the order of instructions in an EBB is
//! determined by the `Layout` data structure defined in this module.

use std::iter::{Iterator, IntoIterator};
use entity_map::{EntityMap, EntityRef};
use ir::entities::{Ebb, NO_EBB, Inst, NO_INST};

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
    // Linked list nodes for the layout order of EBBs Forms a doubly linked list, terminated in
    // both ends by NO_EBB.
    ebbs: EntityMap<Ebb, EbbNode>,

    // Linked list nodes for the layout order of instructions. Forms a double linked list per EBB,
    // terminated in both ends by NO_INST.
    insts: EntityMap<Inst, InstNode>,

    // First EBB in the layout order, or `None` when no EBBs have been laid out.
    first_ebb: Option<Ebb>,

    // Last EBB in the layout order, or `None` when no EBBs have been laid out.
    last_ebb: Option<Ebb>,
}

impl Layout {
    /// Create a new empty `Layout`.
    pub fn new() -> Layout {
        Layout {
            ebbs: EntityMap::new(),
            insts: EntityMap::new(),
            first_ebb: None,
            last_ebb: None,
        }
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
        Some(ebb) == self.first_ebb || (self.ebbs.is_valid(ebb) && self.ebbs[ebb].prev != NO_EBB)
    }

    /// Insert `ebb` as the last EBB in the layout.
    pub fn append_ebb(&mut self, ebb: Ebb) {
        assert!(!self.is_ebb_inserted(ebb),
                "Cannot append EBB that is already in the layout");
        {
            let node = self.ebbs.ensure(ebb);
            assert!(node.first_inst == NO_INST && node.last_inst == NO_INST);
            node.prev = self.last_ebb.unwrap_or_default();
            node.next = NO_EBB;
        }
        if let Some(last) = self.last_ebb {
            self.ebbs[last].next = ebb;
        } else {
            self.first_ebb = Some(ebb);
        }
        self.last_ebb = Some(ebb);
    }

    /// Insert `ebb` in the layout before the existing EBB `before`.
    pub fn insert_ebb(&mut self, ebb: Ebb, before: Ebb) {
        assert!(!self.is_ebb_inserted(ebb),
                "Cannot insert EBB that is already in the layout");
        assert!(self.is_ebb_inserted(before),
                "EBB Insertion point not in the layout");
        let after = self.ebbs[before].prev;
        {
            let node = self.ebbs.ensure(ebb);
            node.next = before;
            node.prev = after;
        }
        self.ebbs[before].prev = ebb;
        if after == NO_EBB {
            self.first_ebb = Some(ebb);
        } else {
            self.ebbs[after].next = ebb;
        }
    }

    /// Insert `ebb` in the layout *after* the existing EBB `after`.
    pub fn insert_ebb_after(&mut self, ebb: Ebb, after: Ebb) {
        assert!(!self.is_ebb_inserted(ebb),
                "Cannot insert EBB that is already in the layout");
        assert!(self.is_ebb_inserted(after),
                "EBB Insertion point not in the layout");
        let before = self.ebbs[after].next;
        {
            let node = self.ebbs.ensure(ebb);
            node.next = before;
            node.prev = after;
        }
        self.ebbs[after].next = ebb;
        if before == NO_EBB {
            self.last_ebb = Some(ebb);
        } else {
            self.ebbs[before].prev = ebb;
        }
    }

    /// Return an iterator over all EBBs in layout order.
    pub fn ebbs<'a>(&'a self) -> Ebbs<'a> {
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
}

#[derive(Clone, Debug, Default)]
struct EbbNode {
    prev: Ebb,
    next: Ebb,
    first_inst: Inst,
    last_inst: Inst,
}

/// Iterate over EBBs in layout order. See `Layout::ebbs()`.
pub struct Ebbs<'a> {
    layout: &'a Layout,
    next: Option<Ebb>,
}

impl<'a> Iterator for Ebbs<'a> {
    type Item = Ebb;

    fn next(&mut self) -> Option<Ebb> {
        match self.next {
            Some(ebb) => {
                self.next = self.layout.ebbs[ebb].next.wrap();
                Some(ebb)
            }
            None => None,
        }
    }
}

/// Use a layout reference in a for loop.
impl<'a> IntoIterator for &'a Layout {
    type Item = Ebb;
    type IntoIter = Ebbs<'a>;

    fn into_iter(self) -> Ebbs<'a> {
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
        if self.insts.is_valid(inst) {
            self.insts[inst].ebb.wrap()
        } else {
            None
        }
    }

    /// Append `inst` to the end of `ebb`.
    pub fn append_inst(&mut self, inst: Inst, ebb: Ebb) {
        assert_eq!(self.inst_ebb(inst), None);
        assert!(self.is_ebb_inserted(ebb),
                "Cannot append instructions to EBB not in layout");
        let ebb_node = &mut self.ebbs[ebb];
        {
            let inst_node = self.insts.ensure(inst);
            inst_node.ebb = ebb;
            inst_node.prev = ebb_node.last_inst;
            assert_eq!(inst_node.next, NO_INST);
        }
        if ebb_node.first_inst == NO_INST {
            ebb_node.first_inst = inst;
        } else {
            self.insts[ebb_node.last_inst].next = inst;
        }
        ebb_node.last_inst = inst;
    }

    /// Fetch an ebb's last instruction.
    pub fn last_inst(&self, ebb: Ebb) -> Inst {
        self.ebbs[ebb].last_inst
    }

    /// Insert `inst` before the instruction `before` in the same EBB.
    pub fn insert_inst(&mut self, inst: Inst, before: Inst) {
        assert_eq!(self.inst_ebb(inst), None);
        let ebb = self.inst_ebb(before)
            .expect("Instruction before insertion point not in the layout");
        let after = self.insts[before].prev;
        {
            let inst_node = self.insts.ensure(inst);
            inst_node.ebb = ebb;
            inst_node.next = before;
            inst_node.prev = after;
        }
        self.insts[before].prev = inst;
        if after == NO_INST {
            self.ebbs[ebb].first_inst = inst;
        } else {
            self.insts[after].next = inst;
        }
    }

    /// Iterate over the instructions in `ebb` in layout order.
    pub fn ebb_insts<'a>(&'a self, ebb: Ebb) -> Insts<'a> {
        Insts {
            layout: self,
            next: self.ebbs[ebb].first_inst.wrap(),
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
        let old_ebb = self.inst_ebb(before)
            .expect("The `before` instruction must be in the layout");
        assert!(!self.is_ebb_inserted(new_ebb));

        // Insert new_ebb after old_ebb.
        let next_ebb = self.ebbs[old_ebb].next;
        let last_inst = self.ebbs[old_ebb].last_inst;
        {
            let node = self.ebbs.ensure(new_ebb);
            node.prev = old_ebb;
            node.next = next_ebb;
            node.first_inst = before;
            node.last_inst = last_inst;
        }
        self.ebbs[old_ebb].next = new_ebb;

        // Fix backwards link.
        if Some(old_ebb) == self.last_ebb {
            self.last_ebb = Some(new_ebb);
        } else {
            self.ebbs[next_ebb].prev = new_ebb;
        }

        // Disconnect the instruction links.
        let prev_inst = self.insts[before].prev;
        self.insts[before].prev = NO_INST;
        self.ebbs[old_ebb].last_inst = prev_inst;
        if prev_inst == NO_INST {
            self.ebbs[old_ebb].first_inst = NO_INST;
        } else {
            self.insts[prev_inst].next = NO_INST;
        }

        // Fix the instruction -> ebb pointers.
        let mut i = before;
        while i != NO_INST {
            debug_assert_eq!(self.insts[i].ebb, old_ebb);
            self.insts[i].ebb = new_ebb;
            i = self.insts[i].next;
        }
    }
}

#[derive(Clone, Debug, Default)]
struct InstNode {
    ebb: Ebb,
    prev: Inst,
    next: Inst,
}

/// Iterate over instructions in an EBB in layout order. See `Layout::ebb_insts()`.
pub struct Insts<'a> {
    layout: &'a Layout,
    next: Option<Inst>,
}

impl<'a> Iterator for Insts<'a> {
    type Item = Inst;

    fn next(&mut self) -> Option<Inst> {
        match self.next {
            Some(inst) => {
                self.next = self.layout.insts[inst].next.wrap();
                Some(inst)
            }
            None => None,
        }
    }
}


/// Layout Cursor.
///
/// A `Cursor` represents a position in a function layout where instructions can be inserted and
/// removed. It can be used to iterate through the instructions of a function while editing them at
/// the same time. A normal instruction iterator can't do this since it holds an immutable refernce
/// to the Layout.
///
/// When new instructions are added, the cursor can either apend them to an EBB or insert them
/// before the current instruction.
pub struct Cursor<'a> {
    layout: &'a mut Layout,
    pos: CursorPosition,
}

/// The possible positions of a cursor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorPosition {
    /// Cursor is not pointing anywhere. No instructions can be inserted.
    Nowhere,
    /// Cursor is pointing at an existing instruction.
    /// New instructions will be inserted *before* the current instruction.
    At(Inst),
    /// Cursor is before the beginning of an EBB. No instructions can be inserted. Calling
    /// `next_inst()` wil move to the first instruction in the EBB.
    Before(Ebb),
    /// Cursor is pointing after the end of an EBB.
    /// New instructions will be appended to the EBB.
    After(Ebb),
}

impl<'a> Cursor<'a> {
    /// Create a new `Cursor` for `layout`.
    /// The cursor holds a mutable reference to `layout` for its entire lifetime.
    pub fn new(layout: &'a mut Layout) -> Cursor {
        Cursor {
            layout: layout,
            pos: CursorPosition::Nowhere,
        }
    }

    /// Get the current position.
    pub fn position(&self) -> CursorPosition {
        self.pos
    }

    /// Get the EBB corresponding to the current position.
    pub fn current_ebb(&self) -> Option<Ebb> {
        use self::CursorPosition::*;
        match self.pos {
            Nowhere => None,
            At(inst) => self.layout.inst_ebb(inst),
            Before(ebb) | After(ebb) => Some(ebb),
        }
    }

    /// Go to a specific instruction which must be inserted in the layout.
    /// New instructions will be inserted before `inst`.
    pub fn goto_inst(&mut self, inst: Inst) {
        assert!(self.layout.inst_ebb(inst).is_some());
        self.pos = CursorPosition::At(inst);
    }

    /// Go to the top of `ebb` which must be inserted into the layout.
    /// At this position, instructions cannot be inserted, but `next_inst()` will move to the first
    /// instruction in `ebb`.
    pub fn goto_top(&mut self, ebb: Ebb) {
        assert!(self.layout.is_ebb_inserted(ebb));
        self.pos = CursorPosition::Before(ebb);
    }

    /// Go to the bottom of `ebb` which must be inserted into the layout.
    /// At this position, inserted instructions will be appended to `ebb`.
    pub fn goto_bottom(&mut self, ebb: Ebb) {
        assert!(self.layout.is_ebb_inserted(ebb));
        self.pos = CursorPosition::After(ebb);
    }

    /// Go to the top of the next EBB in layout order and return it.
    ///
    /// - If the cursor wasn't pointing at anything, go to the top of the first EBB in the
    ///   function.
    /// - If there are no more EBBs, leave the cursor pointing at nothing and return `None`.
    ///
    /// # Examples
    ///
    /// The `next_ebb()` method is intended for iterating over the EBBs in layout order:
    ///
    /// ```
    /// # use cretonne::ir::{Function, Ebb};
    /// # use cretonne::ir::layout::Cursor;
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = Cursor::new(&mut func.layout);
    ///     while let Some(ebb) = cursor.next_ebb() {
    ///         // Edit ebb.
    ///     }
    /// }
    /// ```
    pub fn next_ebb(&mut self) -> Option<Ebb> {
        let next = if let Some(ebb) = self.current_ebb() {
            self.layout.ebbs[ebb].next.wrap()
        } else {
            self.layout.first_ebb
        };
        self.pos = match next {
            Some(ebb) => CursorPosition::Before(ebb),
            None => CursorPosition::Nowhere,
        };
        next
    }

    /// Go to the bottom of the previous EBB in layout order and return it.
    ///
    /// - If the cursor wasn't pointing at anything, go to the bottom of the last EBB in the
    ///   function.
    /// - If there are no more EBBs, leave the cursor pointing at nothing and return `None`.
    ///
    /// # Examples
    ///
    /// The `prev_ebb()` method is intended for iterating over the EBBs in backwards layout order:
    ///
    /// ```
    /// # use cretonne::ir::{Function, Ebb};
    /// # use cretonne::ir::layout::Cursor;
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = Cursor::new(&mut func.layout);
    ///     while let Some(ebb) = cursor.prev_ebb() {
    ///         // Edit ebb.
    ///     }
    /// }
    /// ```
    pub fn prev_ebb(&mut self) -> Option<Ebb> {
        let prev = if let Some(ebb) = self.current_ebb() {
            self.layout.ebbs[ebb].prev.wrap()
        } else {
            self.layout.last_ebb
        };
        self.pos = match prev {
            Some(ebb) => CursorPosition::After(ebb),
            None => CursorPosition::Nowhere,
        };
        prev
    }

    /// Move to the next instruction in the same EBB and return it.
    ///
    /// - If the cursor was positioned before an EBB, go to the first instruction in that EBB.
    /// - If there are no more instructions in the EBB, go to the `After(ebb)` position and return
    ///   `None`.
    /// - If the cursor wasn't pointing anywhere, keep doing that.
    ///
    /// This method will never move the cursor to a different EBB.
    ///
    /// # Examples
    ///
    /// The `next_inst()` method is intended for iterating over the instructions in an EBB like
    /// this:
    ///
    /// ```
    /// # use cretonne::ir::{Function, Ebb};
    /// # use cretonne::ir::layout::Cursor;
    /// fn edit_ebb(func: &mut Function, ebb: Ebb) {
    ///     let mut cursor = Cursor::new(&mut func.layout);
    ///     cursor.goto_top(ebb);
    ///     while let Some(inst) = cursor.next_inst() {
    ///         // Edit instructions...
    ///     }
    /// }
    /// ```
    /// The loop body can insert and remove instructions via the cursor.
    ///
    /// Iterating over all the instructions in a function looks like this:
    ///
    /// ```
    /// # use cretonne::ir::{Function, Ebb};
    /// # use cretonne::ir::layout::Cursor;
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = Cursor::new(&mut func.layout);
    ///     while let Some(ebb) = cursor.next_ebb() {
    ///         while let Some(inst) = cursor.next_inst() {
    ///             // Edit instructions...
    ///         }
    ///     }
    /// }
    /// ```
    pub fn next_inst(&mut self) -> Option<Inst> {
        use self::CursorPosition::*;
        match self.pos {
            Nowhere | After(..) => None,
            At(inst) => {
                if let Some(next) = self.layout.insts[inst].next.wrap() {
                    self.pos = At(next);
                    Some(next)
                } else {
                    self.pos =
                        After(self.layout.inst_ebb(inst).expect("current instruction removed?"));
                    None
                }
            }
            Before(ebb) => {
                if let Some(next) = self.layout.ebbs[ebb].first_inst.wrap() {
                    self.pos = At(next);
                    Some(next)
                } else {
                    self.pos = After(ebb);
                    None
                }
            }
        }
    }

    /// Move to the previous instruction in the same EBB and return it.
    ///
    /// - If the cursor was positioned after an EBB, go to the last instruction in that EBB.
    /// - If there are no more instructions in the EBB, go to the `Before(ebb)` position and return
    ///   `None`.
    /// - If the cursor wasn't pointing anywhere, keep doing that.
    ///
    /// This method will never move the cursor to a different EBB.
    ///
    /// # Examples
    ///
    /// The `prev_inst()` method is intended for iterating backwards over the instructions in an
    /// EBB like this:
    ///
    /// ```
    /// # use cretonne::ir::{Function, Ebb};
    /// # use cretonne::ir::layout::Cursor;
    /// fn edit_ebb(func: &mut Function, ebb: Ebb) {
    ///     let mut cursor = Cursor::new(&mut func.layout);
    ///     cursor.goto_bottom(ebb);
    ///     while let Some(inst) = cursor.prev_inst() {
    ///         // Edit instructions...
    ///     }
    /// }
    /// ```
    pub fn prev_inst(&mut self) -> Option<Inst> {
        use self::CursorPosition::*;
        match self.pos {
            Nowhere | Before(..) => None,
            At(inst) => {
                if let Some(prev) = self.layout.insts[inst].prev.wrap() {
                    self.pos = At(prev);
                    Some(prev)
                } else {
                    self.pos =
                        Before(self.layout.inst_ebb(inst).expect("current instruction removed?"));
                    None
                }
            }
            After(ebb) => {
                if let Some(prev) = self.layout.ebbs[ebb].last_inst.wrap() {
                    self.pos = At(prev);
                    Some(prev)
                } else {
                    self.pos = Before(ebb);
                    None
                }
            }
        }
    }

    /// Insert an instruction at the current position.
    ///
    /// - If pointing at an instruction, the new instruction is inserted before the current
    ///   instruction.
    /// - If pointing at the bottom of an EBB, the new instruction is appended to the EBB.
    /// - Otherwise panic.
    ///
    /// In either case, the cursor is not moved, such that repeates calls to `insert_inst()` causes
    /// instructions to appear in insertion order in the EBB.
    pub fn insert_inst(&mut self, inst: Inst) {
        use self::CursorPosition::*;
        match self.pos {
            Nowhere | Before(..) => panic!("Invalid insert_inst position"),
            At(cur) => self.layout.insert_inst(inst, cur),
            After(ebb) => self.layout.append_inst(inst, ebb),
        }
    }

    /// Insert an EBB at the current position and switch to it.
    ///
    /// As far as possible, this method behaves as if the EBB header were an instruction inserted
    /// at the current position.
    ///
    /// - If the cursor is pointing at an existing instruction, *the current EBB is split in two*
    ///   and the current instruction becomes the first instruction in the inserted EBB.
    /// - If the cursor points at the bottom of an EBB, the new EBB is inserted after the current
    ///   one, and moved to the bottom of the new EBB where instructions can be appended.
    /// - If the cursor points to the top of an EBB, the new EBB is inserted above the current one.
    /// - If the cursor is not pointing at anything, the new EBB is placed last in the layout.
    ///
    /// This means that is is always valid to call this method, and it always leaves the cursor in
    /// a state that will insert instructions into the new EBB.
    pub fn insert_ebb(&mut self, new_ebb: Ebb) {
        use self::CursorPosition::*;
        match self.pos {
            At(inst) => {
                self.layout.split_ebb(new_ebb, inst);
                // All other cases move to `After(ebb)`, but in this case we we'll stay `At(inst)`.
                return;
            }
            Nowhere => self.layout.append_ebb(new_ebb),
            Before(ebb) => self.layout.insert_ebb(new_ebb, ebb),
            After(ebb) => self.layout.insert_ebb_after(new_ebb, ebb),
        }
        // For everything but `At(inst)` we end up appending to the new EBB.
        self.pos = After(new_ebb);
    }
}


#[cfg(test)]
mod tests {
    use super::{Layout, Cursor, CursorPosition};
    use entity_map::EntityRef;
    use ir::{Ebb, Inst};

    fn verify(layout: &mut Layout, ebbs: &[(Ebb, &[Inst])]) {
        // Check that EBBs are inserted and instructions belong the right places.
        // Check forward linkage with iterators.
        {
            let mut ebb_iter = layout.ebbs();
            for &(ebb, insts) in ebbs {
                assert!(layout.is_ebb_inserted(ebb));
                assert_eq!(ebb_iter.next(), Some(ebb));

                let mut inst_iter = layout.ebb_insts(ebb);
                for &inst in insts {
                    assert_eq!(layout.inst_ebb(inst), Some(ebb));
                    assert_eq!(inst_iter.next(), Some(inst));
                }
                assert_eq!(inst_iter.next(), None);
            }
            assert_eq!(ebb_iter.next(), None);
        }

        // Check backwards linkage with a cursor.
        let mut cur = Cursor::new(layout);
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
        let mut cur = Cursor::new(&mut layout);
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

        layout.append_inst(i0, e1);
        verify(&mut layout, &[(e1, &[i1, i2, i0])]);

        // Test cursor positioning.
        let mut cur = Cursor::new(&mut layout);
        cur.goto_top(e1);
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
            let mut cur = Cursor::new(&mut layout);
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
            let mut cur = Cursor::new(&mut layout);
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
    }
}
