use std::mem;
use std::ptr;
use errno;
use libc;
use region;

struct PtrLen {
    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    fn with_size(size: usize) -> Result<Self, String> {
        let page_size = region::page::size();
        let alloc_size = (size + (page_size - 1)) & (page_size - 1);
        unsafe {
            let mut ptr: *mut libc::c_void = mem::uninitialized();
            let err = libc::posix_memalign(&mut ptr, page_size, alloc_size);
            if err == 0 {
                Ok(Self {
                    ptr: mem::transmute(ptr),
                    len: alloc_size,
                })
            } else {
                Err(errno::Errno(err).to_string())
            }
        }
    }
}

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
