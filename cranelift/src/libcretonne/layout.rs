//! Function layout.
//!
//! The order of extended basic blocks in a function and the order of instructions in an EBB is
//! determined by the `Layout` data structure defined in this module.

use std::iter::Iterator;
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
        let node = &mut self.ebbs[ebb];
        assert!(node.first_inst == NO_INST && node.last_inst == NO_INST);
        node.prev = self.last_ebb.unwrap_or_default();
        node.next = NO_EBB;
        self.last_ebb = Some(ebb);
        if self.first_ebb.is_none() {
            self.first_ebb = Some(ebb);
        }
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

/// Methods for arranging instructions.
///
/// An instruction starts out as *not inserted* in the layout. An instruction can be inserted into
/// an EBB at a given position.
impl Layout {
    /// Get the EBB containing `inst`, or `None` if `inst` is not inserted in the layout.
    pub fn inst_ebb(&self, inst: Inst) -> Option<Ebb> {
        if self.insts.is_valid(inst) {
            let ebb = self.insts[inst].ebb;
            if ebb == NO_EBB {
                None
            } else {
                Some(ebb)
            }
        } else {
            None
        }
    }

    /// Append `inst` to the end of `ebb`.
    pub fn append_inst(&self, inst: Inst, ebb: Ebb) {
        assert_eq!(self.inst_ebb(inst), None);
        assert!(self.is_ebb_inserted(ebb),
                "Cannot append instructions to EBB not in layout");
        unimplemented!();
    }

    /// Insert `inst` before the instruction `before` in the same EBB.
    pub fn insert_inst(&self, inst: Inst, before: Inst) {
        assert_eq!(self.inst_ebb(inst), None);
        let ebb = self.inst_ebb(before)
            .expect("Instruction before insertion point not in the layout");
        assert!(ebb != NO_EBB);
        unimplemented!();
    }
}

#[derive(Clone, Debug, Default)]
struct InstNode {
    ebb: Ebb,
    prev: Inst,
    next: Inst,
}

#[cfg(test)]
mod tests {
    use super::Layout;
    use entity_map::EntityRef;
    use entities::Ebb;

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
}
