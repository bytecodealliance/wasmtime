use anyhow::{anyhow, Context, Result};
use rustix::mm::{mprotect, MprotectFlags};
use std::fs::File;
use std::ops::Range;
use std::path::Path;
use std::ptr;

#[derive(Debug)]
pub struct Mmap {
    memory: *mut [u8],
}

// Mmaps are sendable and threadsafe, and otherwise fix the auto-traits on the
// `*mut [u8]` storage internally.
unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    pub fn new_empty() -> Mmap {
        Mmap { memory: &mut [] }
    }

    pub fn new(size: usize) -> Result<Self> {
        let ptr = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
            )?
        };
        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        Ok(Mmap { memory })
    }

    pub fn reserve(size: usize) -> Result<Self> {
        let ptr = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::empty(),
                rustix::mm::MapFlags::PRIVATE,
            )?
        };

        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        Ok(Mmap { memory })
    }

    pub fn from_file(path: &Path) -> Result<(Self, File)> {
        let file = File::open(path).context("failed to open file")?;
        let len = file
            .metadata()
            .context("failed to get file metadata")?
            .len();
        let len = usize::try_from(len).map_err(|_| anyhow!("file too large to map"))?;
        let ptr = unsafe {
            rustix::mm::mmap(
                ptr::null_mut(),
                len,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
                &file,
                0,
            )
            .context(format!("mmap failed to allocate {:#x} bytes", len))?
        };
        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), len);

        Ok((Mmap { memory }, file))
    }

    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        let ptr = self.memory.cast::<u8>();
        unsafe {
            mprotect(
                ptr.add(start).cast(),
                len,
                MprotectFlags::READ | MprotectFlags::WRITE,
            )?;
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
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        let base = self.memory.cast::<u8>().add(range.start).cast();
        let len = range.end - range.start;

        let flags = MprotectFlags::READ | MprotectFlags::EXEC;
        let flags = if enable_branch_protection {
            #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
            if std::arch::is_aarch64_feature_detected!("bti") {
                MprotectFlags::from_bits_unchecked(flags.bits() | /* PROT_BTI */ 0x10)
            } else {
                flags
            }

            #[cfg(not(all(target_arch = "aarch64", target_os = "linux")))]
            flags
        } else {
            flags
        };

        mprotect(base, len, flags)?;

        Ok(())
    }

    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        let base = self.memory.cast::<u8>().add(range.start).cast();
        let len = range.end - range.start;

        mprotect(base, len, MprotectFlags::READ)?;

        Ok(())
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.memory.cast();
            let len = (*self.memory).len();
            if len == 0 {
                return;
            }
            rustix::mm::munmap(ptr, len).expect("munmap failed");
        }
    }
}
