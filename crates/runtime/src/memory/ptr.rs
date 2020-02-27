use super::{array::GuestArray, GuestMemory};
use crate::{borrow::BorrowHandle, GuestError, GuestType, GuestTypeTransparent, Region};
use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct GuestPtr<'a, T> {
    pub(super) mem: &'a GuestMemory<'a>,
    pub(super) region: Region,
    pub(super) type_: PhantomData<T>,
}

impl<'a, T> fmt::Debug for GuestPtr<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestPtr {{ mem: {:?}, region: {:?} }}",
            self.mem, self.region
        )
    }
}

impl<'a, T: GuestType<'a>> GuestPtr<'a, T> {
    pub fn as_raw(&self) -> *const u8 {
        (self.mem.ptr as usize + self.region.start as usize) as *const u8
    }

    pub fn elem(&self, elements: i32) -> Result<Self, GuestError> {
        self.mem
            .ptr(self.region.start + (elements * self.region.len as i32) as u32)
    }

    pub fn cast<CastTo: GuestType<'a>>(
        &self,
        offset: u32,
    ) -> Result<GuestPtr<'a, CastTo>, GuestError> {
        self.mem.ptr(self.region.start + offset)
    }

    pub fn array(&self, num_elems: u32) -> Result<GuestArray<'a, T>, GuestError> {
        let region = self.region.extend(num_elems);
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

impl<'a, T> GuestPtr<'a, T>
where
    T: GuestTypeTransparent<'a>,
{
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

impl<'a, T> GuestPtr<'a, T>
where
    T: GuestType<'a>,
{
    pub fn read(&self) -> Result<T, GuestError> {
        T::read(self)
    }
}

impl<'a, T> GuestType<'a> for GuestPtr<'a, T>
where
    T: GuestType<'a>,
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

    fn validate(location: &GuestPtr<'a, GuestPtr<'a, T>>) -> Result<(), GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let _guest_ptr: GuestPtr<T> = location.mem.ptr(raw_ptr)?;
        Ok(())
    }

    // Operations for reading and writing Ptrs to memory:
    fn read(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let guest_ptr: GuestPtr<'a, T> = location.mem.ptr(raw_ptr)?;
        Ok(guest_ptr)
    }

    fn write(&self, location: &GuestPtrMut<'a, Self>) {
        // location is guaranteed to be in GuestMemory and aligned to 4
        unsafe {
            let raw_ptr: *mut u32 = location.as_raw() as *mut u32;
            raw_ptr.write(self.region.start);
        }
    }
}

#[derive(Clone)]
pub struct GuestPtrMut<'a, T> {
    pub(super) mem: &'a GuestMemory<'a>,
    pub(super) region: Region,
    pub(super) type_: PhantomData<T>,
}

impl<'a, T> fmt::Debug for GuestPtrMut<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GuestPtrMut {{ mem: {:?}, region: {:?} }}",
            self.mem, self.region
        )
    }
}

impl<'a, T> GuestPtrMut<'a, T>
where
    T: GuestType<'a>,
{
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

    pub fn elem(&self, elements: i32) -> Result<Self, GuestError> {
        self.mem
            .ptr_mut(self.region.start + (elements * self.region.len as i32) as u32)
    }

    pub fn cast<CastTo: GuestType<'a>>(
        &self,
        offset: u32,
    ) -> Result<GuestPtrMut<'a, CastTo>, GuestError> {
        self.mem.ptr_mut(self.region.start + offset)
    }
}

impl<'a, T> GuestPtrMut<'a, T>
where
    T: GuestTypeTransparent<'a>,
{
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

impl<'a, T> GuestPtrMut<'a, T>
where
    T: GuestType<'a>,
{
    pub fn read(&self) -> Result<T, GuestError> {
        T::read(&self.as_immut())
    }

    pub fn write(&self, ptr: &T) {
        T::write(ptr, &self);
    }
}

impl<'a, T> GuestType<'a> for GuestPtrMut<'a, T>
where
    T: GuestType<'a>,
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

    fn validate(location: &GuestPtr<'a, GuestPtrMut<'a, T>>) -> Result<(), GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let _guest_ptr: GuestPtr<T> = location.mem.ptr(raw_ptr)?;
        Ok(())
    }

    // Reading and writing GuestPtrMuts to memory:
    fn read(location: &GuestPtr<'a, Self>) -> Result<Self, GuestError> {
        // location is guaranteed to be in GuestMemory and aligned to 4
        let raw_ptr: u32 = unsafe { *(location.as_raw() as *const u32) };
        // GuestMemory can validate that the raw pointer contents are legal for T:
        let guest_ptr_mut: GuestPtrMut<'a, T> = location.mem.ptr_mut(raw_ptr)?;
        Ok(guest_ptr_mut)
    }

    fn write(&self, location: &GuestPtrMut<'a, Self>) {
        // location is guaranteed to be in GuestMemory and aligned to 4
        unsafe {
            let raw_ptr: *mut u32 = location.as_raw() as *mut u32;
            raw_ptr.write(self.region.start);
        }
    }
}

