use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::borrow::{BorrowHandle, GuestBorrows};
use crate::{GuestError, GuestType, Region};

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

    pub fn ptr<T: GuestType>(&'a self, at: u32) -> Result<GuestPtr<'a, T>, GuestError> {
        let region = Region {
            start: at,
            len: T::size(),
        };
        if !self.contains(region) {
            Err(GuestError::PtrOutOfBounds(region))?;
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
            Err(GuestError::PtrBorrowed(region))
        }
    }

    pub fn ptr_mut<T: GuestType>(&'a self, at: u32) -> Result<GuestPtrMut<'a, T>, GuestError> {
        let region = Region {
            start: at,
            len: T::size(),
        };
        if !self.contains(region) {
            Err(GuestError::PtrOutOfBounds(region))?;
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
            Err(GuestError::PtrBorrowed(region))
        }
    }
}

/// These methods should not be used by the end user - just by implementations of the
/// GuestValueClone and GuestValueCopy traits!
pub trait GuestPtrRead<'a, T> {
    fn mem(&self) -> &'a GuestMemory<'a>;
    fn region(&self) -> &Region;
    fn ptr(&self) -> *const u8 {
        (self.mem().ptr as usize + self.region().start as usize) as *const u8
    }
}

pub struct GuestPtr<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    handle: BorrowHandle,
    type_: PhantomData<T>,
}

impl<'a, T> GuestPtrRead<'a, T> for GuestPtr<'a, T> {
    fn mem(&self) -> &'a GuestMemory<'a> {
        self.mem
    }
    fn region(&self) -> &Region {
        &self.region
    }
}

impl<'a, T> GuestType for GuestPtr<'a, T> {
    fn size() -> u32 {
        4
    }
    fn name() -> &'static str {
        "GuestPtr<...>"
    }
}

impl<'a, T: GuestType> GuestPtr<'a, T> {
    pub fn read_ptr<P: GuestPtrRead<'a, Self>>(src: &P) -> Result<Self, GuestError> {
        let raw_ptr = unsafe { ::std::ptr::read_unaligned(src.ptr() as *const u32) };
        src.mem().ptr(raw_ptr)
    }
    pub fn write_ptr(ptr: &Self, dest: &GuestPtrMut<Self>) {
        unsafe { ::std::ptr::write_unaligned(dest.ptr_mut() as *mut u32, ptr.region.start) }
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

impl<'a, T> GuestPtrRead<'a, T> for GuestPtrMut<'a, T> {
    fn mem(&self) -> &'a GuestMemory<'a> {
        self.mem
    }
    fn region(&self) -> &Region {
        &self.region
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

impl<'a, T> GuestType for GuestPtrMut<'a, T> {
    fn size() -> u32 {
        4
    }
    fn name() -> &'static str {
        "GuestPtrMut<...>"
    }
}

impl<'a, T: GuestType> GuestPtrMut<'a, T> {
    pub fn read_ptr<P: GuestPtrRead<'a, Self>>(src: &P) -> Result<Self, GuestError> {
        let raw_ptr = unsafe { ::std::ptr::read_unaligned(src.ptr() as *const u32) };
        src.mem().ptr_mut(raw_ptr)
    }
    pub fn write_ptr(ptr: &Self, dest: &GuestPtrMut<Self>) {
        unsafe { ::std::ptr::write_unaligned(dest.ptr_mut() as *mut u32, ptr.region.start) }
    }

    pub fn as_immut(self) -> GuestPtr<'a, T> {
        let mem = self.mem;
        let start = self.region.start;
        drop(self);
        mem.ptr(start)
            .expect("can borrow just-dropped mutable region as immut")
    }
}
