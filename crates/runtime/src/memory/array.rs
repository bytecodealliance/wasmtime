use super::ptr::{GuestPtr, GuestPtrMut, GuestRef, GuestRefMut};
use crate::{GuestError, GuestType, GuestTypeCopy};
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

pub struct GuestArray<'a, T>
where
    T: GuestType,
{
    pub(super) ptr: GuestPtr<'a, T>,
    pub(super) num_elems: u32,
}

impl<'a, T> fmt::Debug for GuestArray<'a, T>
where
    T: GuestType + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestArray {{ ptr: {:?}, num_elems: {:?} }}",
            self.ptr, self.num_elems
        )
    }
}

impl<'a, T> GuestArray<'a, T>
where
    T: GuestTypeCopy,
{
    pub fn as_ref(&self) -> Result<GuestArrayRef<'a, T>, GuestError> {
        let mut ptr = self.ptr.clone();
        for _ in 0..self.num_elems {
            ptr = ptr.elem(1)?;
            T::validate(&ptr)?;
        }
        let region = self.ptr.region.extend((self.num_elems - 1) * T::size());
        let handle = {
            let mut borrows = self.ptr.mem.borrows.borrow_mut();
            borrows
                .borrow_immut(region)
                .ok_or_else(|| GuestError::PtrBorrowed(region))?
        };
        let ref_ = GuestRef {
            mem: self.ptr.mem,
            region,
            handle,
            type_: self.ptr.type_,
        };
        Ok(GuestArrayRef {
            ref_,
            num_elems: self.num_elems,
        })
    }
}

pub struct GuestArrayRef<'a, T>
where
    T: GuestType,
{
    ref_: GuestRef<'a, T>,
    num_elems: u32,
}

impl<'a, T> fmt::Debug for GuestArrayRef<'a, T>
where
    T: GuestType + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestArrayRef {{ ref_: {:?}, num_elems: {:?} }}",
            self.ref_, self.num_elems
        )
    }
}

impl<'a, T> Deref for GuestArrayRef<'a, T>
where
    T: GuestTypeCopy,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(
                self.ref_.as_ptr().as_raw() as *const T,
                self.num_elems as usize,
            )
        }
    }
}

pub struct GuestArrayMut<'a, T>
where
    T: GuestType,
{
    pub(super) ptr: GuestPtrMut<'a, T>,
    pub(super) num_elems: u32,
}

impl<'a, T> fmt::Debug for GuestArrayMut<'a, T>
where
    T: GuestType + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestArrayMut {{ ptr: {:?}, num_elems: {:?} }}",
            self.ptr, self.num_elems
        )
    }
}

impl<'a, T> GuestArrayMut<'a, T>
where
    T: GuestTypeCopy,
{
    pub fn as_ref(&self) -> Result<GuestArrayRef<'a, T>, GuestError> {
        let arr = GuestArray {
            ptr: self.ptr.as_immut(),
            num_elems: self.num_elems,
        };
        arr.as_ref()
    }

    pub fn as_ref_mut(&self) -> Result<GuestArrayRefMut<'a, T>, GuestError> {
        let mut ptr = self.ptr.as_immut();
        for _ in 0..self.num_elems {
            ptr = ptr.elem(1)?;
            T::validate(&ptr)?;
        }
        let region = self.ptr.region.extend((self.num_elems - 1) * T::size());
        let handle = {
            let mut borrows = self.ptr.mem.borrows.borrow_mut();
            borrows
                .borrow_mut(region)
                .ok_or_else(|| GuestError::PtrBorrowed(region))?
        };
        let ref_mut = GuestRefMut {
            mem: self.ptr.mem,
            region,
            handle,
            type_: self.ptr.type_,
        };
        Ok(GuestArrayRefMut {
            ref_mut,
            num_elems: self.num_elems,
        })
    }
}

pub struct GuestArrayRefMut<'a, T>
where
    T: GuestType,
{
    ref_mut: GuestRefMut<'a, T>,
    num_elems: u32,
}

impl<'a, T> fmt::Debug for GuestArrayRefMut<'a, T>
where
    T: GuestType + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestArrayRefMut {{ ref_mut: {:?}, num_elems: {:?} }}",
            self.ref_mut, self.num_elems
        )
    }
}

impl<'a, T> Deref for GuestArrayRefMut<'a, T>
where
    T: GuestTypeCopy,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(
                self.ref_mut.as_ptr().as_raw() as *const T,
                self.num_elems as usize,
            )
        }
    }
}