pub struct GuestRef<'a, T> {
    pub(super) mem: &'a GuestMemory<'a>,
    pub(super) region: Region,
    pub(super) handle: BorrowHandle,
    pub(super) type_: PhantomData<T>,
}

impl<'a, T> fmt::Debug for GuestRef<'a, T>
where
    T: fmt::Debug,
{
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

impl<'a, T> Deref for GuestRef<'a, T>
where
    T: GuestTypeTransparent<'a>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
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
    pub(super) mem: &'a GuestMemory<'a>,
    pub(super) region: Region,
    pub(super) handle: BorrowHandle,
    pub(super) type_: PhantomData<T>,
}

impl<'a, T> fmt::Debug for GuestRefMut<'a, T>
where
    T: fmt::Debug,
{
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
    T: GuestTypeTransparent<'a>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            ((self.mem.ptr as usize + self.region.start as usize) as *const T)
                .as_ref()
                .expect("GuestRef implies non-null")
        }
    }
}

impl<'a, T> DerefMut for GuestRefMut<'a, T>
where
    T: GuestTypeTransparent<'a>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
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

#[cfg(test)]
mod test {
    use super::{
        super::{GuestError, GuestMemory, Region},
        {GuestPtr, GuestPtrMut},
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
        // try extracting an immutable ptr out of memory bounds
        let err = guest_memory
            .ptr::<GuestPtr<i32>>(4096)
            .expect_err("out of bounds ptr error");
        assert_eq!(err, GuestError::PtrOutOfBounds(Region::new(4096, 4)));
        // try extracting an mutable ptr out of memory bounds
        let err = guest_memory
            .ptr_mut::<GuestPtrMut<i32>>(4096)
            .expect_err("out of bounds ptr error");
        assert_eq!(err, GuestError::PtrOutOfBounds(Region::new(4096, 4)));
    }

    #[test]
    fn not_aligned() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // try extracting a misaligned immutable ptr
        let err = guest_memory
            .ptr::<GuestPtr<i32>>(2)
            .expect_err("ptr misaligned");
        assert_eq!(err, GuestError::PtrNotAligned(Region::new(2, 4), 4));
        // try extracting a misaligned mutable ptr
        let err = guest_memory
            .ptr_mut::<GuestPtrMut<i32>>(2)
            .expect_err("ptr mut misaligned");
        assert_eq!(err, GuestError::PtrNotAligned(Region::new(2, 4), 4));
    }

    #[test]
    fn ptr_from_memory() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // write something to memory
        {
            let ptr: GuestPtrMut<i64> = guest_memory.ptr_mut(8).expect("ptr mut to the el");
            let mut el = ptr.as_ref_mut().expect("ref mut to the el");
            *el = 100;
        }
        // extract as ref
        let ptr: GuestPtr<i64> = guest_memory.ptr(8).expect("ptr to the el");
        let as_ref = ptr.as_ref().expect("el borrowed immutably");
        assert_eq!(*as_ref, 100);
        // borrowing again should be fine
        let as_ref2 = ptr.as_ref().expect("el borrowed immutably again");
        assert_eq!(*as_ref2, *as_ref);
    }

    #[test]
    fn ptr_mut_from_memory() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        // set elems of array to zero
        {
            let ptr: GuestPtrMut<i64> = guest_memory.ptr_mut(8).expect("ptr mut to the el");
            let mut el = ptr.as_ref_mut().expect("ref mut to the el");
            *el = 100;
        }
        // extract as ref
        let ptr: GuestPtrMut<i64> = guest_memory.ptr_mut(8).expect("ptr mut to the el");
        assert_eq!(*ptr.as_ref().expect("el borrowed immutably"), 100);
        // overwrite the memory and re-verify
        *ptr.as_ref_mut().expect("el borrowed mutably") = 2000;
        // re-validate
        assert_eq!(*ptr.as_ref().expect("el borrowed immutably"), 2000);
    }

    #[test]
    #[should_panic(
        expected = "el borrowed immutably while borrowed mutably: PtrBorrowed(Region { start: 0, len: 2 })"
    )]
    fn borrow_mut_then_immut() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i16> = guest_memory.ptr_mut(0).expect("ptr mut to the el");
        // borrow mutably
        let _as_mut = ptr
            .as_ref_mut()
            .expect("el borrowed mutably for the first time");
        // borrow immutably should fail
        let _as_ref = ptr
            .as_ref()
            .expect("el borrowed immutably while borrowed mutably");
    }

    #[test]
    #[should_panic(
        expected = "el borrowed mutably while borrowed mutably: PtrBorrowed(Region { start: 0, len: 2 })"
    )]
    fn borrow_mut_twice() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        let ptr: GuestPtrMut<i16> = guest_memory.ptr_mut(0).expect("ptr mut to the el");
        // borrow mutably
        let _as_mut = ptr
            .as_ref_mut()
            .expect("el borrowed mutably for the first time");
        // try borrowing mutably again
        let _as_mut2 = ptr
            .as_ref_mut()
            .expect("el borrowed mutably while borrowed mutably");
    }
}
