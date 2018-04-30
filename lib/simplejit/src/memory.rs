use std::mem;
use std::ptr;
use errno;
use libc;
use region;

/// Round `size` up to the nearest multiple of `page_size`.
fn round_up_to_page_size(size: usize, page_size: usize) -> usize {
    (size + (page_size - 1)) & !(page_size - 1)
}

/// A simple struct consisting of a pointer and length.
struct PtrLen {
    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new empty `PtrLen`.
    fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    fn with_size(size: usize) -> Result<Self, String> {
        let page_size = region::page::size();
        let alloc_size = round_up_to_page_size(size, page_size);
        unsafe {
            let mut ptr: *mut libc::c_void = mem::uninitialized();
            let err = libc::posix_memalign(&mut ptr, page_size, alloc_size);
            if err == 0 {
                Ok(Self {
                    ptr: ptr as *mut u8,
                    len: alloc_size,
                })
            } else {
                Err(errno::Errno(err).to_string())
            }
        }
    }
}

/// JIT memory manager. This manages pages of suitably aligned and
/// accessible memory.
pub struct Memory {
    allocations: Vec<PtrLen>,
    executable: usize,
    current: PtrLen,
    position: usize,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
            executable: 0,
            current: PtrLen::new(),
            position: 0,
        }
    }

    fn finish_current(&mut self) {
        self.allocations.push(mem::replace(
            &mut self.current,
            PtrLen::new(),
        ));
        self.position = 0;
    }

    /// TODO: Use a proper error type.
    pub fn allocate(&mut self, size: usize) -> Result<*mut u8, String> {
        if size <= self.current.len - self.position {
            // TODO: Ensure overflow is not possible.
            let ptr = unsafe { self.current.ptr.offset(self.position as isize) };
            self.position += size;
            return Ok(ptr);
        }

        self.finish_current();

        // TODO: Allocate more at a time.
        self.current = PtrLen::with_size(size)?;
        self.position = size;
        Ok(self.current.ptr)
    }

    /// Set all memory allocated in this `Memory` up to now as executable.
    pub fn set_executable(&mut self) {
        self.finish_current();

        for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
            if len != 0 {
                unsafe {
                    region::protect(ptr, len, region::Protection::Execute)
                        .expect("unable to make memory executable");
                }
            }
        }
    }

    /// Set all memory allocated in this `Memory` up to now as readonly.
    pub fn set_readonly(&mut self) {
        self.finish_current();

        for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
            if len != 0 {
                unsafe {
                    region::protect(ptr, len, region::Protection::Read).expect(
                        "unable to make memory readonly",
                    );
                }
            }
        }
    }
}

// TODO: Implement Drop to unprotect and deallocate the memory?

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
