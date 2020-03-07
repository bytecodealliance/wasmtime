use crate::region::Region;
use crate::GuestError;

#[derive(Debug)]
pub struct GuestBorrows {
    borrows: Vec<Region>,
}

impl GuestBorrows {
    pub fn new() -> Self {
        Self {
            borrows: Vec::new(),
        }
    }

    fn is_borrowed(&self, r: Region) -> bool {
        !self.borrows.iter().all(|b| !b.overlaps(r))
    }

    pub fn borrow(&mut self, r: Region) -> Result<(), GuestError> {
        if self.is_borrowed(r) {
            Err(GuestError::PtrBorrowed(r))
        } else {
            self.borrows.push(r);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn nonoverlapping() {
        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        bs.borrow(r2).expect("can borrow r2");
    }

    #[test]
    fn overlapping() {
        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.borrow(r1).expect("can borrow r1");
        assert!(bs.borrow(r2).is_err(), "cant borrow r2");

        let mut bs = GuestBorrows::new();
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
}
