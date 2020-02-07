use std::cell::RefCell;
use std::fmt;
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

impl<'a, T: fmt::Debug> fmt::Debug for GuestPtr<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestPtr {{ mem: {:?}, region: {:?} }}",
            self.mem, self.region
        )
    }
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

    pub fn array(&self, num_elems: u32) -> Result<GuestArray<'a, T>, GuestError> {
        let region = self.region.extend((num_elems - 1) * T::size());
        if self.mem.contains(region) {
            let ptr = GuestPtr {
                mem: self.mem,
                region: self.region,
                type_: self.type_,
            };
            Ok(GuestArray { ptr, num_elems })
        } else {
            Err(GuestError::PtrOutOfBounds(region))
        }
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

impl<'a, T: fmt::Debug> fmt::Debug for GuestPtrMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestPtrMut {{ mem: {:?}, region: {:?} }}",
            self.mem, self.region
        )
    }
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

    pub fn array_mut(&self, num_elems: u32) -> Result<GuestArrayMut<'a, T>, GuestError> {
        let region = self.region.extend((num_elems - 1) * T::size());
        if self.mem.contains(region) {
            let ptr = GuestPtrMut {
                mem: self.mem,
                region: self.region,
                type_: self.type_,
            };
            Ok(GuestArrayMut { ptr, num_elems })
        } else {
            Err(GuestError::PtrOutOfBounds(region))
        }
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

impl<'a, T: fmt::Debug> fmt::Debug for GuestRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestRef {{ mem: {:?}, region: {:?}, handle: {:?} }}",
            self.mem, self.region, self.handle
        )
    }
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

impl<'a, T: fmt::Debug> fmt::Debug for GuestRefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestRefMut {{ mem: {:?}, region: {:?}, handle: {:?} }}",
            self.mem, self.region, self.handle
        )
    }
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

pub struct GuestArray<'a, T>
where
    T: GuestType,
{
    ptr: GuestPtr<'a, T>,
    num_elems: u32,
}

impl<'a, T: GuestType + fmt::Debug> fmt::Debug for GuestArray<'a, T> {
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

impl<'a, T> ::std::ops::Deref for GuestArrayRef<'a, T>
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
    ptr: GuestPtrMut<'a, T>,
    num_elems: u32,
}

impl<'a, T: GuestType + fmt::Debug> fmt::Debug for GuestArrayMut<'a, T> {
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

impl<'a, T> ::std::ops::Deref for GuestArrayRefMut<'a, T>
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

impl<'a, T> ::std::ops::DerefMut for GuestArrayRefMut<'a, T>
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
    use super::*;

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
    fn guest_array_out_of_bounds() {
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
    fn guest_array() {
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
    fn guest_array_mut() {
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
    fn guest_array_mut_borrow_checker_1() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
        let arr = GuestArrayMut { ptr, num_elems: 3 };
        // borrow mutably
        let _as_mut = arr
            .as_ref_mut()
            .expect("array borrowed mutably for the first time");
        // borrow immutably should be fine
        let _as_ref = arr
            .as_ref()
            .expect("array borrowed immutably while borrowed mutably");
    }

    #[test]
    #[should_panic(
        expected = "array borrowed mutably while borrowed mutably: PtrBorrowed(Region { start: 0, len: 12 })"
    )]
    fn guest_array_mut_borrow_checker_2() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i32> = guest_memory.ptr_mut(0).expect("ptr mut to first el");
        let arr = GuestArrayMut { ptr, num_elems: 3 };
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
