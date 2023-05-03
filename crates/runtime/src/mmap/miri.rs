//! A "dummy" implementation of mmaps for miri where "we do the best we can"
//!
//! Namely this uses `alloc` to allocate memory for the "mmap" specifically to
//! create page-aligned allocations. This allocation doesn't handle oeprations
//! like becoming executable or becoming readonly or being created from files,
//! but it's enough to get various tests running relying on memories and such.

use anyhow::{bail, Result};
use std::alloc::{self, Layout};
use std::fs::File;
use std::ops::Range;
use std::path::Path;

#[derive(Debug)]
pub struct Mmap {
    memory: *mut [u8],
}

unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    pub fn new_empty() -> Mmap {
        Mmap { memory: &mut [] }
    }

    pub fn new(size: usize) -> Result<Self> {
        let mut ret = Mmap::reserve(size)?;
        ret.make_accessible(0, size)?;
        Ok(ret)
    }

    pub fn reserve(size: usize) -> Result<Self> {
        let layout = Layout::from_size_align(size, crate::page_size()).unwrap();
        let ptr = unsafe { alloc::alloc(layout) };
        if ptr.is_null() {
            bail!("failed to allocate memory");
        }

        Ok(Mmap {
            memory: std::ptr::slice_from_raw_parts_mut(ptr, size),
        })
    }

    pub fn from_file(_path: &Path) -> Result<(Self, File)> {
        bail!("not supported on miri");
    }

    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        // The memory is technically always accessible but this marks it as
        // initialized for miri-level checking.
        unsafe {
            std::ptr::write_bytes(self.memory.cast::<u8>().add(start), 0u8, len);
        }
        Ok(())
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.memory as *const u8
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.memory.cast()
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.memory).len() }
    }

    pub unsafe fn make_executable(
        &self,
        _range: Range<usize>,
        _enable_branch_protection: bool,
    ) -> Result<()> {
        Ok(())
    }

    pub unsafe fn make_readonly(&self, _range: Range<usize>) -> Result<()> {
        Ok(())
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        if self.len() == 0 {
            return;
        }
        unsafe {
            let layout = Layout::from_size_align(self.len(), crate::page_size()).unwrap();
            alloc::dealloc(self.as_mut_ptr(), layout);
        }
    }
}
