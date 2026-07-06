use crate::runtime::vm::sys::DecommitBehavior;
use rustix::fd::AsRawFd;
use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect};
use std::fs::File;
use std::io;
#[cfg(feature = "std")]
use std::sync::Arc;

pub use super::pagemap::{PageMap, reset_with_pagemap};

pub unsafe fn expose_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    unsafe {
        mprotect(ptr.cast(), len, MprotectFlags::READ | MprotectFlags::WRITE)?;
    }
    Ok(())
}

pub unsafe fn hide_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    unsafe {
        mprotect(ptr.cast(), len, MprotectFlags::empty())?;
    }
    Ok(())
}

pub unsafe fn erase_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    let ret = unsafe {
        mmap_anonymous(
            ptr.cast(),
            len,
            ProtFlags::empty(),
            MapFlags::PRIVATE | super::mmap::MMAP_NORESERVE_FLAG | MapFlags::FIXED,
        )?
    };
    assert_eq!(ptr, ret.cast());
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub use libc::iovec;

#[cfg(feature = "pooling-allocator")]
pub unsafe fn commit_pages(_addr: *mut u8, _len: usize) -> io::Result<()> {
    // Pages are always READ | WRITE so there's nothing that needs to be done
    // here.
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn decommit_pages(iov: &[iovec]) -> io::Result<()> {
    // Attempt to use `process_madvise` as it batches everything into a singl
    // syscall instead of requiring `madvise`-per-rgion like below. This is only
    // supported on Linux with (as of the time of this writing) a relatively
    // recent kernel.
    #[cfg(target_os = "linux")]
    unsafe {
        if iov.len() > 1 && process_madvise::run_self(iov, libc::MADV_DONTNEED, 0)? {
            return Ok(());
        }
    }
    for iov in iov {
        if iov.iov_len == 0 {
            continue;
        }

        unsafe {
            cfg_if::cfg_if! {
                if #[cfg(target_os = "linux")] {
                    use rustix::mm::{madvise, Advice};

                    // On Linux, this is enough to cause the kernel to initialize
                    // the pages to 0 on next access
                    madvise(iov.iov_base, iov.iov_len, Advice::LinuxDontNeed)?;
                } else {
                    // By creating a new mapping at the same location, this will
                    // discard the mapping for the pages in the given range.
                    // The new mapping will be to the CoW zero page, so this
                    // effectively zeroes the pages.
                    mmap_anonymous(
                        iov.iov_base,
                        iov.iov_len,
                        ProtFlags::READ | ProtFlags::WRITE,
                        MapFlags::PRIVATE | super::mmap::MMAP_NORESERVE_FLAG | MapFlags::FIXED,
                    )?;
                }
            }
        }
    }
    Ok(())
}

// NB: this function is duplicated in `crates/fiber/src/unix.rs` so if this
// changes that should probably get updated as well.
pub fn get_page_size() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE).try_into().unwrap() }
}

pub fn decommit_behavior() -> DecommitBehavior {
    if cfg!(target_os = "linux") {
        DecommitBehavior::RestoreOriginalMapping
    } else {
        DecommitBehavior::Zero
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
    pub fn from_data(data: &[u8]) -> crate::Result<Option<MemoryImageSource>> {
        // On Linux `memfd_create` is used to create an anonymous
        // in-memory file to represent the heap image. This anonymous
        // file is then used as the basis for further mmaps.

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
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        memfd.as_file().write_all(data)?;

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
        memfd.add_seals(&[
            memfd::FileSeal::SealGrow,
            memfd::FileSeal::SealShrink,
            memfd::FileSeal::SealWrite,
            memfd::FileSeal::SealSeal,
        ])?;

        Ok(Some(MemoryImageSource::Memfd(memfd)))
    }

    pub(super) fn as_file(&self) -> &File {
        match *self {
            #[cfg(feature = "std")]
            MemoryImageSource::Mmap(ref file) => file,
            #[cfg(target_os = "linux")]
            MemoryImageSource::Memfd(ref memfd) => memfd.as_file(),
        }
    }

    pub unsafe fn remap_as_zeros_at(&self, base: *mut u8, len: usize) -> io::Result<()> {
        let ptr = unsafe {
            mmap_anonymous(
                base.cast(),
                len,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::PRIVATE | super::mmap::MMAP_NORESERVE_FLAG | MapFlags::FIXED,
            )?
        };
        assert_eq!(base, ptr.cast());
        Ok(())
    }
}

impl PartialEq for MemoryImageSource {
    fn eq(&self, other: &MemoryImageSource) -> bool {
        self.as_file().as_raw_fd() == other.as_file().as_raw_fd()
    }
}

/// Wrapper around Linux's `process_madvise` syscall.
///
/// This module is a wrapper around the ability to use `process_madvise` as an
/// implementation detail of the `decommit_pages` function above. This is only
/// available on Linux and additionally has kernel requirements:
///
/// * `process_madvise` itself is available from Linux 5.10+
/// * `MADV_DONTNEED` on the self-process is only available in Linux 6.13+
/// * `PIDFD_SELF` on the self-process is only available in Linux 6.14+
///
/// This module uses `libc::syscall` to make the call to avoid glibc
/// requirements, and it additionally attempts to handle syscall failures to
/// indicate that this isn't supported at all (e.g. prior to 6.14).
///
/// # Why?
///
/// With `process_madvise` it's possible to inform the kernel all-at-once of a
/// list of regions to `MADV_DONTNEED`. This primarily empowers the kernel to
/// issue a single IPI for invalidating page tables on other cores as part of
/// this syscall. This is in contrast to a syscall-per-region to madvise which
/// requires an IPI-per-region. For the pooling allocator it can be much more
/// beneficial to issue a batched syscall with one IPI overhead.
#[cfg(all(feature = "pooling-allocator", target_os = "linux"))]
mod process_madvise {
    use super::iovec;
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

    static SUPPORTED: AtomicBool = AtomicBool::new(true);
    const PIDFD_SELF: libc::c_int = -10000;

    pub unsafe fn run_self(
        raw: &[iovec],
        advice: libc::c_int,
        flags: libc::c_int,
    ) -> io::Result<bool> {
        if !SUPPORTED.load(Relaxed) {
            return Ok(false);
        }
        for chunk in raw.chunks(usize::try_from(libc::UIO_MAXIOV).unwrap()) {
            let expected: usize = chunk.iter().map(|iovec| iovec.iov_len).sum();
            let ret = unsafe {
                libc::syscall(
                    libc::SYS_process_madvise,
                    PIDFD_SELF,
                    chunk.as_ptr(),
                    chunk.len(),
                    advice,
                    flags,
                )
            };

            if ret < 0 {
                let err = io::Error::last_os_error();
                // All of these are permanent failure conditions that we can't
                // recover here, for example too old a kernel for the syscall
                // (ENOSYS), too old a kernel for MADV_DONTNEED (EINVAL), too
                // old a kernel for PIDFD_SELF (EINVAL), or we're not allowed to
                // use this syscall (EPERM).
                match err.raw_os_error() {
                    Some(libc::ENOSYS) | Some(libc::EINVAL) | Some(libc::EPERM) => {
                        SUPPORTED.store(false, Relaxed);
                        return Ok(false);
                    }
                    _ => {}
                }
                return Err(err);
            }

            // If the kernel didn't actually reset everything for us then that's
            // considered a failure and this needs to be done one-by-one.
            if usize::try_from(ret).unwrap() != expected {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
