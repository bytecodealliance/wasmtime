#![doc(hidden)]

use crate::instance::InstanceHandle;
use crate::vmcontext::VMContext;
use wasmtime_environ::{EntityRef, MemoryIndex};

static mut VMCTX_AND_MEMORY: (*mut VMContext, usize) = (std::ptr::null_mut(), 0);

#[no_mangle]
pub unsafe extern "C" fn resolve_vmctx_memory(ptr: usize) -> *const u8 {
    let handle = InstanceHandle::from_vmctx(VMCTX_AND_MEMORY.0);
    assert!(
        VMCTX_AND_MEMORY.1 < handle.module().memory_plans.len(),
        "memory index for debugger is out of bounds"
    );
    let index = MemoryIndex::new(VMCTX_AND_MEMORY.1);
    let mem = handle.instance().get_memory(index);    
    mem.base.add(ptr)
}


#[no_mangle]
pub unsafe extern "C" fn resolve_vmctx_memory_ptr(p: *const u32) -> *const u8 {
    let ptr = std::ptr::read(p);
    assert!(
        !VMCTX_AND_MEMORY.0.is_null(),
        "must call `__vmctx->set()` before resolving Wasm pointers"
    );
    let handle = InstanceHandle::from_vmctx(VMCTX_AND_MEMORY.0);
    assert!(
        VMCTX_AND_MEMORY.1 < handle.module().memory_plans.len(),
        "memory index for debugger is out of bounds"
    );
    let index = MemoryIndex::new(VMCTX_AND_MEMORY.1);
    let mem = handle.instance().get_memory(index);
    mem.base.add(ptr as usize)
}

#[no_mangle]
pub unsafe extern "C" fn set_vmctx_memory(vmctx_ptr: *mut VMContext) {
    // TODO multi-memory
    VMCTX_AND_MEMORY = (vmctx_ptr, 0);
}

// Ensures that set_vmctx_memory and resolve_vmctx_memory_ptr are linked and
// exported as symbols. It is a workaround: the executable normally ignores
// `pub extern "C"`, see rust-lang/rust#25057.
pub fn ensure_exported() {
    unsafe {
        std::ptr::read_volatile(resolve_vmctx_memory_ptr as *const u8);
        std::ptr::read_volatile(set_vmctx_memory as *const u8);
        std::ptr::read_volatile(resolve_vmctx_memory as *const u8);
    }
}
