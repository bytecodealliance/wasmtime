//! Function layout.
//!
//! The order of extended basic blocks in a function and the order of instructions in an EBB is
//! determined by the `Layout` data structure defined in this module.

use std::iter::{Iterator, IntoIterator};
use entity_map::{EntityMap, EntityRef};
use entities::{Ebb, NO_EBB, Inst, NO_INST};

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
/// EBBs does not affect the semantics of the program.
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
            let node = &mut self.ebbs[ebb];
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
        self.ebbs[ebb].next = before;
        self.ebbs[ebb].prev = after;
        self.ebbs[before].prev = ebb;
        if after == NO_EBB {
            self.first_ebb = Some(ebb);
        } else {
            self.ebbs[after].next = ebb;
        }
    }

    /// Return an iterator over all EBBs in layout order.
    pub fn ebbs<'a>(&'a self) -> Ebbs<'a> {
        Ebbs {
            layout: self,
            next: self.first_ebb,
        }
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
            let inst_node = &mut self.insts[inst];
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

    /// Insert `inst` before the instruction `before` in the same EBB.
    pub fn insert_inst(&mut self, inst: Inst, before: Inst) {
        assert_eq!(self.inst_ebb(inst), None);
        let ebb = self.inst_ebb(before)
            .expect("Instruction before insertion point not in the layout");
        let after = self.insts[before].prev;
        {
            let inst_node = &mut self.insts[inst];
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

#[cfg(test)]
mod tests {
    use super::Layout;
    use entity_map::EntityRef;
    use entities::{Ebb, Inst};

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

            let v: Vec<Ebb> = layout.ebbs().collect();
            assert_eq!(v, []);
        }

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
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e1]);

        layout.insert_ebb(e2, e1);
        assert!(!layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e2, e1]);

        layout.insert_ebb(e0, e1);
        assert!(layout.is_ebb_inserted(e0));
        assert!(layout.is_ebb_inserted(e1));
        assert!(layout.is_ebb_inserted(e2));
        let v: Vec<Ebb> = layout.ebbs().collect();
        assert_eq!(v, [e2, e0, e1]);
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
        assert_eq!(layout.inst_ebb(i0), Some(e1));
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), Some(e1));
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i1, i2, i0]);
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
        assert_eq!(layout.inst_ebb(i0), Some(e1));
        assert_eq!(layout.inst_ebb(i1), Some(e1));
        assert_eq!(layout.inst_ebb(i2), Some(e1));
        let v: Vec<Inst> = layout.ebb_insts(e1).collect();
        assert_eq!(v, [i2, i0, i1]);
    }

    #[test]
    fn multiple_ebbs() {
        let mut layout = Layout::new();

        let e0 = Ebb::new(0);
        let e1 = Ebb::new(1);

        layout.append_ebb(e0);
        layout.append_ebb(e1);

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
}
