use crate::{BorrowHandle, GuestError, Region};
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// A simple borrow checker to implement the API guarantees of Wiggle.
///
/// This is not a generalized borrow checker and is coarse-grained where it
/// doesn't actually take any regions into account. Instead it only tracks
/// whether there are temporally any shared/mutable borrows. This is
/// more-or-less a poor-man's `RefCell<T>`.
///
/// Note that this uses `&AtomicU32` because this is passed around as
/// `&BorrowChecker` in all `GuestPtr` structures. This needs to be mutated
/// which might look like it needs `Cell<u32>`, but `&Cell<u32>` isn't `Sync`
/// and we want futures with `&BorrowChecker` to be `Sync`, so this is an atomic
/// instead. Only one of these is created per-invocation though and it's not
/// actually shared across threads, so mutations here are not done with
/// compare-and-swap but instead just loads/stores.
pub struct BorrowChecker {
    // 0        => no borrows
    // >0       => shared borrows
    // u32::MAX => mutable borrow
    state: AtomicU32,
}

impl BorrowChecker {
    pub fn new() -> Self {
        BorrowChecker {
            state: AtomicU32::new(0),
        }
    }
    pub fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        match self.state.load(Relaxed) {
            n if n >= u32::MAX - 1 => Err(GuestError::PtrBorrowed(r)),
            n => {
                self.state.store(n + 1, Relaxed);
                Ok(BorrowHandle { _priv: () })
            }
        }
    }
    pub fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        match self.state.load(Relaxed) {
            0 => {
                self.state.store(u32::MAX, Relaxed);
                Ok(BorrowHandle { _priv: () })
            }
            _ => Err(GuestError::PtrBorrowed(r)),
        }
    }
    pub fn shared_unborrow(&self, _: BorrowHandle) {
        let prev = self.state.load(Relaxed);
        debug_assert!(prev > 0);
        self.state.store(prev - 1, Relaxed);
    }
    pub fn mut_unborrow(&self, _: BorrowHandle) {
        let prev = self.state.load(Relaxed);
        debug_assert!(prev == u32::MAX);
        self.state.store(0, Relaxed);
    }
    pub fn can_read(&self, _: Region) -> bool {
        self.state.load(Relaxed) != u32::MAX
    }
    pub fn can_write(&self, _: Region) -> bool {
        self.state.load(Relaxed) == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke() {
        let b = BorrowChecker::new();
        let mut next = 0;
        let mut region = || {
            let a = next;
            next += 1;
            Region::new(a, a + 1)
        };

        // can read/write initially
        assert!(b.can_read(region()));
        assert!(b.can_write(region()));

        // can shared borrow multiple times which will prevent mutable borrows
        let h1 = b.shared_borrow(region()).unwrap();
        let h2 = b.shared_borrow(region()).unwrap();
        assert!(b.mut_borrow(region()).is_err());

        // can read, but can't write, while there are shared borrows
        assert!(b.can_read(region()));
        assert!(!b.can_write(region()));

        // releasing shared borrows enables reading/writing again
        b.shared_unborrow(h1);
        b.shared_unborrow(h2);
        assert!(b.can_read(region()));
        assert!(b.can_write(region()));

        // mutable borrow disallows shared borrows
        let h1 = b.mut_borrow(region()).unwrap();
        assert!(b.shared_borrow(region()).is_err());

        // active mutable borrows disallows reads/writes
        assert!(!b.can_read(region()));
        assert!(!b.can_write(region()));

        // releasing the mutable borrows allows reading/writing again
        b.mut_unborrow(h1);
        assert!(b.can_read(region()));
        assert!(b.can_write(region()));
    }
}
