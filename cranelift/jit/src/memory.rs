#[cfg(feature = "selinux-fix")]
use memmap2::MmapMut;

#[cfg(not(any(feature = "selinux-fix", windows)))]
use std::alloc;
use std::convert::TryFrom;
use std::io;
use std::mem;
use std::ptr;

/// A simple struct consisting of a pointer and length.
struct PtrLen {
    #[cfg(feature = "selinux-fix")]
    map: Option<MmapMut>,

    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new empty `PtrLen`.
    fn new() -> Self {
        Self {
            #[cfg(feature = "selinux-fix")]
            map: None,

            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    #[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
    fn with_size(size: usize) -> io::Result<Self> {
        let alloc_size = region::page::ceil(size);
        MmapMut::map_anon(alloc_size).map(|mut mmap| {
            // The order here is important; we assign the pointer first to get
            // around compile time borrow errors.
            Ok(Self {
                ptr: mmap.as_mut_ptr(),
                map: Some(mmap),
                len: alloc_size,
            })
        })
    }

    #[cfg(all(not(target_os = "windows"), not(feature = "selinux-fix")))]
    fn with_size(size: usize) -> io::Result<Self> {
        assert_ne!(size, 0);
        let page_size = region::page::size();
        let alloc_size = region::page::ceil(size);
        let layout = alloc::Layout::from_size_align(alloc_size, page_size).unwrap();
        // Safety: We assert that the size is non-zero above.
        let ptr = unsafe { alloc::alloc(layout) };

        Ok(Self {
            ptr,
            len: alloc_size,
        })
    }

    #[cfg(target_os = "windows")]
    fn with_size(size: usize) -> io::Result<Self> {
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};

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
                len: region::page::ceil(size),
            })
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

// `MMapMut` from `cfg(feature = "selinux-fix")` already deallocates properly.
#[cfg(all(not(target_os = "windows"), not(feature = "selinux-fix")))]
impl Drop for PtrLen {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let page_size = region::page::size();
            let layout = alloc::Layout::from_size_align(self.len, page_size).unwrap();
            unsafe {
                region::protect(self.ptr, self.len, region::Protection::READ_WRITE)
                    .expect("unable to unprotect memory");
                alloc::dealloc(self.ptr, layout)
            }
        }
    }
}

// TODO: add a `Drop` impl for `cfg(target_os = "windows")`

/// JIT memory manager. This manages pages of suitably aligned and
/// accessible memory. Memory will be leaked by default to have
/// function pointers remain valid for the remainder of the
/// program's life.
pub(crate) struct Memory {
    allocations: Vec<PtrLen>,
    executable: usize,
    current: PtrLen,
    position: usize,
}

impl Memory {
    pub(crate) fn new() -> Self {
        Self {
            allocations: Vec::new(),
            executable: 0,
            current: PtrLen::new(),
            position: 0,
        }
    }

    fn finish_current(&mut self) {
        self.allocations
            .push(mem::replace(&mut self.current, PtrLen::new()));
        self.position = 0;
    }

    pub(crate) fn allocate(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        let align = usize::try_from(align).expect("alignment too big");
        if self.position % align != 0 {
            self.position += align - self.position % align;
            debug_assert!(self.position % align == 0);
        }

        if size <= self.current.len - self.position {
            // TODO: Ensure overflow is not possible.
            let ptr = unsafe { self.current.ptr.add(self.position) };
            self.position += size;
            return Ok(ptr);
        }

        self.finish_current();

        // TODO: Allocate more at a time.
        self.current = PtrLen::with_size(size)?;
        self.position = size;
        Ok(self.current.ptr)
    }

    /// Set all memory allocated in this `Memory` up to now as readable and executable.
    pub(crate) fn set_readable_and_executable(&mut self) {
        self.finish_current();

        #[cfg(feature = "selinux-fix")]
        {
            for &PtrLen { ref map, ptr, len } in &self.allocations[self.executable..] {
                if len != 0 && map.is_some() {
                    unsafe {
                        region::protect(ptr, len, region::Protection::READ_EXECUTE)
                            .expect("unable to make memory readable+executable");
                    }
                }
            }
        }

        #[cfg(not(feature = "selinux-fix"))]
        {
            for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
                if len != 0 {
                    unsafe {
                        region::protect(ptr, len, region::Protection::READ_EXECUTE)
                            .expect("unable to make memory readable+executable");
                    }
                }
            }
        }

        self.executable = self.allocations.len();
    }

    /// Set all memory allocated in this `Memory` up to now as readonly.
    pub(crate) fn set_readonly(&mut self) {
        self.finish_current();

        #[cfg(feature = "selinux-fix")]
        {
            for &PtrLen { ref map, ptr, len } in &self.allocations[self.executable..] {
                if len != 0 && map.is_some() {
                    unsafe {
                        region::protect(ptr, len, region::Protection::READ)
                            .expect("unable to make memory readonly");
                    }
                }
            }
        }

        #[cfg(not(feature = "selinux-fix"))]
        {
            for &PtrLen { ptr, len } in &self.allocations[self.executable..] {
                if len != 0 {
                    unsafe {
                        region::protect(ptr, len, region::Protection::READ)
                            .expect("unable to make memory readonly");
                    }
                }
            }
        }

        self.executable = self.allocations.len();
    }

    /// Frees all allocated memory regions that would be leaked otherwise.
    /// Likely to invalidate existing function pointers, causing unsafety.
    pub(crate) unsafe fn free_memory(&mut self) {
        self.allocations.clear();
        self.executable = 0;
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        // leak memory to guarantee validity of function pointers
        mem::replace(&mut self.allocations, Vec::new())
            .into_iter()
            .for_each(mem::forget);
    }
}
