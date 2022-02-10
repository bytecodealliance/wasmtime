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
use std::sync::Arc;

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
    file: Option<Arc<File>>,
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
                file: Some(Arc::new(file)),
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
                    file: Some(Arc::new(file)),
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

    /// Returns the underlying file that this mmap is mapping, if present.
    pub fn original_file(&self) -> Option<&Arc<File>> {
        self.file.as_ref()
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
