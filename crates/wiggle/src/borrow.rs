use crate::error::GuestError;
use crate::region::Region;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BorrowHandle(usize);

pub struct BorrowChecker {
    bc: RefCell<InnerBorrowChecker>,
}

impl BorrowChecker {
    /// A `BorrowChecker` manages run-time validation of borrows from a `GuestMemory`. It keeps
    /// track of regions of guest memory which are possible to alias with Rust references (via the
    /// `GuestSlice` and `GuestStr` structs, which implement `std::ops::Deref` and
    /// `std::ops::DerefMut`. It also enforces that `GuestPtr::read` and `GuestPtr::write` do not
    /// access memory with an outstanding borrow.
    /// The safety of this mechanism depends on creating exactly one `BorrowChecker` per
    /// WebAssembly memory. There must be no other reads or writes of WebAssembly the memory by
    /// either Rust or WebAssembly code while there are any outstanding borrows, as given by
    /// `BorrowChecker::has_outstanding_borrows()`.
    pub unsafe fn new() -> Self {
        BorrowChecker {
            bc: RefCell::new(InnerBorrowChecker::new()),
        }
    }
    /// Indicates whether any outstanding borrows are known to the `BorrowChecker`. This function
    /// must be `false` in order for it to be safe to recursively call into a WebAssembly module,
    /// or to manipulate the WebAssembly memory by any other means.
    pub fn has_outstanding_borrows(&self) -> bool {
        self.bc.borrow().has_outstanding_borrows()
    }

    pub(crate) fn borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.borrow_mut().borrow(r)
    }
    pub(crate) fn unborrow(&self, h: BorrowHandle) {
        self.bc.borrow_mut().unborrow(h)
    }
    pub(crate) fn is_borrowed(&self, r: Region) -> bool {
        self.bc.borrow().is_borrowed(r)
    }
}

#[derive(Debug)]
struct InnerBorrowChecker {
    borrows: HashMap<BorrowHandle, Region>,
    next_handle: BorrowHandle,
}

impl InnerBorrowChecker {
    fn new() -> Self {
        InnerBorrowChecker {
            borrows: HashMap::new(),
            next_handle: BorrowHandle(0),
        }
    }

    fn has_outstanding_borrows(&self) -> bool {
        !self.borrows.is_empty()
    }

    fn is_borrowed(&self, r: Region) -> bool {
        !self.borrows.values().all(|b| !b.overlaps(r))
    }

    fn new_handle(&mut self) -> Result<BorrowHandle, GuestError> {
        // Reset handles to 0 if all handles have been returned.
        if self.borrows.is_empty() {
            self.next_handle = BorrowHandle(0);
        }
        let h = self.next_handle;
        self.next_handle = BorrowHandle(
            h.0.checked_add(1)
                .ok_or_else(|| GuestError::BorrowCheckerOOM)?,
        );
        Ok(h)
    }

    fn borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {
        if self.is_borrowed(r) {
            return Err(GuestError::PtrBorrowed(r));
        }
        let h = self.new_handle()?;
        self.borrows.insert(h, r);
        Ok(h)
    }

    fn unborrow(&mut self, h: BorrowHandle) {
        let _ = self.borrows.remove(&h);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn nonoverlapping() {
        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");

        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");
    }

    #[test]
    fn overlapping() {
        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(10, 5);
        let r3 = Region::new(15, 5);
        let r4 = Region::new(0, 10);
        assert!(r1.overlaps(r4));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");
        bs.borrow(r3).expect("can borrow r3");
        assert!(bs.borrow(r4).is_err(), "cant borrow r4");
    }

    #[test]
    fn unborrowing() {
        let mut bs = InnerBorrowChecker::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        assert_eq!(bs.has_outstanding_borrows(), false, "start with no borrows");
        let h1 = bs.borrow(r1).expect("can borrow r1");
        assert_eq!(bs.has_outstanding_borrows(), true, "h1 is outstanding");
        let h2 = bs.borrow(r2).expect("can borrow r2");

        assert!(bs.borrow(r2).is_err(), "can't borrow r2 twice");
        bs.unborrow(h2);
        assert_eq!(
            bs.has_outstanding_borrows(),
            true,
            "h1 is still outstanding"
        );
        bs.unborrow(h1);
        assert_eq!(bs.has_outstanding_borrows(), false, "no remaining borrows");

        let _h3 = bs
            .borrow(r2)
            .expect("can borrow r2 again now that its been unborrowed");
    }
}
