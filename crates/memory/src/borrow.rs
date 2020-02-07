use std::collections::HashMap;

use crate::region::Region;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BorrowHandle(usize);

#[derive(Debug)]
pub struct GuestBorrows {
    immutable: HashMap<BorrowHandle, Region>,
    mutable: HashMap<BorrowHandle, Region>,
    next_handle: BorrowHandle,
}

impl GuestBorrows {
    pub fn new() -> Self {
        GuestBorrows {
            immutable: HashMap::new(),
            mutable: HashMap::new(),
            next_handle: BorrowHandle(0),
        }
    }

    fn is_borrowed_immut(&self, r: Region) -> bool {
        !self.immutable.values().all(|b| !b.overlaps(r))
    }

    fn is_borrowed_mut(&self, r: Region) -> bool {
        !self.mutable.values().all(|b| !b.overlaps(r))
    }

    fn new_handle(&mut self) -> BorrowHandle {
        let h = self.next_handle;
        self.next_handle = BorrowHandle(h.0 + 1);
        h
    }

    pub fn borrow_immut(&mut self, r: Region) -> Option<BorrowHandle> {
        if self.is_borrowed_mut(r) {
            return None;
        }
        let h = self.new_handle();
        self.immutable.insert(h, r);
        Some(h)
    }

    pub fn unborrow_immut(&mut self, h: BorrowHandle) {
        self.immutable
            .remove(&h)
            .expect("handle exists in immutable borrows");
    }

    pub fn borrow_mut(&mut self, r: Region) -> Option<BorrowHandle> {
        if self.is_borrowed_immut(r) || self.is_borrowed_mut(r) {
            return None;
        }
        let h = self.new_handle();
        self.mutable.insert(h, r);
        Some(h)
    }

    pub fn unborrow_mut(&mut self, h: BorrowHandle) {
        self.mutable
            .remove(&h)
            .expect("handle exists in mutable borrows");
    }
}
