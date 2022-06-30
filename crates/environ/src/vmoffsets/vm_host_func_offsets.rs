// Currently the `VMHostFuncContext` allocation by field looks like this:
//
// struct VMHostFuncContext {
//      magic: u32,
//      _padding: u32, // (on 64-bit systems)
//      host_func: NonNull<VMFunctionBody>,
//      wasm_to_host_trampoline: VMCallerCheckedAnyfunc,
//      host_state: Box<dyn Any + Send + Sync>,
// }
//
// Keep this in sync with `wasmtime_runtime::VMHostFuncContext`.

use crate::PtrSize;

/// Equivalent of `VMCONTEXT_MAGIC` except for host functions.
///
/// This is stored at the start of all `VMHostFuncContext` structures and
/// double-checked on `VMHostFuncContext::from_opaque`.
pub const VM_HOST_FUNC_MAGIC: u32 = u32::from_le_bytes(*b"host");

/// Runtime offsets within a `VMHostFuncContext`.
///
/// These offsets are the same for every host function.
#[derive(Debug, Clone, Copy)]
pub struct VMHostFuncOffsets<P> {
    /// The host pointer size
    pub ptr: P,

    // precalculated offsets of various member fields
    magic: u32,
    host_func: u32,
    wasm_to_host_trampoline: u32,
    host_state: u32,
}

impl<P: PtrSize> VMHostFuncOffsets<P> {
    /// Creates a new set of offsets.
    pub fn new(ptr: P) -> Self {
        let magic = 0;
        let host_func = super::align(
            u32::try_from(std::mem::size_of::<u32>()).unwrap(),
            ptr.size().into(),
        );
        let wasm_to_host_trampoline = host_func + u32::from(ptr.size());
        let host_state =
            wasm_to_host_trampoline + u32::from(ptr.size_of_vmcaller_checked_anyfunc());
        Self {
            ptr,
            magic,
            host_func,
            wasm_to_host_trampoline,
            host_state,
        }
    }

    /// The size, in bytes, of the host pointer.
    #[inline]
    pub fn pointer_size(&self) -> u8 {
        self.ptr.size()
    }

    /// The offset of the `magic` field.
    #[inline]
    pub fn magic(&self) -> u32 {
        self.magic
    }

    /// The offset of the `host_func` field.
    #[inline]
    pub fn host_func(&self) -> u32 {
        self.host_func
    }

    /// The offset of the `wasm_to_host_trampoline` field.
    #[inline]
    pub fn wasm_to_host_trampoline(&self) -> u32 {
        self.wasm_to_host_trampoline
    }

    /// The offset of the `host_state` field.
    #[inline]
    pub fn host_state(&self) -> u32 {
        self.host_state
    }
}
