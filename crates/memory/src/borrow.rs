use crate::region::Region;

pub struct GuestBorrows {
    immutable: Vec<Region>,
    mutable: Vec<Region>,
}

impl GuestBorrows {
    pub fn new() -> Self {
        GuestBorrows {
            immutable: Vec::new(),
            mutable: Vec::new(),
        }
    }

    fn is_borrowed_immut(&self, r: Region) -> bool {
        !self.immutable.iter().all(|b| !b.overlaps(r))
    }

    fn is_borrowed_mut(&self, r: Region) -> bool {
        !self.mutable.iter().all(|b| !b.overlaps(r))
    }

    pub fn borrow_immut(&mut self, r: Region) -> bool {
        if self.is_borrowed_mut(r) {
            return false;
        }
        self.immutable.push(r);
        true
    }

    pub fn unborrow_immut(&mut self, r: Region) {
        let (ix, _) = self
            .immutable
            .iter()
            .enumerate()
            .find(|(_, reg)| r == **reg)
            .expect("region exists in borrows");
        self.immutable.remove(ix);
    }

    pub fn borrow_mut(&mut self, r: Region) -> bool {
        if self.is_borrowed_immut(r) || self.is_borrowed_mut(r) {
            return false;
        }
        self.mutable.push(r);
        true
    }

    pub fn unborrow_mut(&mut self, r: Region) {
        let (ix, _) = self
            .mutable
            .iter()
            .enumerate()
            .find(|(_, reg)| r == **reg)
            .expect("region exists in borrows");
        self.mutable.remove(ix);
    }
}