impl<'a, T> DerefMut for GuestArrayRefMut<'a, T>
where
    T: GuestTypeCopy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.ref_mut.as_ptr_mut().as_raw() as *mut T,
                self.num_elems as usize,
            )
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::{
        ptr::{GuestPtr, GuestPtrMut},
        GuestError, GuestMemory, Region,
    };

    #[repr(align(4096))]
    struct HostMemory {
        buffer: [u8; 4096],
    }

    impl HostMemory {
        pub fn new() -> Self {
            Self { buffer: [0; 4096] }
        }
        pub fn as_mut_ptr(&mut self) -> *mut u8 {
            self.buffer.as_mut_ptr()
        }
        pub fn len(&self) -> usize {
            self.buffer.len()
        }
    }

    #[test]
    fn out_of_bounds() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // try extracting an immutable array out of memory bounds
        let ptr: GuestPtr<i32> = guest_memory.ptr(4092).expect("ptr to last i32 el");
        let err = ptr.array(2).expect_err("out of bounds ptr error");
        assert_eq!(err, GuestError::PtrOutOfBounds(Region::new(4092, 8)));
        // try extracting an mutable array out of memory bounds
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(4092).expect("ptr mut to last i32 el");
        let err = ptr.array_mut(2).expect_err("out of bounds ptr error");
        assert_eq!(err, GuestError::PtrOutOfBounds(Region::new(4092, 8)));
    }

    #[test]
    fn ptr_to_array() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // write a simple array into memory
        {
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
            let mut el = ptr.as_ref_mut().expect("ref mut to first el");
            *el = 1;
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(4).expect("ptr mut to second el");
            let mut el = ptr.as_ref_mut().expect("ref mu to second el");
            *el = 2;
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(8).expect("ptr mut to third el");
            let mut el = ptr.as_ref_mut().expect("ref mut to third el");
            *el = 3;
        }
        // extract as array
        let ptr: GuestPtr<i32> = guest_memory.ptr(0).expect("ptr to first el");
        let arr = ptr.array(3).expect("convert ptr to array");
        let as_ref = arr.as_ref().expect("array borrowed immutably");
        assert_eq!(&*as_ref, &[1, 2, 3]);
        // borrowing again should be fine
        let as_ref2 = arr.as_ref().expect("array borrowed immutably again");
        assert_eq!(&*as_ref2, &*as_ref);
    }

    #[test]
    fn ptr_mut_to_array_mut() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // set elems of array to zero
        {
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
            let mut el = ptr.as_ref_mut().expect("ref mut to first el");
            *el = 0;
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(4).expect("ptr mut to second el");
            let mut el = ptr.as_ref_mut().expect("ref mu to second el");
            *el = 0;
            let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(8).expect("ptr mut to third el");
            let mut el = ptr.as_ref_mut().expect("ref mut to third el");
            *el = 0;
        }
        // extract as array and verify all is zero
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
        let arr = ptr.array_mut(3).expect("convert ptr mut to array mut");
        assert_eq!(&*arr.as_ref().expect("array borrowed immutably"), &[0; 3]);
        // populate the array and re-verify
        for el in &mut *arr.as_ref_mut().expect("array borrowed mutably") {
            *el = 10;
        }
        // re-validate
        assert_eq!(&*arr.as_ref().expect("array borrowed immutably"), &[10; 3]);
    }

    #[test]
    #[should_panic(
        expected = "array borrowed immutably while borrowed mutably: PtrBorrowed(Region { start: 0, len: 12 })"
    )]
    fn borrow_mut_then_immut() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
        let arr = ptr.array_mut(3).expect("convert ptr mut to array mut");
        // borrow mutably
        let _as_mut = arr
            .as_ref_mut()
            .expect("array borrowed mutably for the first time");
        // borrow immutably should fail
        let _as_ref = arr
            .as_ref()
            .expect("array borrowed immutably while borrowed mutably");
    }

    #[test]
    #[should_panic(
        expected = "array borrowed mutably while borrowed mutably: PtrBorrowed(Region { start: 0, len: 12 })"
    )]
    fn borrow_mut_twice() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
        let arr = ptr.array_mut(3).expect("convert ptr mut to array mut");
        // borrow mutably
        let _as_mut = arr
            .as_ref_mut()
            .expect("array borrowed mutably for the first time");
        // try borrowing mutably again
        let _as_mut2 = arr
            .as_ref_mut()
            .expect("array borrowed mutably while borrowed mutably");
    }
}
