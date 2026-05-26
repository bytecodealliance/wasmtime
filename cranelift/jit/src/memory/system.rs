use cranelift_module::{ModuleError, ModuleResult};
use memmap2::MmapMut;

use std::io;
use std::mem;
use std::ptr;

use super::{BranchProtection, JITMemoryKind, JITMemoryProvider};

/// A simple struct consisting of a pointer and length.
struct PtrLen {
    map: Option<MmapMut>,
    ptr: *mut u8,
    len: usize,
}

impl PtrLen {
    /// Create a new empty `PtrLen`.
    fn new() -> Self {
        Self {
            map: None,
            ptr: ptr::null_mut(),
            len: 0,
        }
    }

    // macOS ARM64: Use mmap for W^X policy compliance.
    // Only passes MAP_JIT for pages that will actually be executable.
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    fn with_size(size: usize, executable: bool) -> io::Result<Self> {
        assert_ne!(size, 0);
        let alloc_size = region::page::ceil(size as *const ()) as usize;

        let mut flags = libc::MAP_PRIVATE | libc::MAP_ANON;
        if executable {
            flags |= libc::MAP_JIT;
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                alloc_size,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            map: None,
            ptr: ptr as *mut u8,
            len: alloc_size,
        })
    }

    // Linux x86_64: Use mmap with a hint address to keep JIT code within 2GB
    // of runtime symbols. Without this, the system allocator may place JIT code
    // at arbitrary virtual addresses >2GB away, causing i32 overflow in x86_64
    // PC-relative relocations (X86PCRel4, X86CallPCRel4).
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn with_size(size: usize, _executable: bool) -> io::Result<Self> {
        assert_ne!(size, 0);
        let alloc_size = region::page::ceil(size as *const ()) as usize;

        // Use this function's own address as the mmap hint. Since this code is
        // linked into the same binary as the runtime symbols, the OS will try
        // to allocate nearby, keeping JIT code within 32-bit relative range.
        let hint = Self::with_size as *const () as *mut libc::c_void;
        let ptr = unsafe {
            libc::mmap(
                hint,
                alloc_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            map: None,
            ptr: ptr as *mut u8,
            len: alloc_size,
        })
    }

    /// Create a new `PtrLen` pointing to at least `size` bytes of memory,
    /// suitably sized and aligned for memory protection.
    #[cfg(not(any(
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_os = "linux", target_arch = "x86_64"),
    )))]
    fn with_size(size: usize, _executable: bool) -> io::Result<Self> {
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
}

// macOS ARM64 and Linux x86_64 allocate via raw `mmap`, so they need an
// explicit `munmap` on drop. Other platforms back allocations with `MmapMut`,
// whose own `Drop` impl frees the memory.
#[cfg(any(
    all(target_arch = "aarch64", target_os = "macos"),
    all(target_os = "linux", target_arch = "x86_64"),
))]
impl Drop for PtrLen {
    fn drop(&mut self) {
        if self.map.is_none() && !self.ptr.is_null() {
            unsafe {
                let _ = region::protect(self.ptr, self.len, region::Protection::READ_WRITE);
                libc::munmap(self.ptr as *mut libc::c_void, self.len);
            }
        }
    }
}

/// JIT memory manager. This manages pages of suitably aligned and
/// accessible memory. Memory will be leaked by default to have
/// function pointers remain valid for the remainder of the
/// program's life.
pub(crate) struct Memory {
    allocations: Vec<PtrLen>,
    already_protected: usize,
    current: PtrLen,
    position: usize,
    executable: bool,
}

unsafe impl Send for Memory {}

impl Memory {
    pub(crate) fn new(executable: bool) -> Self {
        Self {
            allocations: Vec::new(),
            already_protected: 0,
            current: PtrLen::new(),
            position: 0,
            executable,
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
        self.current = PtrLen::with_size(size, self.executable)?;
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
        // Raw-mmap platforms (macOS ARM64, Linux x86_64) leave `map` as `None`
        // even for valid allocations, so check `len` only. Other platforms use
        // `MmapMut`, where a non-empty allocation always carries `Some(map)`.
        let iter = self.allocations[self.already_protected..].iter();

        #[cfg(any(
            all(target_arch = "aarch64", target_os = "macos"),
            all(target_os = "linux", target_arch = "x86_64"),
        ))]
        return iter.filter(|&PtrLen { len, .. }| *len != 0);

        #[cfg(not(any(
            all(target_arch = "aarch64", target_os = "macos"),
            all(target_os = "linux", target_arch = "x86_64"),
        )))]
        return iter.filter(|&PtrLen { map, len, .. }| *len != 0 && map.is_some());
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
            code: Memory::new(true),
            readonly: Memory::new(false),
            writable: Memory::new(false),
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

    fn allocate(&mut self, size: usize, align: u64, kind: JITMemoryKind) -> io::Result<*mut u8> {
        match kind {
            JITMemoryKind::Executable => self.code.allocate(size, align),
            JITMemoryKind::Writable => self.writable.allocate(size, align),
            JITMemoryKind::ReadOnly => self.readonly.allocate(size, align),
        }
    }
}
