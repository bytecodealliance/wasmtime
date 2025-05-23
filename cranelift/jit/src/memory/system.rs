use cranelift_module::{ModuleError, ModuleResult};

#[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
use memmap2::MmapMut;

#[cfg(not(any(feature = "selinux-fix", windows)))]
use std::alloc;
use std::io;
use std::mem;
use std::ptr;

use super::BranchProtection;
use super::JITMemoryProvider;

/// A simple struct consisting of a pointer and length.
struct PtrLen {
    #[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
    map: Option<MmapMut>,

    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new empty `PtrLen`.
    fn new() -> Self {
        Self {
            #[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
            map: None,

            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    #[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
    fn with_size(size: usize) -> io::Result<Self> {
        let alloc_size = region::page::ceil(size as *const ()) as usize;
        MmapMut::map_anon(alloc_size).map(|mut mmap| {
            // The order here is important; we assign the pointer first to get
            // around compile time borrow errors.
            Self {
                ptr: mmap.as_mut_ptr(),
                map: Some(mmap),
                len: alloc_size,
            }
        })
    }

    #[cfg(all(not(target_os = "windows"), not(feature = "selinux-fix")))]
    fn with_size(size: usize) -> io::Result<Self> {
        assert_ne!(size, 0);
        let page_size = region::page::size();
        let alloc_size = region::page::ceil(size as *const ()) as usize;
        let layout = alloc::Layout::from_size_align(alloc_size, page_size).unwrap();
        // Safety: We assert that the size is non-zero above.
        let ptr = unsafe { alloc::alloc(layout) };

        if !ptr.is_null() {
            Ok(Self {
                ptr,
                len: alloc_size,
            })
        } else {
            Err(io::Error::from(io::ErrorKind::OutOfMemory))
        }
    }

    #[cfg(target_os = "windows")]
    fn with_size(size: usize) -> io::Result<Self> {
        use windows_sys::Win32::System::Memory::{
            MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
        };

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
                len: region::page::ceil(size as *const ()) as usize,
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
    already_protected: usize,
    current: PtrLen,
    position: usize,
}

unsafe impl Send for Memory {}

impl Memory {
    pub(crate) fn new() -> Self {
        Self {
            allocations: Vec::new(),
            already_protected: 0,
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
    pub(crate) fn set_readable_and_executable(
        &mut self,
        branch_protection: BranchProtection,
    ) -> ModuleResult<()> {
        self.finish_current();

        for &PtrLen { ptr, len, .. } in self.non_protected_allocations_iter() {
            super::set_readable_and_executable(ptr, len, branch_protection)?;
        }

        // Flush any in-flight instructions from the pipeline
        wasmtime_jit_icache_coherence::pipeline_flush_mt().expect("Failed pipeline flush");

        self.already_protected = self.allocations.len();
        Ok(())
    }

    /// Set all memory allocated in this `Memory` up to now as readonly.
    pub(crate) fn set_readonly(&mut self) -> ModuleResult<()> {
        self.finish_current();

        for &PtrLen { ptr, len, .. } in self.non_protected_allocations_iter() {
            unsafe {
                region::protect(ptr, len, region::Protection::READ).map_err(|e| {
                    ModuleError::Backend(
                        anyhow::Error::new(e).context("unable to make memory readonly"),
                    )
                })?;
            }
        }

        self.already_protected = self.allocations.len();
        Ok(())
    }

    /// Iterates non protected memory allocations that are of not zero bytes in size.
    fn non_protected_allocations_iter(&self) -> impl Iterator<Item = &PtrLen> {
        let iter = self.allocations[self.already_protected..].iter();

        #[cfg(all(not(target_os = "windows"), feature = "selinux-fix"))]
        return iter.filter(|&PtrLen { map, len, .. }| *len != 0 && map.is_some());

        #[cfg(any(target_os = "windows", not(feature = "selinux-fix")))]
        return iter.filter(|&PtrLen { len, .. }| *len != 0);
    }

    /// Frees all allocated memory regions that would be leaked otherwise.
    /// Likely to invalidate existing function pointers, causing unsafety.
    pub(crate) unsafe fn free_memory(&mut self) {
        self.allocations.clear();
        self.already_protected = 0;
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

/// A memory provider that allocates memory on-demand using the system
/// allocator.
///
/// Note: Memory will be leaked by default unless
/// [`JITMemoryProvider::free_memory`] is called to ensure function pointers
/// remain valid for the remainder of the program's life.
pub struct SystemMemoryProvider {
    code: Memory,
    readonly: Memory,
    writable: Memory,
}

impl SystemMemoryProvider {
    /// Create a new memory handle with the given branch protection.
    pub fn new() -> Self {
        Self {
            code: Memory::new(),
            readonly: Memory::new(),
            writable: Memory::new(),
        }
    }
}

impl JITMemoryProvider for SystemMemoryProvider {
    unsafe fn free_memory(&mut self) {
        self.code.free_memory();
        self.readonly.free_memory();
        self.writable.free_memory();
    }

    fn finalize(&mut self, branch_protection: BranchProtection) -> ModuleResult<()> {
        self.readonly.set_readonly()?;
        self.code.set_readable_and_executable(branch_protection)
    }

    fn allocate_readexec(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.code.allocate(size, align)
    }

    fn allocate_readwrite(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.writable.allocate(size, align)
    }

    fn allocate_readonly(&mut self, size: usize, align: u64) -> io::Result<*mut u8> {
        self.readonly.allocate(size, align)
    }
}
