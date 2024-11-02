use crate::prelude::*;
use crate::runtime::vm::SendSyncPtr;
use rustix::mm::{mprotect, MprotectFlags};
use std::ops::Range;
use std::ptr::{self, NonNull};
#[cfg(feature = "std")]
use std::{fs::File, path::Path};

#[derive(Debug)]
pub struct Mmap {
    memory: SendSyncPtr<[u8]>,
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "illumos", target_os = "linux"))] {
        // On illumos, by default, mmap reserves what it calls "swap space" ahead of time, so that
        // memory accesses are guaranteed not to fail once mmap succeeds. NORESERVE is for cases
        // where that memory is never meant to be accessed -- e.g. memory that's used as guard
        // pages.
        //
        // This is less crucial on Linux because Linux tends to overcommit memory by default, but is
        // still a good idea to pass in for large allocations that don't need to be backed by
        // physical memory.
        pub(super) const MMAP_NORESERVE_FLAG: rustix::mm::MapFlags =
            rustix::mm::MapFlags::NORESERVE;
    } else {
        pub(super) const MMAP_NORESERVE_FLAG: rustix::mm::MapFlags = rustix::mm::MapFlags::empty();
    }
}

impl Mmap {
    pub fn new_empty() -> Mmap {
        Mmap {
            memory: crate::vm::sys::empty_mmap(),
        }
    }

    pub fn new(size: usize) -> Result<Self> {
        let ptr = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE | MMAP_NORESERVE_FLAG,
            )
            .err2anyhow()?
        };
        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Mmap { memory })
    }

    pub fn reserve(size: usize) -> Result<Self> {
        let ptr = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::empty(),
                // Astute readers might be wondering why a function called "reserve" passes in a
                // NORESERVE flag. That's because "reserve" in this context means one of two
                // different things.
                //
                // * This method is used to allocate virtual memory that starts off in a state where
                //   it cannot be accessed (i.e. causes a segfault if accessed).
                // * NORESERVE is meant for virtual memory space for which backing physical/swap
                //   pages are reserved on first access.
                //
                // Virtual memory that cannot be accessed should not have a backing store reserved
                // for it. Hence, passing in NORESERVE is correct here.
                rustix::mm::MapFlags::PRIVATE | MMAP_NORESERVE_FLAG,
            )
            .err2anyhow()?
        };

        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Mmap { memory })
    }

    #[cfg(feature = "std")]
    pub fn from_file(path: &Path) -> Result<(Self, File)> {
        let file = File::open(path)
            .err2anyhow()
            .context("failed to open file")?;
        let len = file
            .metadata()
            .err2anyhow()
            .context("failed to get file metadata")?
            .len();
        let len = usize::try_from(len).map_err(|_| anyhow::anyhow!("file too large to map"))?;
        let ptr = unsafe {
            rustix::mm::mmap(
                ptr::null_mut(),
                len,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
                &file,
                0,
            )
            .err2anyhow()
            .context(format!("mmap failed to allocate {len:#x} bytes"))?
        };
        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), len);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());

        Ok((Mmap { memory }, file))
    }

    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        let ptr = self.memory.as_ptr();
        unsafe {
            mprotect(
                ptr.byte_add(start).cast(),
                len,
                MprotectFlags::READ | MprotectFlags::WRITE,
            )
            .err2anyhow()?;
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

        let flags = MprotectFlags::READ | MprotectFlags::EXEC;
        let flags = if enable_branch_protection {
            #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
            if std::arch::is_aarch64_feature_detected!("bti") {
                MprotectFlags::from_bits_retain(flags.bits() | /* PROT_BTI */ 0x10)
            } else {
                flags
            }

            #[cfg(not(all(target_arch = "aarch64", target_os = "linux")))]
            flags
        } else {
            flags
        };

        mprotect(base, len, flags).err2anyhow()?;

        Ok(())
    }

    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        let base = self.memory.as_ptr().byte_add(range.start).cast();
        let len = range.end - range.start;

        mprotect(base, len, MprotectFlags::READ).err2anyhow()?;

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
            rustix::mm::munmap(ptr, len).expect("munmap failed");
        }
    }
}
