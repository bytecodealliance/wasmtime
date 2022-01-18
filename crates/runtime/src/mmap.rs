//! Low-level abstraction for allocating and managing zero-filled pages
//! of memory.

use anyhow::anyhow;
use anyhow::{Context, Result};
use more_asserts::assert_le;
use std::convert::TryFrom;
use std::fs::File;
use std::ops::Range;
use std::path::Path;
use std::ptr;
use std::slice;

/// A simple struct consisting of a page-aligned pointer to page-aligned
/// and initially-zeroed memory and a length.
#[derive(Debug)]
pub struct Mmap {
    // Note that this is stored as a `usize` instead of a `*const` or `*mut`
    // pointer to allow this structure to be natively `Send` and `Sync` without
    // `unsafe impl`. This type is sendable across threads and shareable since
    // the coordination all happens at the OS layer.
    ptr: usize,
    len: usize,
    file: Option<File>,

    #[cfg(target_os = "linux")]
    memfd: Option<(rustix::io::OwnedFd, usize, usize)>,
}

impl Mmap {
    /// Construct a new empty instance of `Mmap`.
    pub fn new() -> Self {
        // Rust's slices require non-null pointers, even when empty. `Vec`
        // contains code to create a non-null dangling pointer value when
        // constructed empty, so we reuse that here.
        let empty = Vec::<u8>::new();
        Self {
            ptr: empty.as_ptr() as usize,
            len: 0,
            file: None,

            #[cfg(target_os = "linux")]
            memfd: None,
        }
    }

    /// Create a new `Mmap` pointing to at least `size` bytes of page-aligned accessible memory.
    pub fn with_at_least(size: usize) -> Result<Self> {
        let rounded_size = region::page::ceil(size);
        Self::accessible_reserved(rounded_size, rounded_size)
    }

    /// Creates a new `Mmap` by opening the file located at `path` and mapping
    /// it into memory.
    ///
    /// The memory is mapped in read-only mode for the entire file. If portions
    /// of the file need to be modified then the `region` crate can be use to
    /// alter permissions of each page.
    ///
    /// The memory mapping and the length of the file within the mapping are
    /// returned.
    pub fn from_file(path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            let file = File::open(path).context("failed to open file")?;
            let len = file
                .metadata()
                .context("failed to get file metadata")?
                .len();
            let len = usize::try_from(len).map_err(|_| anyhow!("file too large to map"))?;
            let ptr = unsafe {
                rustix::io::mmap(
                    ptr::null_mut(),
                    len,
                    rustix::io::ProtFlags::READ,
                    rustix::io::MapFlags::PRIVATE,
                    &file,
                    0,
                )
                .context(format!("mmap failed to allocate {:#x} bytes", len))?
            };

            Ok(Self {
                ptr: ptr as usize,
                len,
                file: Some(file),

                #[cfg(target_os = "linux")]
                memfd: None,
            })
        }

