use super::ptr::{GuestPtr, GuestRef};
use crate::{GuestError, GuestType, GuestTypeCopy};
use std::{fmt, ops::Deref};

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
    T: GuestType,
{
    pub fn iter(&self) -> GuestArrayIter<'a, T> {
        let next = GuestPtr {
            mem: self.ptr.mem,
            region: self.ptr.region,
            type_: self.ptr.type_,
        };
        GuestArrayIter {
            next,
            num_elems: self.num_elems,
            count: 0,
        }
    }
}

pub struct GuestArrayIter<'a, T>
where
    T: GuestType,
{
    next: GuestPtr<'a, T>,
    num_elems: u32,
    count: u32,
}

impl<'a, T> Iterator for GuestArrayIter<'a, T>
where
    T: GuestType,
{
    type Item = Result<GuestPtr<'a, T>, GuestError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.num_elems {
            // ok...
            Some(T::validate(&self.next).and_then(|()| {
                let curr = GuestPtr {
                    mem: self.next.mem,
                    region: self.next.region,
                    type_: self.next.type_,
                };
                self.next = self.next.elem(1)?;
                self.count += 1;
                Ok(curr)
            }))
        } else {
            // no more elements...
            None
        }
    }
}

impl<'a, T> GuestArray<'a, T>
where
    T: GuestTypeCopy,
{
    pub fn as_ref(&self) -> Result<GuestArrayRef<'a, T>, GuestError> {
        let mut next = self.ptr.elem(0)?;
        for _ in 0..self.num_elems {
            T::validate(&next)?;
            next = next.elem(1)?;
        }
        let region = self.ptr.region.extend(self.num_elems);
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
    T: GuestTypeCopy,
{
    ref_: GuestRef<'a, T>,
    num_elems: u32,
}

impl<'a, T> fmt::Debug for GuestArrayRef<'a, T>
where
    T: GuestTypeCopy + fmt::Debug,
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
                self.ref_.as_ptr().as_raw() as *const _,
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
        // try extracting an array out of memory bounds
        let ptr: GuestPtr<i32> = guest_memory.ptr(4092).expect("ptr to last i32 el");
        let err = ptr.array(2).expect_err("out of bounds ptr error");
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
        let as_ref = &*arr.as_ref().expect("array borrowed immutably");
        assert_eq!(as_ref, &[1, 2, 3]);
        // borrowing again should be fine
        let as_ref2 = &*arr.as_ref().expect("array borrowed immutably again");
        assert_eq!(as_ref2, as_ref);
    }

    #[test]
    fn ptr_to_ptr_array() {
        let mut host_memory = HostMemory::new();
        let guest_memory = GuestMemory::new(host_memory.as_mut_ptr(), host_memory.len() as u32);
        {
            let val_ptr: GuestPtrMut<u8> =
                guest_memory.ptr_mut(0).expect("ptr mut to the first value");
            let mut val = val_ptr.as_ref_mut().expect("ref mut to the first value");
            *val = 255;
            let val_ptr: GuestPtrMut<u8> = guest_memory
                .ptr_mut(4)
                .expect("ptr mut to the second value");
            let mut val = val_ptr.as_ref_mut().expect("ref mut to the second value");
            *val = 254;
            let val_ptr: GuestPtrMut<u8> =
                guest_memory.ptr_mut(8).expect("ptr mut to the third value");
            let mut val = val_ptr.as_ref_mut().expect("ref mut to the third value");
            *val = 253;
        }
        {
            let ptr = guest_memory.ptr_mut(12).expect("ptr mut to first el");
            ptr.write_ptr_to_guest(
                &guest_memory
                    .ptr::<GuestPtr<u8>>(0)
                    .expect("ptr to the first value"),
            );
            let ptr = guest_memory.ptr_mut(16).expect("ptr mut to first el");
            ptr.write_ptr_to_guest(
                &guest_memory
                    .ptr::<GuestPtr<u8>>(4)
                    .expect("ptr to the second value"),
            );
            let ptr = guest_memory.ptr_mut(20).expect("ptr mut to first el");
            ptr.write_ptr_to_guest(
                &guest_memory
                    .ptr::<GuestPtr<u8>>(8)
                    .expect("ptr to the third value"),
            );
        }
        // extract as array
        let ptr: GuestPtr<GuestPtr<u8>> = guest_memory.ptr(12).expect("ptr to first el");
        let arr = ptr.array(3).expect("convert ptr to array");
        let contents = arr
            .iter()
            .map(|ptr_ptr| {
                *ptr_ptr
                    .expect("valid ptr to ptr")
                    .read_ptr_from_guest()
                    .expect("valid ptr to some value")
                    .as_ref()
                    .expect("deref ptr to some value")
            })
            .collect::<Vec<_>>();
        assert_eq!(&contents, &[255, 254, 253]);
    }
}
