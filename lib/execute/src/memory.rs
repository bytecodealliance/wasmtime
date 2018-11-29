use errno;
use libc;
use region;
use std::fmt;
use std::mem;
use std::ptr;
use std::slice;
use wasmtime_environ::{MemoryPlan, MemoryStyle, WASM_MAX_PAGES, WASM_PAGE_SIZE};

/// Round `size` up to the nearest multiple of `page_size`.
fn round_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + (page_size - 1)) & !(page_size - 1)
}

/// A simple struct consisting of a page-aligned pointer to page-aligned
/// and initially-zeroed memory and a length.
struct PtrLen {
    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    #[cfg(not(target_os = "windows"))]
    fn with_size(size: usize) -> Result<Self, String> {
        let page_size = region::page::size();
        let alloc_size = round_up_to_page_size(size, page_size);
        unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                alloc_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            if mem::transmute::<_, isize>(ptr) != -1isize {
                Ok(Self {
                    ptr: ptr as *mut u8,
                    len: alloc_size,
                })
            } else {
                Err(errno::errno().to_string())
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn with_size(size: usize) -> Result<Self, String> {
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};

        let page_size = region::page::size();

        // VirtualAlloc always rounds up to the next multiple of the page size
        let ptr = unsafe {
            VirtualAlloc(
                ptr::null_mut(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if !ptr.is_null() {
            Ok(Self {
                ptr: ptr as *mut u8,
                len: round_up_to_page_size(size, page_size),
            })
        } else {
            Err(errno::errno().to_string())
        }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for PtrLen {
    #[cfg(not(target_os = "windows"))]
    fn drop(&mut self) {
        let r = unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
        assert_eq!(r, 0);
    }

    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        use winapi::um::memoryapi::VirtualFree;
        use winapi::um::winnt::MEM_RELEASE;
        let r = unsafe { VirtualFree(self.ptr, self.len, MEM_RELEASE) };
        assert_eq!(r, 0);
    }
}

/// A linear memory instance.
///
/// This linear memory has a stable base address and at the same time allows
/// for dynamical growing.
pub struct LinearMemory {
    ptrlen: PtrLen,
    current: u32,
    maximum: Option<u32>,
    offset_guard_size: usize,
}

impl LinearMemory {
    /// Create a new linear memory instance with specified minimum and maximum number of pages.
    pub fn new(plan: &MemoryPlan) -> Result<Self, String> {
        // `maximum` cannot be set to more than `65536` pages.
        assert!(plan.memory.minimum <= WASM_MAX_PAGES);
        assert!(plan.memory.maximum.is_none() || plan.memory.maximum.unwrap() <= WASM_MAX_PAGES);

        let offset_guard_bytes = plan.offset_guard_size as usize;

        let minimum_pages = match plan.style {
            MemoryStyle::Dynamic => plan.memory.minimum,
            MemoryStyle::Static { bound } => {
                assert!(bound >= plan.memory.minimum);
                bound
            }
        } as usize;
        let minimum_bytes = minimum_pages.checked_mul(WASM_PAGE_SIZE as usize).unwrap();
        let request_bytes = minimum_bytes.checked_add(offset_guard_bytes).unwrap();
        let mapped_pages = plan.memory.minimum as usize;
        let mapped_bytes = mapped_pages * WASM_PAGE_SIZE as usize;
        let unmapped_pages = minimum_pages - mapped_pages;
        let unmapped_bytes = unmapped_pages * WASM_PAGE_SIZE as usize;
        let inaccessible_bytes = unmapped_bytes + offset_guard_bytes;

        let ptrlen = PtrLen::with_size(request_bytes)?;

        // Make the unmapped and offset-guard pages inaccessible.
        unsafe {
            region::protect(
                ptrlen.ptr.add(mapped_bytes),
                inaccessible_bytes,
                region::Protection::Read,
            ).expect("unable to make memory readonly");
        }

        Ok(Self {
            ptrlen,
            current: plan.memory.minimum,
            maximum: plan.memory.maximum,
            offset_guard_size: offset_guard_bytes,
        })
    }

    /// Returns an base address of this linear memory.
    pub fn base_addr(&mut self) -> *mut u8 {
        self.ptrlen.ptr
    }

    /// Returns a number of allocated wasm pages.
    pub fn current_size(&self) -> u32 {
        assert_eq!(self.ptrlen.len % WASM_PAGE_SIZE as usize, 0);
        let num_pages = self.ptrlen.len / WASM_PAGE_SIZE as usize;
        assert_eq!(num_pages as u32 as usize, num_pages);
        num_pages as u32
    }

    /// Grow memory by the specified amount of pages.
    ///
    /// Returns `None` if memory can't be grown by the specified amount
    /// of pages.
    pub fn grow(&mut self, delta: u32) -> Option<u32> {
        let new_pages = match self.current.checked_add(delta) {
            Some(new_pages) => new_pages,
            // Linear memory size overflow.
            None => return None,
        };
        let prev_pages = self.current;

        if let Some(maximum) = self.maximum {
            if new_pages > maximum {
                // Linear memory size would exceed the declared maximum.
                return None;
            }
        }

        // Wasm linear memories are never allowed to grow beyond what is
        // indexable. If the memory has no maximum, enforce the greatest
        // limit here.
        if new_pages >= WASM_MAX_PAGES {
            // Linear memory size would exceed the index range.
            return None;
        }

        let new_bytes = new_pages as usize * WASM_PAGE_SIZE as usize;

        if new_bytes > self.ptrlen.len {
            // If we have no maximum, this is a "dynamic" heap, and it's allowed to move.
            assert!(self.maximum.is_none());
            let mapped_pages = self.current as usize;
            let mapped_bytes = mapped_pages * WASM_PAGE_SIZE as usize;
            let guard_bytes = self.offset_guard_size;

            let mut new_ptrlen = PtrLen::with_size(new_bytes).ok()?;

            // Make the offset-guard pages inaccessible.
            unsafe {
                region::protect(
                    new_ptrlen.ptr.add(mapped_bytes),
                    guard_bytes,
                    region::Protection::Read,
                ).expect("unable to make memory readonly");
            }

            new_ptrlen
                .as_mut_slice()
                .copy_from_slice(self.ptrlen.as_slice());

            self.ptrlen = new_ptrlen;
        }

        self.current = new_pages;

        Some(prev_pages)
    }
}

impl fmt::Debug for LinearMemory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LinearMemory")
            .field("current", &self.current)
            .field("maximum", &self.maximum)
            .finish()
    }
}

impl AsRef<[u8]> for LinearMemory {
    fn as_ref(&self) -> &[u8] {
        self.ptrlen.as_slice()
    }
}

impl AsMut<[u8]> for LinearMemory {
    fn as_mut(&mut self) -> &mut [u8] {
        self.ptrlen.as_mut_slice()
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
