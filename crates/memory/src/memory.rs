use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use thiserror::Error;

use crate::borrow::{BorrowHandle, GuestBorrows};
use crate::guest_type::GuestType;
use crate::region::Region;

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

    pub fn ptr<T: GuestType>(&'a self, at: u32) -> Result<GuestPtr<'a, T>, MemoryError> {
        let region = Region {
            start: at,
            len: T::size(),
        };
        if !self.contains(region) {
            Err(MemoryError::OutOfBounds(region))?;
        }
        let mut borrows = self.borrows.borrow_mut();
        if let Some(handle) = borrows.borrow_immut(region) {
            Ok(GuestPtr {
                mem: &self,
                region,
                handle,
                type_: PhantomData,
            })
        } else {
            Err(MemoryError::Borrowed(region))
        }
    }

    pub fn ptr_mut<T: GuestType>(&'a self, at: u32) -> Result<GuestPtrMut<'a, T>, MemoryError> {
        let region = Region {
            start: at,
            len: T::size(),
        };
        if !self.contains(region) {
            Err(MemoryError::OutOfBounds(region))?;
        }
        let mut borrows = self.borrows.borrow_mut();
        if let Some(handle) = borrows.borrow_mut(region) {
            Ok(GuestPtrMut {
                mem: &self,
                region,
                handle,
                type_: PhantomData,
            })
        } else {
            Err(MemoryError::Borrowed(region))
        }
    }
}

pub trait GuestPtrRead<T> {
    fn ptr(&self) -> *const u8;
}

pub struct GuestPtr<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    handle: BorrowHandle,
    type_: PhantomData<T>,
}

impl<'a, T: GuestType> GuestPtrRead<T> for GuestPtr<'a, T> {
    fn ptr(&self) -> *const u8 {
        (self.mem.ptr as usize + self.region.start as usize) as *const u8
    }
}

impl<'a, T> Drop for GuestPtr<'a, T> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_immut(self.handle);
    }
}

pub struct GuestPtrMut<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    handle: BorrowHandle,
    type_: PhantomData<T>,
}

impl<'a, T: GuestType> GuestPtrRead<T> for GuestPtrMut<'a, T> {
    fn ptr(&self) -> *const u8 {
        (self.mem.ptr as usize + self.region.start as usize) as *const u8
    }
}

impl<'a, T> GuestPtrMut<'a, T> {
    pub fn ptr_mut(&self) -> *mut u8 {
        (self.mem.ptr as usize + self.region.start as usize) as *mut u8
    }
}
impl<'a, T> Drop for GuestPtrMut<'a, T> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_mut(self.handle);
    }
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Out of bounds: {0:?}")]
    OutOfBounds(Region),
    #[error("Borrowed: {0:?}")]
    Borrowed(Region),
}