        #[cfg(windows)]
        {
            use std::fs::OpenOptions;
            use std::io;
            use std::os::windows::prelude::*;
            use winapi::um::handleapi::*;
            use winapi::um::memoryapi::*;
            use winapi::um::winnt::*;
            unsafe {
                // Open the file with read/execute access and only share for
                // read. This will enable us to perform the proper mmap below
                // while also disallowing other processes modifying the file
                // and having those modifications show up in our address space.
                let file = OpenOptions::new()
                    .read(true)
                    .access_mode(FILE_GENERIC_READ | FILE_GENERIC_EXECUTE)
                    .share_mode(FILE_SHARE_READ)
                    .open(path)
                    .context("failed to open file")?;

                let len = file
                    .metadata()
                    .context("failed to get file metadata")?
                    .len();
                let len = usize::try_from(len).map_err(|_| anyhow!("file too large to map"))?;

                // Create a file mapping that allows PAGE_EXECUTE_READ which
                // we'll be using for mapped text sections in ELF images later.
                let mapping = CreateFileMappingW(
                    file.as_raw_handle().cast(),
                    ptr::null_mut(),
                    PAGE_EXECUTE_READ,
                    0,
                    0,
                    ptr::null(),
                );
                if mapping.is_null() {
                    return Err(io::Error::last_os_error())
                        .context("failed to create file mapping");
                }

                // Create a view for the entire file using `FILE_MAP_EXECUTE`
                // here so that we can later change the text section to execute.
                let ptr = MapViewOfFile(mapping, FILE_MAP_READ | FILE_MAP_EXECUTE, 0, 0, len);
                let err = io::Error::last_os_error();
                CloseHandle(mapping);
                if ptr.is_null() {
                    return Err(err)
                        .context(format!("failed to create map view of {:#x} bytes", len));
                }

                let ret = Self {
                    ptr: ptr as usize,
                    len,
                    file: Some(file),
                };

                // Protect the entire file as PAGE_READONLY to start (i.e.
                // remove the execute bit)
                let mut old = 0;
                if VirtualProtect(ret.ptr as *mut _, ret.len, PAGE_READONLY, &mut old) == 0 {
                    return Err(io::Error::last_os_error())
                        .context("failed change pages to `PAGE_READONLY`");
                }

                Ok(ret)
            }
        }
    }

    /// Create a new `Mmap` pointing to `accessible_size` bytes of page-aligned accessible memory,
    /// within a reserved mapping of `mapping_size` bytes. `accessible_size` and `mapping_size`
    /// must be native page-size multiples.
    #[cfg(not(target_os = "windows"))]
    pub fn accessible_reserved(accessible_size: usize, mapping_size: usize) -> Result<Self> {
        let page_size = region::page::size();
        assert_le!(accessible_size, mapping_size);
        assert_eq!(mapping_size & (page_size - 1), 0);
        assert_eq!(accessible_size & (page_size - 1), 0);

        // Mmap may return EINVAL if the size is zero, so just
        // special-case that.
        if mapping_size == 0 {
            return Ok(Self::new());
        }

        Ok(if accessible_size == mapping_size {
            // Allocate a single read-write region at once.
            let ptr = unsafe {
                rustix::io::mmap_anonymous(
                    ptr::null_mut(),
                    mapping_size,
                    rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                    rustix::io::MapFlags::PRIVATE,
                )
                .context(format!("mmap failed to allocate {:#x} bytes", mapping_size))?
            };

            Self {
                ptr: ptr as usize,
                len: mapping_size,
                file: None,

                #[cfg(target_os = "linux")]
                memfd: None,
            }
        } else {
            // Reserve the mapping size.
            let ptr = unsafe {
                rustix::io::mmap_anonymous(
                    ptr::null_mut(),
                    mapping_size,
                    rustix::io::ProtFlags::empty(),
                    rustix::io::MapFlags::PRIVATE,
                )
                .context(format!("mmap failed to allocate {:#x} bytes", mapping_size))?
            };

            let mut result = Self {
                ptr: ptr as usize,
                len: mapping_size,
                file: None,

                #[cfg(target_os = "linux")]
                memfd: None,
            };

            if accessible_size != 0 {
                // Commit the accessible size.
                result.make_accessible(0, accessible_size)?;
            }

            result
        })
    }

    /// Create a new `Mmap` pointing to `accessible_size` bytes of page-aligned accessible memory,
    /// within a reserved mapping of `mapping_size` bytes. `accessible_size` and `mapping_size`
    /// must be native page-size multiples.
    #[cfg(target_os = "windows")]
    pub fn accessible_reserved(accessible_size: usize, mapping_size: usize) -> Result<Self> {
        use anyhow::bail;
        use std::io;
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE};

        if mapping_size == 0 {
            return Ok(Self::new());
        }

        let page_size = region::page::size();
        assert_le!(accessible_size, mapping_size);
        assert_eq!(mapping_size & (page_size - 1), 0);
        assert_eq!(accessible_size & (page_size - 1), 0);

        Ok(if accessible_size == mapping_size {
            // Allocate a single read-write region at once.
            let ptr = unsafe {
                VirtualAlloc(
                    ptr::null_mut(),
                    mapping_size,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                )
            };
            if ptr.is_null() {
                bail!("VirtualAlloc failed: {}", io::Error::last_os_error());
            }

            Self {
                ptr: ptr as usize,
                len: mapping_size,
                file: None,
            }
        } else {
            // Reserve the mapping size.
            let ptr =
                unsafe { VirtualAlloc(ptr::null_mut(), mapping_size, MEM_RESERVE, PAGE_NOACCESS) };
            if ptr.is_null() {
                bail!("VirtualAlloc failed: {}", io::Error::last_os_error());
            }

            let mut result = Self {
                ptr: ptr as usize,
                len: mapping_size,
                file: None,
            };

            if accessible_size != 0 {
                // Commit the accessible size.
                result.make_accessible(0, accessible_size)?;
            }

            result
        })
    }

    /// Make the memory starting at `start` and extending for `len` bytes accessible.
    /// `start` and `len` must be native page-size multiples and describe a range within
    /// `self`'s reserved memory.
    #[cfg(not(target_os = "windows"))]
    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        let page_size = region::page::size();
        assert_eq!(start & (page_size - 1), 0);
        assert_eq!(len & (page_size - 1), 0);
        assert_le!(len, self.len);
        assert_le!(start, self.len - len);

        // Commit the accessible size.
        let ptr = self.ptr as *const u8;
        unsafe {
            region::protect(ptr.add(start), len, region::Protection::READ_WRITE)?;
        }

        Ok(())
    }

    /// Make the memory starting at `start` and extending for `len` bytes accessible.
    /// `start` and `len` must be native page-size multiples and describe a range within
    /// `self`'s reserved memory.
    #[cfg(target_os = "windows")]
    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        use anyhow::bail;
        use std::io;
        use winapi::ctypes::c_void;
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, PAGE_READWRITE};
        let page_size = region::page::size();
        assert_eq!(start & (page_size - 1), 0);
        assert_eq!(len & (page_size - 1), 0);
        assert_le!(len, self.len);
        assert_le!(start, self.len - len);

        // Commit the accessible size.
        let ptr = self.ptr as *const u8;
        if unsafe {
            VirtualAlloc(
                ptr.add(start) as *mut c_void,
                len,
                MEM_COMMIT,
                PAGE_READWRITE,
            )
        }
        .is_null()
        {
            bail!("VirtualAlloc failed: {}", io::Error::last_os_error());
        }

        Ok(())
    }

    /// Returns a page-aligned offset + length pair delimiting the memory pages which
    /// are currently populated without generating any extraneous page faults.
    #[cfg(target_os = "linux")]
    fn populated_range(
        &self,
        accessible_offset: usize,
        accessible: usize,
    ) -> Result<(usize, usize)> {
        // Docs: https://www.kernel.org/doc/Documentation/vm/pagemap.txt
        use std::io::{Read, Seek};
        const PAGE_SIZE: usize = 4096;

        assert_eq!(rustix::process::page_size(), PAGE_SIZE);

        assert_eq!(accessible_offset % PAGE_SIZE, 0);
        assert_eq!(accessible % PAGE_SIZE, 0);

        let mut page_last_index = 0;
        let mut page_first_index = None;
        unsafe {
            let mut fp = std::fs::File::open("/proc/self/pagemap")
                .context("failed to open /proc/self/pagemap")?;

            let offset = (self.as_ptr() as usize + accessible_offset) / PAGE_SIZE
                * std::mem::size_of::<u64>();
            fp.seek(std::io::SeekFrom::Start(offset as u64))
                .context("failed to seek inside of /proc/self/pagemap")?;

            union Buffer {
                as_u8: [u8; PAGE_SIZE],
                as_u64: [u64; PAGE_SIZE / std::mem::size_of::<u64>()],
            }

            let mut buffer: Buffer = std::mem::zeroed();

            let total_page_count = accessible / PAGE_SIZE;
            let mut current_page_offset = 0;
            while current_page_offset < total_page_count {
                let page_count =
                    std::cmp::min(buffer.as_u64.len(), total_page_count - current_page_offset);
                fp.read(&mut buffer.as_u8[..page_count * std::mem::size_of::<u64>()])
                    .context("failed to read from /proc/self/pagemap")?;

                for relative_page_index in 0..page_count {
                    let is_populated = (buffer.as_u64[relative_page_index] & (0b11 << 62)) != 0;
                    if is_populated {
                        page_last_index = current_page_offset + relative_page_index;
                        if page_first_index.is_none() {
                            page_first_index = Some(page_last_index);
                        }
                    }
                }

                current_page_offset += page_count;
            }
        }

        if let Some(page_first_index) = page_first_index {
            let (data_offset, data_length) = (
                accessible_offset + page_first_index * PAGE_SIZE,
                (page_last_index - page_first_index + 1) * PAGE_SIZE,
            );

            assert!(data_offset + data_length <= self.len);
            Ok((data_offset, data_length))
        } else {
            Ok((accessible_offset, 0))
        }
    }

    /// Saves a snapshot of the current contents of the mapping in an memfd,
    /// and replaces that part of the mapping with a copy-on-write copy.
    ///
    /// Can only be used once.
    #[cfg(target_os = "linux")]
    pub fn create_snapshot(&mut self, accessible_offset: usize, accessible: usize) -> Result<()> {
        assert!(self.memfd.is_none());
        assert!(accessible_offset + accessible_offset <= self.len);

        // Here we narrow down the exact range of memory which is populated.
        //
        // We could in theory not do this, however resetting copy-on-write
        // pages is visibly slower (since that actually copies memory) than
        // resetting those which are not copy-on-write, so we want to only
        // snapshot as narrow of a memory region as possible.
        let (data_offset, data_length) = self.populated_range(accessible_offset, accessible)?;

        debug_assert!(self.as_slice()[accessible_offset..data_offset]
            .iter()
            .all(|&byte| byte == 0));

        debug_assert!(
            self.as_slice()[data_offset + data_length..accessible_offset + accessible]
                .iter()
                .all(|&byte| byte == 0)
        );

        if data_length == 0 {
            // Memory is completely empty, so no point in doing anything.
            return Ok(());
        }

        unsafe {
            let memfd = rustix::fs::memfd_create("wasmtime", rustix::fs::MemfdFlags::CLOEXEC)
                .context("memfd_create failed")?;

            rustix::fs::ftruncate(&memfd, data_length as u64)
                .context("failed to enlarge memfd: ftruncate failed")?;

            // In theory we could just map the memfd in memory and do a direct copy,
            // but simply using `write` is going to have a lower overhead.
            use rustix::fd::AsRawFd;
            let bytes_written = libc::write(
                memfd.as_raw_fd(),
                (self.ptr as *const u8)
                    .add(data_offset)
                    .cast::<std::os::raw::c_void>(),
                data_length,
            );
            if bytes_written < 0 {
                anyhow::bail!(
                    "failed to copy memory contents into memfd: write failed: {}",
                    std::io::Error::last_os_error()
                );
            }
            if bytes_written as usize != data_length {
                anyhow::bail!("failed to copy memory contents into memfd: managed to write only {} bytes; expected {} bytes", bytes_written, data_length);
            }

            rustix::io::mmap(
                (self.ptr as *mut u8)
                    .add(data_offset)
                    .cast::<std::os::raw::c_void>(),
                data_length,
                rustix::io::ProtFlags::READ | rustix::io::ProtFlags::WRITE,
                rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                &memfd,
                0,
            )
            .context("failed to attach the memfd: mmap failed")?;

            self.memfd = Some((memfd, data_offset, data_length));
        };

        Ok(())
    }

    /// Resets the memory contents within the given range.
    ///
    /// The memory will be filled with its original contents from
    /// when [`Mmap::create_snapshot`] was called, or cleared
    /// with zeros for non-memfd mappings.
    #[cfg(target_os = "linux")]
    pub unsafe fn reset(&mut self, offset: usize, length: usize) -> Result<()> {
        rustix::io::madvise(
            (self.ptr as *mut u8)
                .add(offset)
                .cast::<std::os::raw::c_void>(),
            length,
            rustix::io::Advice::LinuxDontNeed,
        )?;

        Ok(())
    }

    /// Makes the memory within the given range completely inaccessible.
    #[cfg(target_os = "linux")]
    pub unsafe fn make_inaccessible(&mut self, offset: usize, length: usize) -> Result<()> {
        rustix::io::mprotect(
            (self.ptr as *mut u8)
                .add(offset)
                .cast::<std::os::raw::c_void>(),
            length,
            rustix::io::MprotectFlags::empty(),
        )?;
        Ok(())
    }

    /// Reallocates this mapping preserving its contents.
    pub fn reallocate(
        &mut self,
        accessible_offset: usize,
        old_accessible_size: usize,
        new_mapping_size: usize,
        new_accessible_size: usize,
    ) -> Result<()> {
        let mut new_mmap = Self::accessible_reserved(0, new_mapping_size)?;

        #[cfg(target_os = "linux")]
        if let Some((ref memfd, data_offset, data_length)) = self.memfd {
            unsafe {
                rustix::io::mmap(
                    (new_mmap.ptr as *mut u8)
                        .add(data_offset)
                        .cast::<std::os::raw::c_void>(),
                    data_length,
                    rustix::io::ProtFlags::empty(),
                    rustix::io::MapFlags::PRIVATE | rustix::io::MapFlags::FIXED,
                    memfd,
                    0,
                )
                .context("failed to map the memfd: mmap failed")?;
            }
        }

        new_mmap.make_accessible(accessible_offset, new_accessible_size)?;
        new_mmap.as_mut_slice()[accessible_offset..][..old_accessible_size]
            .copy_from_slice(&self.as_slice()[accessible_offset..][..old_accessible_size]);

        #[cfg(target_os = "linux")]
        {
            new_mmap.memfd = self.memfd.take();
        }

        std::mem::swap(self, &mut new_mmap);
        Ok(())
    }

    /// Return the allocated memory as a slice of u8.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }

    /// Return the allocated memory as a mutable slice of u8.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(!self.is_readonly());
        unsafe { slice::from_raw_parts_mut(self.ptr as *mut u8, self.len) }
    }

    /// Return the allocated memory as a pointer to u8.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr as *const u8
    }

    /// Return the allocated memory as a mutable pointer to u8.
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.ptr as *mut u8
    }

    /// Return the length of the allocated memory.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return whether any memory has been allocated.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the underlying mapping is readonly, meaning that
    /// attempts to write will fault.
    pub fn is_readonly(&self) -> bool {
        self.file.is_some()
    }

    /// Makes the specified `range` within this `Mmap` to be read/write.
    pub unsafe fn make_writable(&self, range: Range<usize>) -> Result<()> {
        assert!(range.start <= self.len());
        assert!(range.end <= self.len());
        assert!(range.start <= range.end);
        assert!(
            range.start % region::page::size() == 0,
            "changing of protections isn't page-aligned",
        );

        let base = self.as_ptr().add(range.start);
        let len = range.end - range.start;

        // On Windows when we have a file mapping we need to specifically use
        // `PAGE_WRITECOPY` to ensure that pages are COW'd into place because
        // we don't want our modifications to go back to the original file.
        #[cfg(windows)]
        {
            use std::io;
            use winapi::um::memoryapi::*;
            use winapi::um::winnt::*;

            if self.file.is_some() {
                let mut old = 0;
                if VirtualProtect(base as *mut _, len, PAGE_WRITECOPY, &mut old) == 0 {
                    return Err(io::Error::last_os_error())
                        .context("failed to change pages to `PAGE_WRITECOPY`");
                }
                return Ok(());
            }
        }

        // If we're not on Windows or if we're on Windows with an anonymous
        // mapping then we can use the `region` crate.
        region::protect(base, len, region::Protection::READ_WRITE)?;
        Ok(())
    }

    /// Makes the specified `range` within this `Mmap` to be read/execute.
    pub unsafe fn make_executable(&self, range: Range<usize>) -> Result<()> {
        assert!(range.start <= self.len());
        assert!(range.end <= self.len());
        assert!(range.start <= range.end);
        assert!(
            range.start % region::page::size() == 0,
            "changing of protections isn't page-aligned",
        );

        region::protect(
            self.as_ptr().add(range.start),
            range.end - range.start,
            region::Protection::READ_EXECUTE,
        )?;
        Ok(())
    }
}

