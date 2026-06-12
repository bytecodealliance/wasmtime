use core::alloc::Layout;
use std::alloc::{alloc, dealloc};
use std::vec::Vec;

use cranelift_module::{ModuleError, ModuleResult};

use super::{BranchProtection, JITMemoryKind, JITMemoryProvider};

/// A memory provider that stores allocations in heap-allocated `Vec`s
/// without applying any memory protections.
///
/// This is useful for dumping compiled code (e.g. for shellcode generation)
/// where the code will not be executed in-process. Memory is leaked by
/// default to keep returned pointers valid; call
/// [`JITMemoryProvider::free_memory`] to explicitly deallocate.
pub struct VecMemoryProvider {
    allocations: Vec<Allocation>,
}

struct Allocation {
    ptr: *mut u8,
    layout: Layout,
}

unsafe impl Send for VecMemoryProvider {}

impl VecMemoryProvider {
    /// Create a new `VecMemoryProvider`.
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct AllocFailed;

impl core::fmt::Display for AllocFailed {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str("memory allocation failed")
    }
}

impl core::error::Error for AllocFailed {}

impl JITMemoryProvider for VecMemoryProvider {
    fn allocate(&mut self, size: usize, align: u64, _kind: JITMemoryKind) -> ModuleResult<*mut u8> {
        let align = usize::try_from(align).expect("alignment too big").max(1);
        let size = size.max(1);
        let layout = Layout::from_size_align(size, align).map_err(ModuleError::allocation)?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(ModuleError::allocation(AllocFailed));
        }

        self.allocations.push(Allocation { ptr, layout });
        Ok(ptr)
    }

    unsafe fn free_memory(&mut self) {
        for alloc in self.allocations.drain(..) {
            unsafe { dealloc(alloc.ptr, alloc.layout) };
        }
    }

    fn finalize(&mut self, _branch_protection: BranchProtection) -> ModuleResult<()> {
        Ok(())
    }
}

impl Drop for VecMemoryProvider {
    fn drop(&mut self) {
        // Intentionally leak memory to keep function pointers valid,
        // matching the behavior of SystemMemoryProvider and ArenaMemoryProvider.
        // Call `free_memory()` to explicitly deallocate.
    }
}
