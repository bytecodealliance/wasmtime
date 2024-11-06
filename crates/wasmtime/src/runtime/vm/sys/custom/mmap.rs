use super::cvt;
use crate::prelude::*;
use crate::runtime::vm::sys::capi;
use crate::runtime::vm::SendSyncPtr;
use core::ops::Range;
use core::ptr::{self, NonNull};
#[cfg(feature = "std")]
use std::{fs::File, path::Path};

#[cfg(feature = "std")]
pub fn open_file_for_mmap(_path: &Path) -> Result<File> {
    anyhow::bail!("not supported on this platform");
}

#[derive(Debug)]
pub struct Mmap {
    memory: SendSyncPtr<[u8]>,
}

impl Mmap {
    pub fn new_empty() -> Mmap {
        Mmap {
            memory: crate::vm::sys::empty_mmap(),
        }
    }

    pub fn new(size: usize) -> Result<Self> {
        let mut ptr = ptr::null_mut();
        cvt(unsafe {
            capi::wasmtime_mmap_new(size, capi::PROT_READ | capi::PROT_WRITE, &mut ptr)
        })?;
        let memory = ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Mmap { memory })
    }

    pub fn reserve(size: usize) -> Result<Self> {
        let mut ptr = ptr::null_mut();
        cvt(unsafe { capi::wasmtime_mmap_new(size, 0, &mut ptr) })?;
        let memory = ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Mmap { memory })
    }

    #[cfg(feature = "std")]
    pub fn from_file(_file: &File) -> Result<Self> {
        anyhow::bail!("not supported on this platform");
    }

    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        let ptr = self.memory.as_ptr();
        unsafe {
            cvt(capi::wasmtime_mprotect(
                ptr.byte_add(start).cast(),
                len,
                capi::PROT_READ | capi::PROT_WRITE,
            ))?;
        }

        Ok(())
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.memory.as_ptr() as *const u8
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.memory.as_ptr().cast()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.memory.as_ptr().len()
    }

    pub unsafe fn make_executable(
        &self,
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        let base = self.memory.as_ptr().byte_add(range.start).cast();
        let len = range.end - range.start;

        // not mapped into the C API at this time.
        let _ = enable_branch_protection;

        cvt(capi::wasmtime_mprotect(
            base,
            len,
            capi::PROT_READ | capi::PROT_EXEC,
        ))?;
        Ok(())
    }

    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        let base = self.memory.as_ptr().byte_add(range.start).cast();
        let len = range.end - range.start;

        cvt(capi::wasmtime_mprotect(base, len, capi::PROT_READ))?;
        Ok(())
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.memory.as_ptr().cast();
            let len = self.memory.as_ptr().len();
            if len == 0 {
                return;
            }
            cvt(capi::wasmtime_munmap(ptr, len)).unwrap();
        }
    }
}
