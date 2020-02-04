use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::borrow::{BorrowHandle, GuestBorrows};
use crate::{GuestError, GuestType, GuestTypeClone, GuestTypeCopy, GuestTypePtr, Region};

pub struct GuestMemory<'a> {
    ptr: *mut u8,
    len: u32,
    lifetime: PhantomData<&'a ()>,
    borrows: Rc<RefCell<GuestBorrows>>,
}

impl<'a> GuestMemory<'a> {
    pub fn new(ptr: *mut u8, len: u32) -> GuestMemory<'a> {
        assert_eq!(ptr as usize % 4096, 0, "GuestMemory must be page-aligned");
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

#[derive(Clone)]
pub struct GuestPtr<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    type_: PhantomData<T>,
}

impl<'a, T: GuestType> GuestPtr<'a, T> {
    pub fn as_raw(&self) -> *const u8 {
        (self.mem.ptr as usize + self.region.start as usize) as *const u8
    }

    pub fn elem(&self, elements: i32) -> Result<GuestPtr<'a, T>, GuestError> {
        self.mem
            .ptr(self.region.start + (elements * self.region.len as i32) as u32)
    }

    pub fn cast<TT: GuestType>(&self, offset: u32) -> Result<GuestPtr<'a, TT>, GuestError> {
        self.mem.ptr(self.region.start + offset)
    }
}

impl<'a, T: GuestTypeCopy> GuestPtr<'a, T> {
    pub fn as_ref(&self) -> Result<GuestRef<'a, T>, GuestError> {
        T::validate(&self)?;
        let handle = {
            let mut borrows = self.mem.borrows.borrow_mut();
            borrows
                .borrow_immut(self.region)
                .ok_or_else(|| GuestError::PtrBorrowed(self.region))?
        };
        Ok(GuestRef {
            mem: self.mem,
            region: self.region,
            handle,
            type_: self.type_,
        })
    }
}

impl<'a, T: GuestTypeClone> GuestPtr<'a, T> {
    pub fn clone_from_guest(&self) -> Result<T, GuestError> {
        T::read_from_guest(self)
    }
}

impl<'a, T: GuestTypePtr<'a>> GuestPtr<'a, T> {
    pub fn read_ptr_from_guest(&self) -> Result<T, GuestError> {
        T::read_from_guest(self)
    }
}

impl<'a, T> GuestType for GuestPtr<'a, T>
where
    T: GuestType,
{
    fn size() -> u32 {
        4
    }
    fn align() -> u32 {
        4
    }
    fn name() -> String {
        format!("GuestPtr<{}>", T::name())
    }
    fn validate<'b>(location: &GuestPtr<'b, GuestPtr<'b, T>>) -> Result<(), GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let _guest_ptr: GuestPtr<T> = location.mem.ptr(raw_ptr)?;
        Ok(())
    }
}

// Operations for reading and writing Ptrs to memory:
impl<'a, T> GuestTypePtr<'a> for GuestPtr<'a, T>
where
    T: GuestType,
{
    fn read_from_guest(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let guest_ptr: GuestPtr<'a, T> = location.mem.ptr(raw_ptr)?;
        Ok(guest_ptr)
    }
    fn write_to_guest(&self, location: &GuestPtrMut<'a, Self>) {
        // location is guaranteed to be in GuestMemory and aligned to 4
        unsafe {
            let raw_ptr: *mut u32 = location.as_raw() as *mut u32;
            raw_ptr.write(self.region.start);
        }
    }
}

#[derive(Clone)]
pub struct GuestPtrMut<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    type_: PhantomData<T>,
}

impl<'a, T: GuestType> GuestPtrMut<'a, T> {
    pub fn as_immut(&self) -> GuestPtr<'a, T> {
        GuestPtr {
            mem: self.mem,
            region: self.region,
            type_: self.type_,
        }
    }

    pub fn as_raw(&self) -> *const u8 {
        self.as_immut().as_raw()
    }
    pub fn elem(&self, elements: i32) -> Result<GuestPtrMut<'a, T>, GuestError> {
        self.mem
            .ptr_mut(self.region.start + (elements * self.region.len as i32) as u32)
    }

    pub fn cast<TT: GuestType>(&self, offset: u32) -> Result<GuestPtrMut<'a, TT>, GuestError> {
        self.mem.ptr_mut(self.region.start + offset)
    }
}

