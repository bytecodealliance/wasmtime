mod array;
mod ptr;

pub use array::*;
pub use ptr::*;

use crate::{borrow::GuestBorrows, GuestError, GuestType, Region};
use std::{cell::RefCell, fmt, marker::PhantomData, rc::Rc};

pub struct GuestMemory<'a> {
    ptr: *mut u8,
    len: u32,
    lifetime: PhantomData<&'a ()>,
    borrows: Rc<RefCell<GuestBorrows>>,
}

impl<'a> fmt::Debug for GuestMemory<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestMemory {{ ptr: {:?}, len: {:?}, borrows: {:?} }}",
            self.ptr, self.len, self.borrows
        )
    }
}

impl<'a> GuestMemory<'a> {
    pub fn new(ptr: *mut u8, len: u32) -> Self {
        assert_eq!(ptr as usize % 4096, 0, "GuestMemory must be page-aligned");
        Self {
            ptr,
            len,
            lifetime: PhantomData,
            borrows: Rc::new(RefCell::new(GuestBorrows::new())),
        }
    }

    fn contains(&self, r: Region) -> bool {
        r.start < self.len
            && r.len < self.len // make sure next clause doesnt underflow
            && r.start <= (self.len - r.len)
    }

    pub fn ptr<T: GuestType>(&'a self, at: u32) -> Result<GuestPtr<'a, T>, GuestError> {
        let region = Region {
            start: at,
            len: T::size(),
        };
        if !self.contains(region) {
            Err(GuestError::PtrOutOfBounds(region))?;
        }
        if at % T::align() != 0 {
            Err(GuestError::PtrNotAligned(region, T::align()))?;
        }
        Ok(GuestPtr {
            mem: &self,
            region,
            type_: PhantomData,
        })
    }

    pub fn ptr_mut<T: GuestType>(&'a self, at: u32) -> Result<GuestPtrMut<'a, T>, GuestError> {
        let ptr = self.ptr(at)?;
        Ok(GuestPtrMut {
            mem: ptr.mem,
            region: ptr.region,
            type_: ptr.type_,
        })
    }
}
