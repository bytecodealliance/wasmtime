//! Low-level abstraction for allocating and managing zero-filled pages
//! of memory.

use core::ptr;
use core::slice;
use errno;
use libc;
use region;
use std::string::{String, ToString};
use std::vec::Vec;

/// Round `size` up to the nearest multiple of `page_size`.
fn round_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + (page_size - 1)) & !(page_size - 1)
}

/// A simple struct consisting of a page-aligned pointer to page-aligned
/// and initially-zeroed memory and a length.
#[derive(Debug)]
pub struct Mmap {
    ptr: *mut u8,
    len: usize,
}

impl Mmap {
    /// Construct a new empty instance of `Mmap`.
    pub fn new() -> Self {
        // Rust's slices require non-null pointers, even when empty. `Vec`
        // contains code to create a non-null dangling pointer value when
        // constructed empty, so we reuse that here.
        Self {
            ptr: Vec::new().as_mut_ptr(),
            len: 0,
        }
    }

    /// Create a new `Mmap` pointing to at least `size` bytes of accessible memory,
    /// suitably sized and aligned for memory protection.
    pub fn with_size(size: usize) -> Result<Self, String> {
        Self::accessible_reserved(size, size)
    }

    /// Create a new `Mmap` pointing to at least `accessible_size` bytes of accessible memory,
    /// within a reserved mapping of at least `mapping_size` bytes, suitably sized and aligned
    /// for memory protection.
    #[cfg(not(target_os = "windows"))]
    pub fn accessible_reserved(
        accessible_size: usize,
        mapping_size: usize,
    ) -> Result<Self, String> {
        assert!(accessible_size <= mapping_size);

        // Mmap may return EINVAL if the size is zero, so just
        // special-case that.
        if mapping_size == 0 {
            return Ok(Self::new());
        }

        let page_size = region::page::size();
        let rounded_mapping_size = round_up_to_page_size(mapping_size, page_size);

        Ok(if accessible_size == mapping_size {
            // Allocate a single read-write region at once.
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    rounded_mapping_size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANON,
                    -1,
                    0,
                )
            };
            if ptr as isize == -1_isize {
                return Err(errno::errno().to_string());
            }

            Self {
                ptr: ptr as *mut u8,
                len: rounded_mapping_size,
            }
        } else {
            // Reserve the mapping size.
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    rounded_mapping_size,
                    libc::PROT_NONE,
                    libc::MAP_PRIVATE | libc::MAP_ANON,
                    -1,
                    0,
                )
            };
            if ptr as isize == -1_isize {
                return Err(errno::errno().to_string());
            }

            let result = Self {
                ptr: ptr as *mut u8,
                len: rounded_mapping_size,
            };

            if accessible_size != 0 {
                // Commit the accessible size.
                let rounded_accessible_size = round_up_to_page_size(accessible_size, page_size);
                unsafe {
                    region::protect(
                        result.ptr,
                        rounded_accessible_size,
                        region::Protection::ReadWrite,
                    )
                }
                .map_err(|e| e.to_string())?;
            }

            result
        })
    }

    /// Create a new `Mmap` pointing to at least `accessible_size` bytes of accessible memory,
    /// within a reserved mapping of at least `mapping_size` bytes, suitably sized and aligned
    /// for memory protection.
    #[cfg(target_os = "windows")]
    pub fn accessible_reserved(
        accessible_size: usize,
        mapping_size: usize,
    ) -> Result<Self, String> {
        assert!(accessible_size <= mapping_size);

        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE};

        let page_size = region::page::size();
        let rounded_mapping_size = round_up_to_page_size(mapping_size, page_size);

        Ok(if accessible_size == mapping_size {
            // Allocate a single read-write region at once.
            let ptr = unsafe {
                VirtualAlloc(
                    ptr::null_mut(),
                    rounded_mapping_size,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                )
            };
            if ptr.is_null() {
                return Err(errno::errno().to_string());
            }

            Self {
                ptr: ptr as *mut u8,
                len: rounded_mapping_size,
            }
        } else {
            // Reserve the mapping size.
            let ptr = unsafe {
                VirtualAlloc(
                    ptr::null_mut(),
                    rounded_mapping_size,
                    MEM_RESERVE,
                    PAGE_NOACCESS,
                )
            };
            if ptr.is_null() {
                return Err(errno::errno().to_string());
            }

            let result = Self {
                ptr: ptr as *mut u8,
                len: rounded_mapping_size,
            };

            if accessible_size != 0 {
                // Commit the accessible size.
                let rounded_accessible_size = round_up_to_page_size(accessible_size, page_size);
                if unsafe { VirtualAlloc(ptr, rounded_accessible_size, MEM_COMMIT, PAGE_READWRITE) }
                    .is_null()
                {
                    return Err(errno::errno().to_string());
                }
            }

            result
        })
    }

    /// Make the memory starting at `start` and extending for `len` bytes accessible.
    #[cfg(not(target_os = "windows"))]
    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<(), String> {
        // Mmap may return EINVAL if the size is zero, so just
        // special-case that.
        if len == 0 {
            return Ok(());
        }

        let page_size = region::page::size();

        assert_eq!(start % page_size, 0);
        assert_eq!(len % page_size, 0);
        assert!(len < self.len);
        assert!(start < self.len - len);

        // Commit the accessible size.
        unsafe { region::protect(self.ptr.add(start), len, region::Protection::ReadWrite) }
            .map_err(|e| e.to_string())
    }

    /// Make the memory starting at `start` and extending for `len` bytes accessible.
    #[cfg(target_os = "windows")]
    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<(), String> {
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE};

        let page_size = region::page::size();

        assert_eq!(start % page_size, 0);
        assert_eq!(len % page_size, 0);
        assert!(len < self.len);
        assert!(start < self.len - len);

        // Commit the accessible size.
        if unsafe { VirtualAlloc(self.ptr.add(start), len, MEM_COMMIT, PAGE_READWRITE) }.is_null() {
            return Err(errno::errno().to_string());
        }

        Ok(())
    }

    /// Return the allocated memory as a slice of u8.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Return the allocated memory as a mutable slice of u8.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Return the allocated memory as a pointer to u8.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Return the allocated memory as a mutable pointer to u8.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Return the length of the allocated memory.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for Mmap {
    #[cfg(not(target_os = "windows"))]
    fn drop(&mut self) {
        if self.len != 0 {
            let r = unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
            assert_eq!(r, 0, "munmap failed: {}", errno::errno());
        }
    }

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        if self.len != 0 {
            use winapi::um::memoryapi::VirtualFree;
            use winapi::um::winnt::MEM_RELEASE;
            let r = unsafe { VirtualFree(self.ptr as *mut libc::c_void, self.len, MEM_RELEASE) };
            assert_eq!(r, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_up_to_page_size() {
        assert_eq!(round_up_to_page_size(0, 4096), 0);
        assert_eq!(round_up_to_page_size(1, 4096), 4096);
        assert_eq!(round_up_to_page_size(4096, 4096), 4096);
        assert_eq!(round_up_to_page_size(4097, 4096), 8192);
    }
}