impl<'a, T: GuestTypeCopy> GuestPtrMut<'a, T> {
    pub fn as_ref(&self) -> Result<GuestRef<'a, T>, GuestError> {
        self.as_immut().as_ref()
    }

    pub fn as_ref_mut(&self) -> Result<GuestRefMut<'a, T>, GuestError> {
        T::validate(&self.as_immut())?;
        let handle = {
            let mut borrows = self.mem.borrows.borrow_mut();
            borrows
                .borrow_mut(self.region)
                .ok_or_else(|| GuestError::PtrBorrowed(self.region))?
        };
        Ok(GuestRefMut {
            mem: self.mem,
            region: self.region,
            handle,
            type_: self.type_,
        })
    }
}

impl<'a, T: GuestTypePtr<'a>> GuestPtrMut<'a, T> {
    pub fn read_ptr_from_guest(&self) -> Result<T, GuestError> {
        T::read_from_guest(&self.as_immut())
    }
    pub fn write_ptr_to_guest(&self, ptr: &T) {
        T::write_to_guest(ptr, &self);
    }
}

impl<'a, T: GuestTypeClone> GuestPtrMut<'a, T> {
    pub fn clone_from_guest(&self) -> Result<T, GuestError> {
        T::read_from_guest(&self.as_immut())
    }

    pub fn clone_to_guest(&self, val: &T) {
        T::write_to_guest(val, &self)
    }
}

impl<'a, T> GuestType for GuestPtrMut<'a, T>
where
    T: GuestType,
{
    fn size() -> u32 {
        4
    }
    fn align() -> u32 {
        4
    }
    fn name() -> String {
        format!("GuestPtrMut<{}>", T::name())
    }
    fn validate<'b>(location: &GuestPtr<'b, GuestPtrMut<'b, T>>) -> Result<(), GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let _guest_ptr: GuestPtr<T> = location.mem.ptr(raw_ptr)?;
        Ok(())
    }
}

// Reading and writing GuestPtrMuts to memory:
impl<'a, T> GuestTypePtr<'a> for GuestPtrMut<'a, T>
where
    T: GuestType,
{
    fn read_from_guest(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let guest_ptr_mut: GuestPtrMut<'a, T> = location.mem.ptr_mut(raw_ptr)?;
        Ok(guest_ptr_mut)
    }
    fn write_to_guest(&self, location: &GuestPtrMut<'a, Self>) {
        // location is guaranteed to be in GuestMemory and aligned to 4
        unsafe {
            let raw_ptr: *mut u32 = location.as_raw() as *mut u32;
            raw_ptr.write(self.region.start);
        }
    }
}

pub struct GuestRef<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    handle: BorrowHandle,
    type_: PhantomData<T>,
}

impl<'a, T> GuestRef<'a, T> {
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        GuestPtr {
            mem: self.mem,
            region: self.region,
            type_: self.type_,
        }
    }
}

impl<'a, T> ::std::ops::Deref for GuestRef<'a, T>
where
    T: GuestTypeCopy,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            ((self.mem.ptr as usize + self.region.start as usize) as *const T)
                .as_ref()
                .expect("GuestRef implies non-null")
        }
    }
}

impl<'a, T> Drop for GuestRef<'a, T> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_immut(self.handle);
    }
}

pub struct GuestRefMut<'a, T> {
    mem: &'a GuestMemory<'a>,
    region: Region,
    handle: BorrowHandle,
    type_: PhantomData<T>,
}

impl<'a, T> GuestRefMut<'a, T> {
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        GuestPtr {
            mem: self.mem,
            region: self.region,
            type_: self.type_,
        }
    }
    pub fn as_ptr_mut(&self) -> GuestPtrMut<'a, T> {
        GuestPtrMut {
            mem: self.mem,
            region: self.region,
            type_: self.type_,
        }
    }
}

impl<'a, T> ::std::ops::Deref for GuestRefMut<'a, T>
where
    T: GuestTypeCopy,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            ((self.mem.ptr as usize + self.region.start as usize) as *const T)
                .as_ref()
                .expect("GuestRef implies non-null")
        }
    }
}

impl<'a, T> ::std::ops::DerefMut for GuestRefMut<'a, T>
where
    T: GuestTypeCopy,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            ((self.mem.ptr as usize + self.region.start as usize) as *mut T)
                .as_mut()
                .expect("GuestRef implies non-null")
        }
    }
}

impl<'a, T> Drop for GuestRefMut<'a, T> {
    fn drop(&mut self) {
        let mut borrows = self.mem.borrows.borrow_mut();
        borrows.unborrow_mut(self.handle);
    }
}
