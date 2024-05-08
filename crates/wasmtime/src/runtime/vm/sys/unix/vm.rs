use rustix::fd::AsRawFd;
use rustix::mm::{mmap, mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};
use std::fs::File;
use std::io;
#[cfg(feature = "std")]
use std::sync::Arc;

pub unsafe fn expose_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    mprotect(ptr.cast(), len, MprotectFlags::READ | MprotectFlags::WRITE)?;
    Ok(())
}

pub unsafe fn hide_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    mprotect(ptr.cast(), len, MprotectFlags::empty())?;
    Ok(())
}

pub unsafe fn erase_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    let ret = mmap_anonymous(
        ptr.cast(),
        len,
        ProtFlags::empty(),
        MapFlags::PRIVATE | MapFlags::FIXED,
    )?;
    assert_eq!(ptr, ret.cast());
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn decommit_pages(addr: *mut u8, len: usize) -> io::Result<()> {
    if len == 0 {
        return Ok(());
    }

    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                use rustix::mm::{madvise, Advice};

                // On Linux, this is enough to cause the kernel to initialize
                // the pages to 0 on next access
                madvise(addr as _, len, Advice::LinuxDontNeed)?;
            } else {
                // By creating a new mapping at the same location, this will
                // discard the mapping for the pages in the given range.
                // The new mapping will be to the CoW zero page, so this
                // effectively zeroes the pages.
                mmap_anonymous(
                    addr as _,
                    len,
                    ProtFlags::READ | ProtFlags::WRITE,
                    MapFlags::PRIVATE | MapFlags::FIXED,
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn commit_pages(_addr: *mut u8, _len: usize) -> io::Result<()> {
    // Pages are always READ | WRITE so there's nothing that needs to be done
    // here.
    Ok(())
}

pub fn get_page_size() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE).try_into().unwrap() }
}

pub fn supports_madvise_dontneed() -> bool {
    cfg!(target_os = "linux")
}

pub unsafe fn madvise_dontneed(ptr: *mut u8, len: usize) -> io::Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            rustix::mm::madvise(ptr.cast(), len, rustix::mm::Advice::LinuxDontNeed)?;
            Ok(())
        } else {
            let _ = (ptr, len);
            unreachable!();
        }
    }
}

#[derive(Debug)]
pub enum MemoryImageSource {
    #[cfg(feature = "std")]
    Mmap(Arc<File>),
    #[cfg(target_os = "linux")]
    Memfd(memfd::Memfd),
}

impl MemoryImageSource {
    #[cfg(feature = "std")]
    pub fn from_file(file: &Arc<File>) -> Option<MemoryImageSource> {
        Some(MemoryImageSource::Mmap(file.clone()))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn from_data(_data: &[u8]) -> io::Result<Option<MemoryImageSource>> {
        Ok(None)
    }

    #[cfg(target_os = "linux")]
    pub fn from_data(data: &[u8]) -> anyhow::Result<Option<MemoryImageSource>> {
        // On Linux `memfd_create` is used to create an anonymous
        // in-memory file to represent the heap image. This anonymous
        // file is then used as the basis for further mmaps.

        use crate::prelude::*;
        use std::io::{ErrorKind, Write};

        // Create the memfd. It needs a name, but the documentation for
        // `memfd_create()` says that names can be duplicated with no issues.
        let memfd = match memfd::MemfdOptions::new()
            .allow_sealing(true)
            .create("wasm-memory-image")
        {
            Ok(memfd) => memfd,
            // If this kernel is old enough to not support memfd then attempt to
            // gracefully handle that and fall back to skipping the memfd
            // optimization.
            Err(memfd::Error::Create(err)) if err.kind() == ErrorKind::Unsupported => {
                return Ok(None)
            }
            Err(e) => return Err(e.into_anyhow()),
        };
        memfd.as_file().write_all(data).err2anyhow()?;

        // Seal the memfd's data and length.
        //
        // This is a defense-in-depth security mitigation. The
        // memfd will serve as the starting point for the heap of
        // every instance of this module. If anything were to
        // write to this, it could affect every execution. The
        // memfd object itself is owned by the machinery here and
        // not exposed elsewhere, but it is still an ambient open
        // file descriptor at the syscall level, so some other
        // vulnerability that allowed writes to arbitrary fds
        // could modify it. Or we could have some issue with the
        // way that we map it into each instance. To be
        // extra-super-sure that it never changes, and because
        // this costs very little, we use the kernel's "seal" API
        // to make the memfd image permanently read-only.
        memfd
            .add_seals(&[
                memfd::FileSeal::SealGrow,
                memfd::FileSeal::SealShrink,
                memfd::FileSeal::SealWrite,
                memfd::FileSeal::SealSeal,
            ])
            .err2anyhow()?;

        Ok(Some(MemoryImageSource::Memfd(memfd)))
    }

    fn as_file(&self) -> &File {
        match *self {
            #[cfg(feature = "std")]
            MemoryImageSource::Mmap(ref file) => file,
            #[cfg(target_os = "linux")]
            MemoryImageSource::Memfd(ref memfd) => memfd.as_file(),
        }
    }

    pub unsafe fn map_at(&self, base: *mut u8, len: usize, offset: u64) -> io::Result<()> {
        let ptr = mmap(
            base.cast(),
            len,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE | MapFlags::FIXED,
            self.as_file(),
            offset,
        )?;
        assert_eq!(base, ptr.cast());
        Ok(())
    }

    pub unsafe fn remap_as_zeros_at(&self, base: *mut u8, len: usize) -> io::Result<()> {
        let ptr = mmap_anonymous(
            base.cast(),
            len,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE | MapFlags::FIXED,
        )?;
        assert_eq!(base, ptr.cast());
        Ok(())
    }
}

impl PartialEq for MemoryImageSource {
    fn eq(&self, other: &MemoryImageSource) -> bool {
        self.as_file().as_raw_fd() == other.as_file().as_raw_fd()
    }
}