impl Drop for Mmap {
    #[cfg(not(target_os = "windows"))]
    fn drop(&mut self) {
        if self.len != 0 {
            unsafe { rustix::io::munmap(self.ptr as *mut std::ffi::c_void, self.len) }
                .expect("munmap failed");
        }
    }

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        if self.len != 0 {
            use winapi::ctypes::c_void;
            use winapi::um::memoryapi::*;
            use winapi::um::winnt::MEM_RELEASE;
            if self.file.is_none() {
                let r = unsafe { VirtualFree(self.ptr as *mut c_void, 0, MEM_RELEASE) };
                assert_ne!(r, 0);
            } else {
                let r = unsafe { UnmapViewOfFile(self.ptr as *mut c_void) };
                assert_ne!(r, 0);
            }
        }
    }
}

fn _assert() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<Mmap>();
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
    use super::Mmap;
    use anyhow::Result;

    // A few helper functions to make the tests easier to read.
    fn pages(count: usize) -> usize {
        count * 4096
    }
    fn first_byte_of_page(count: usize) -> usize {
        count * 4096
    }
    fn last_byte_of_page(count: usize) -> usize {
        count * 4096 + 4095
    }

    #[test]
    fn create_snapshot_and_reset() -> Result<()> {
        let mut mmap = Mmap::accessible_reserved(0, pages(4))?;
        mmap.make_accessible(pages(1), pages(3))?;
        mmap.as_mut_slice()[first_byte_of_page(1)] = 1;
        mmap.create_snapshot(pages(1), pages(2))?;

        mmap.as_mut_slice()[first_byte_of_page(1)] = 10;
        mmap.as_mut_slice()[first_byte_of_page(2)] = 100;
        unsafe {
            mmap.reset(pages(1), pages(3))?;
        }

        assert_eq!(mmap.as_slice()[first_byte_of_page(1)], 1);
        assert_eq!(mmap.as_slice()[first_byte_of_page(2)], 0);
        Ok(())
    }

    #[test]
    fn populated_range() -> Result<()> {
        let mut mmap = Mmap::with_at_least(pages(16))?;
        assert_eq!(mmap.populated_range(0, pages(16))?, (0, 0));
        assert_eq!(mmap.populated_range(pages(1), pages(15))?, (pages(1), 0));

        mmap.as_mut_slice()[last_byte_of_page(1)] = 1;
        assert_eq!(mmap.populated_range(0, pages(16))?, (pages(1), pages(1)));

        mmap.as_mut_slice()[first_byte_of_page(1)] = 1;
        assert_eq!(mmap.populated_range(0, pages(16))?, (pages(1), pages(1)));

        mmap.as_mut_slice()[last_byte_of_page(3)] = 1;
        assert_eq!(mmap.populated_range(0, pages(16))?, (pages(1), pages(3)));

        mmap.as_mut_slice()[first_byte_of_page(1)] = 1;
        assert_eq!(mmap.populated_range(0, pages(16))?, (pages(1), pages(3)));

        assert_eq!(
            mmap.populated_range(pages(2), pages(14))?,
            (pages(3), pages(1))
        );
        Ok(())
    }
}
