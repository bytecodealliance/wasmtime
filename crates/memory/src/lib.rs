#![allow(dead_code, unused)] // DURING DEVELOPMENT

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    start: u32,
    len: u32,
}

impl Region {
    fn overlaps(&self, rhs: Region) -> bool {
        let self_start = self.start as u64;
        let self_end = self.start as u64 + self.len as u64;

        let rhs_start = rhs.start as u64;
        let rhs_end = rhs.start as u64 + rhs.len as u64;

        // start of rhs inside self:
        if (rhs_start >= self_start && rhs_start < self_end) {
            return true;
        }

        // end of rhs inside self:
        if (rhs_end >= self_start && rhs_end < self_end) {
            return true;
        }

        // start of self inside rhs:
        if (self_start >= rhs_start && self_start < rhs_end) {
            return true;
        }

        // end of self inside rhs: XXX is this redundant? i suspect it is but im too tired
        if (self_end >= rhs_start && self_end < rhs_end) {
            return true;
        }

        false
    }
}

struct GuestBorrows {
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

pub struct GuestMemory<'a> {
    ptr: *mut u8,
    len: u32,
    lifetime: PhantomData<&'a ()>,
    borrows: Rc<RefCell<GuestBorrows>>,
}

impl<'a> GuestMemory<'a> {
    pub fn new(ptr: *mut u8, len: u32) -> GuestMemory<'a> {
        GuestMemory {
            ptr,
            len,
            lifetime: PhantomData,
            borrows: Rc::new(RefCell::new(GuestBorrows::new())),
        }
    }

    fn contains(&self, r: Region) -> bool {
        r.start < self.len 
            && r.len < self.len // make sure next clause doesnt underflow
            && r.start < (self.len - r.len)
    }

    pub fn ptr(&'a self, r: Region) -> Result<Option<GuestPtr<'a>>, MemoryError> {
        let mut borrows = self.borrows.borrow_mut();
        if !self.contains(r) {
            Err(MemoryError::OutOfBounds)?;
        }
        if borrows.borrow_immut(r) {
            Ok(Some(GuestPtr {
                mem: &self,
                region: r,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn ptr_mut(&'a self, r: Region) -> Result<Option<GuestPtrMut<'a>>, MemoryError> {
        let mut borrows = self.borrows.borrow_mut();
        if !self.contains(r) {
            Err(MemoryError::OutOfBounds)?;
        }
        if borrows.borrow_immut(r) {
            Ok(Some(GuestPtrMut {
                mem: &self,
                region: r,
            }))
        } else {
            Ok(None)
        }
    }

}

pub struct GuestPtr<'a> {
    mem: &'a GuestMemory<'a>,
    region: Region,
}

impl<'a> Drop for GuestPtr<'a> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_immut(self.region);
    }
}


pub struct GuestPtrMut<'a> {
    mem: &'a GuestMemory<'a>,
    region: Region,
}

impl<'a> Drop for GuestPtrMut<'a> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_mut(self.region);
    }
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Out of bounds")]
    OutOfBounds,
}
