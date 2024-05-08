use super::cvt;
use crate::runtime::vm::sys::capi;
use crate::runtime::vm::SendSyncPtr;
use anyhow::Result;
use core::ptr::{self, NonNull};
#[cfg(feature = "std")]
use std::{fs::File, sync::Arc};

pub unsafe fn expose_existing_mapping(ptr: *mut u8, len: usize) -> Result<()> {
    cvt(capi::wasmtime_mprotect(
        ptr.cast(),
        len,
        capi::PROT_READ | capi::PROT_WRITE,
    ))
}

pub unsafe fn hide_existing_mapping(ptr: *mut u8, len: usize) -> Result<()> {
    cvt(capi::wasmtime_mprotect(ptr.cast(), len, 0))
}

pub unsafe fn erase_existing_mapping(ptr: *mut u8, len: usize) -> Result<()> {
    cvt(capi::wasmtime_mmap_remap(ptr.cast(), len, 0))
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn commit_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // Pages are always READ | WRITE so there's nothing that needs to be
    // done here.
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn decommit_pages(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    cvt(capi::wasmtime_mmap_remap(
        addr,
        len,
        capi::PROT_READ | capi::PROT_WRITE,
    ))
}

pub fn get_page_size() -> usize {
    unsafe { capi::wasmtime_page_size() }
}

pub fn supports_madvise_dontneed() -> bool {
    false
}

pub unsafe fn madvise_dontneed(_ptr: *mut u8, _len: usize) -> Result<()> {
    unreachable!()
}

#[derive(PartialEq, Debug)]
pub struct MemoryImageSource {
    data: SendSyncPtr<capi::wasmtime_memory_image>,
}

impl MemoryImageSource {
    #[cfg(feature = "std")]
    pub fn from_file(_file: &Arc<File>) -> Option<MemoryImageSource> {
        None
    }

    pub fn from_data(data: &[u8]) -> Result<Option<MemoryImageSource>> {
        unsafe {
            let mut ptr = ptr::null_mut();
            cvt(capi::wasmtime_memory_image_new(
                data.as_ptr(),
                data.len(),
                &mut ptr,
            ))?;
            match NonNull::new(ptr) {
                Some(ptr) => Ok(Some(MemoryImageSource {
                    data: SendSyncPtr::new(ptr),
                })),
                None => Ok(None),
            }
        }
    }

    pub unsafe fn map_at(&self, base: *mut u8, len: usize, offset: u64) -> Result<()> {
        assert_eq!(offset, 0);
        cvt(capi::wasmtime_memory_image_map_at(
            self.data.as_ptr(),
            base,
            len,
        ))
    }

    pub unsafe fn remap_as_zeros_at(&self, base: *mut u8, len: usize) -> Result<()> {
        cvt(capi::wasmtime_mmap_remap(
            base.cast(),
            len,
            capi::PROT_READ | capi::PROT_WRITE,
        ))
    }
}

impl Drop for MemoryImageSource {
    fn drop(&mut self) {
        unsafe {
            capi::wasmtime_memory_image_free(self.data.as_ptr());
        }
    }
}
