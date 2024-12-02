#![doc(hidden)]

use crate::runtime::vm::instance::Instance;
use crate::runtime::vm::vmcontext::VMContext;
use wasmtime_environ::{EntityRef, MemoryIndex};
use wasmtime_versioned_export_macros::versioned_export;

static mut VMCTX_AND_MEMORY: (*mut VMContext, usize) = (std::ptr::null_mut(), 0);

// These implementatations are referenced from C code in "helpers.c". The symbols defined
// there (prefixed by "wasmtime_") are the real 'public' interface used in the debug info.

#[versioned_export]
pub unsafe extern "C" fn resolve_vmctx_memory_ptr(p: *const u32) -> *const u8 {
    let ptr = std::ptr::read(p);
    assert!(
        !VMCTX_AND_MEMORY.0.is_null(),
        "must call `__vmctx->set()` before resolving Wasm pointers"
    );
    Instance::from_vmctx(VMCTX_AND_MEMORY.0, |handle| {
        assert!(
            VMCTX_AND_MEMORY.1 < handle.env_module().memories.len(),
            "memory index for debugger is out of bounds"
        );
        let index = MemoryIndex::new(VMCTX_AND_MEMORY.1);
        let mem = handle.get_memory(index);
        mem.base.add(ptr as usize)
    })
}

#[versioned_export]
pub unsafe extern "C" fn set_vmctx_memory(vmctx_ptr: *mut VMContext) {
    // TODO multi-memory
    VMCTX_AND_MEMORY = (vmctx_ptr, 0);
}
