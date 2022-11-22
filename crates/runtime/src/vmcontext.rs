//! This file declares `VMContext` and several related structs which contain
//! fields that compiled wasm code accesses directly.

mod vm_host_func_context;

use crate::externref::VMExternRef;
use crate::instance::Instance;
use std::any::Any;
use std::cell::UnsafeCell;
use std::marker;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::u32;
pub use vm_host_func_context::VMHostFuncContext;
use wasmtime_environ::DefinedMemoryIndex;

pub const VMCONTEXT_MAGIC: u32 = u32::from_le_bytes(*b"core");

/// An imported function.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMFunctionImport {
    /// A pointer to the imported function body.
    pub body: NonNull<VMFunctionBody>,

    /// The VM state associated with this function.
    ///
    /// For core wasm instances this will be `*mut VMContext` but for the
    /// upcoming implementation of the component model this will be something
    /// else. The actual definition of what this pointer points to depends on
    /// the definition of `func_ptr` and what compiled it.
    pub vmctx: *mut VMOpaqueContext,
}

// Declare that this type is send/sync, it's the responsibility of users of
// `VMFunctionImport` to uphold this guarantee.
unsafe impl Send for VMFunctionImport {}
unsafe impl Sync for VMFunctionImport {}

#[cfg(test)]
mod test_vmfunction_import {
    use super::VMFunctionImport;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmfunction_import_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMFunctionImport>(),
            usize::from(offsets.size_of_vmfunction_import())
        );
        assert_eq!(
            offset_of!(VMFunctionImport, body),
            usize::from(offsets.vmfunction_import_body())
        );
        assert_eq!(
            offset_of!(VMFunctionImport, vmctx),
            usize::from(offsets.vmfunction_import_vmctx())
        );
    }
}

/// A placeholder byte-sized type which is just used to provide some amount of type
/// safety when dealing with pointers to JIT-compiled function bodies. Note that it's
/// deliberately not Copy, as we shouldn't be carelessly copying function body bytes
/// around.
#[repr(C)]
pub struct VMFunctionBody(u8);

#[cfg(test)]
mod test_vmfunction_body {
    use super::VMFunctionBody;
    use std::mem::size_of;

    #[test]
    fn check_vmfunction_body_offsets() {
        assert_eq!(size_of::<VMFunctionBody>(), 1);
    }
}

/// The fields compiled code needs to access to utilize a WebAssembly table
/// imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableImport {
    /// A pointer to the imported table description.
    pub from: *mut VMTableDefinition,

    /// A pointer to the `VMContext` that owns the table description.
    pub vmctx: *mut VMContext,
}

// Declare that this type is send/sync, it's the responsibility of users of
// `VMTableImport` to uphold this guarantee.
unsafe impl Send for VMTableImport {}
unsafe impl Sync for VMTableImport {}

#[cfg(test)]
mod test_vmtable_import {
    use super::VMTableImport;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmtable_import_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMTableImport>(),
            usize::from(offsets.size_of_vmtable_import())
        );
        assert_eq!(
            offset_of!(VMTableImport, from),
            usize::from(offsets.vmtable_import_from())
        );
        assert_eq!(
            offset_of!(VMTableImport, vmctx),
            usize::from(offsets.vmtable_import_vmctx())
        );
    }
}

/// The fields compiled code needs to access to utilize a WebAssembly linear
/// memory imported from another instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMMemoryImport {
    /// A pointer to the imported memory description.
    pub from: *mut VMMemoryDefinition,

    /// A pointer to the `VMContext` that owns the memory description.
    pub vmctx: *mut VMContext,

    /// The index of the memory in the containing `vmctx`.
    pub index: DefinedMemoryIndex,
}

// Declare that this type is send/sync, it's the responsibility of users of
// `VMMemoryImport` to uphold this guarantee.
unsafe impl Send for VMMemoryImport {}
unsafe impl Sync for VMMemoryImport {}

#[cfg(test)]
mod test_vmmemory_import {
    use super::VMMemoryImport;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmmemory_import_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMMemoryImport>(),
            usize::from(offsets.size_of_vmmemory_import())
        );
        assert_eq!(
            offset_of!(VMMemoryImport, from),
            usize::from(offsets.vmmemory_import_from())
        );
        assert_eq!(
            offset_of!(VMMemoryImport, vmctx),
            usize::from(offsets.vmmemory_import_vmctx())
        );
    }
}

/// The fields compiled code needs to access to utilize a WebAssembly global
/// variable imported from another instance.
///
/// Note that unlike with functions, tables, and memories, `VMGlobalImport`
/// doesn't include a `vmctx` pointer. Globals are never resized, and don't
/// require a `vmctx` pointer to access.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMGlobalImport {
    /// A pointer to the imported global variable description.
    pub from: *mut VMGlobalDefinition,
}

// Declare that this type is send/sync, it's the responsibility of users of
// `VMGlobalImport` to uphold this guarantee.
unsafe impl Send for VMGlobalImport {}
unsafe impl Sync for VMGlobalImport {}

#[cfg(test)]
mod test_vmglobal_import {
    use super::VMGlobalImport;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmglobal_import_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMGlobalImport>(),
            usize::from(offsets.size_of_vmglobal_import())
        );
        assert_eq!(
            offset_of!(VMGlobalImport, from),
            usize::from(offsets.vmglobal_import_from())
        );
    }
}

/// The fields compiled code needs to access to utilize a WebAssembly linear
/// memory defined within the instance, namely the start address and the
/// size in bytes.
#[derive(Debug)]
#[repr(C)]
pub struct VMMemoryDefinition {
    /// The start address.
    pub base: *mut u8,

    /// The current logical size of this linear memory in bytes.
    ///
    /// This is atomic because shared memories must be able to grow their length
    /// atomically. For relaxed access, see
    /// [`VMMemoryDefinition::current_length()`].
    pub current_length: AtomicUsize,
}

impl VMMemoryDefinition {
    /// Return the current length of the [`VMMemoryDefinition`] by performing a
    /// relaxed load; do not use this function for situations in which a precise
    /// length is needed. Owned memories (i.e., non-shared) will always return a
    /// precise result (since no concurrent modification is possible) but shared
    /// memories may see an imprecise value--a `current_length` potentially
    /// smaller than what some other thread observes. Since Wasm memory only
    /// grows, this under-estimation may be acceptable in certain cases.
    pub fn current_length(&self) -> usize {
        self.current_length.load(Ordering::Relaxed)
    }

    /// Return a copy of the [`VMMemoryDefinition`] using the relaxed value of
    /// `current_length`; see [`VMMemoryDefinition::current_length()`].
    pub unsafe fn load(ptr: *mut Self) -> Self {
        let other = &*ptr;
        VMMemoryDefinition {
            base: other.base,
            current_length: other.current_length().into(),
        }
    }
}

#[cfg(test)]
mod test_vmmemory_definition {
    use super::VMMemoryDefinition;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn check_vmmemory_definition_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMMemoryDefinition>(),
            usize::from(offsets.ptr.size_of_vmmemory_definition())
        );
        assert_eq!(
            offset_of!(VMMemoryDefinition, base),
            usize::from(offsets.ptr.vmmemory_definition_base())
        );
        assert_eq!(
            offset_of!(VMMemoryDefinition, current_length),
            usize::from(offsets.ptr.vmmemory_definition_current_length())
        );
        /* TODO: Assert that the size of `current_length` matches.
        assert_eq!(
            size_of::<VMMemoryDefinition::current_length>(),
            usize::from(offsets.size_of_vmmemory_definition_current_length())
        );
        */
    }
}

/// The fields compiled code needs to access to utilize a WebAssembly table
/// defined within the instance.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMTableDefinition {
    /// Pointer to the table data.
    pub base: *mut u8,

    /// The current number of elements in the table.
    pub current_elements: u32,
}

#[cfg(test)]
mod test_vmtable_definition {
    use super::VMTableDefinition;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmtable_definition_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMTableDefinition>(),
            usize::from(offsets.size_of_vmtable_definition())
        );
        assert_eq!(
            offset_of!(VMTableDefinition, base),
            usize::from(offsets.vmtable_definition_base())
        );
        assert_eq!(
            offset_of!(VMTableDefinition, current_elements),
            usize::from(offsets.vmtable_definition_current_elements())
        );
    }
}

/// The storage for a WebAssembly global defined within the instance.
///
/// TODO: Pack the globals more densely, rather than using the same size
/// for every type.
#[derive(Debug)]
#[repr(C, align(16))]
pub struct VMGlobalDefinition {
    storage: [u8; 16],
    // If more elements are added here, remember to add offset_of tests below!
}

#[cfg(test)]
mod test_vmglobal_definition {
    use super::VMGlobalDefinition;
    use crate::externref::VMExternRef;
    use std::mem::{align_of, size_of};
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn check_vmglobal_definition_alignment() {
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<i32>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<i64>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<f32>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<f64>());
        assert!(align_of::<VMGlobalDefinition>() >= align_of::<[u8; 16]>());
    }

    #[test]
    fn check_vmglobal_definition_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMGlobalDefinition>(),
            usize::from(offsets.ptr.size_of_vmglobal_definition())
        );
    }

    #[test]
    fn check_vmglobal_begins_aligned() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(offsets.vmctx_globals_begin() % 16, 0);
    }

    #[test]
    fn check_vmglobal_can_contain_externref() {
        assert!(size_of::<VMExternRef>() <= size_of::<VMGlobalDefinition>());
    }
}

impl VMGlobalDefinition {
    /// Construct a `VMGlobalDefinition`.
    pub fn new() -> Self {
        Self { storage: [0; 16] }
    }

    /// Return a reference to the value as an i32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i32(&self) -> &i32 {
        &*(self.storage.as_ref().as_ptr().cast::<i32>())
    }

    /// Return a mutable reference to the value as an i32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i32_mut(&mut self) -> &mut i32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<i32>())
    }

    /// Return a reference to the value as a u32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u32(&self) -> &u32 {
        &*(self.storage.as_ref().as_ptr().cast::<u32>())
    }

    /// Return a mutable reference to the value as an u32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u32_mut(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u32>())
    }

    /// Return a reference to the value as an i64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i64(&self) -> &i64 {
        &*(self.storage.as_ref().as_ptr().cast::<i64>())
    }

    /// Return a mutable reference to the value as an i64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_i64_mut(&mut self) -> &mut i64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<i64>())
    }

    /// Return a reference to the value as an u64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u64(&self) -> &u64 {
        &*(self.storage.as_ref().as_ptr().cast::<u64>())
    }

    /// Return a mutable reference to the value as an u64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u64_mut(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u64>())
    }

    /// Return a reference to the value as an f32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32(&self) -> &f32 {
        &*(self.storage.as_ref().as_ptr().cast::<f32>())
    }

    /// Return a mutable reference to the value as an f32.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_mut(&mut self) -> &mut f32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<f32>())
    }

    /// Return a reference to the value as f32 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_bits(&self) -> &u32 {
        &*(self.storage.as_ref().as_ptr().cast::<u32>())
    }

    /// Return a mutable reference to the value as f32 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f32_bits_mut(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u32>())
    }

    /// Return a reference to the value as an f64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64(&self) -> &f64 {
        &*(self.storage.as_ref().as_ptr().cast::<f64>())
    }

    /// Return a mutable reference to the value as an f64.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_mut(&mut self) -> &mut f64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<f64>())
    }

    /// Return a reference to the value as f64 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_bits(&self) -> &u64 {
        &*(self.storage.as_ref().as_ptr().cast::<u64>())
    }

    /// Return a mutable reference to the value as f64 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_f64_bits_mut(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u64>())
    }

    /// Return a reference to the value as an u128.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u128(&self) -> &u128 {
        &*(self.storage.as_ref().as_ptr().cast::<u128>())
    }

    /// Return a mutable reference to the value as an u128.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u128_mut(&mut self) -> &mut u128 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u128>())
    }

    /// Return a reference to the value as u128 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u128_bits(&self) -> &[u8; 16] {
        &*(self.storage.as_ref().as_ptr().cast::<[u8; 16]>())
    }

    /// Return a mutable reference to the value as u128 bits.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_u128_bits_mut(&mut self) -> &mut [u8; 16] {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<[u8; 16]>())
    }

    /// Return a reference to the value as an externref.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_externref(&self) -> &Option<VMExternRef> {
        &*(self.storage.as_ref().as_ptr().cast::<Option<VMExternRef>>())
    }

    /// Return a mutable reference to the value as an externref.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_externref_mut(&mut self) -> &mut Option<VMExternRef> {
        &mut *(self
            .storage
            .as_mut()
            .as_mut_ptr()
            .cast::<Option<VMExternRef>>())
    }

    /// Return a reference to the value as an anyfunc.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_anyfunc(&self) -> *const VMCallerCheckedAnyfunc {
        *(self
            .storage
            .as_ref()
            .as_ptr()
            .cast::<*const VMCallerCheckedAnyfunc>())
    }

    /// Return a mutable reference to the value as an anyfunc.
    #[allow(clippy::cast_ptr_alignment)]
    pub unsafe fn as_anyfunc_mut(&mut self) -> &mut *const VMCallerCheckedAnyfunc {
        &mut *(self
            .storage
            .as_mut()
            .as_mut_ptr()
            .cast::<*const VMCallerCheckedAnyfunc>())
    }
}

/// An index into the shared signature registry, usable for checking signatures
/// at indirect calls.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct VMSharedSignatureIndex(u32);

#[cfg(test)]
mod test_vmshared_signature_index {
    use super::VMSharedSignatureIndex;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmshared_signature_index() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMSharedSignatureIndex>(),
            usize::from(offsets.size_of_vmshared_signature_index())
        );
    }
}

impl VMSharedSignatureIndex {
    /// Create a new `VMSharedSignatureIndex`.
    #[inline]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the underlying bits of the index.
    #[inline]
    pub fn bits(&self) -> u32 {
        self.0
    }
}

impl Default for VMSharedSignatureIndex {
    #[inline]
    fn default() -> Self {
        Self::new(u32::MAX)
    }
}

/// The VM caller-checked "anyfunc" record, for caller-side signature checking.
/// It consists of the actual function pointer and a signature id to be checked
/// by the caller.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VMCallerCheckedAnyfunc {
    /// Function body.
    pub func_ptr: NonNull<VMFunctionBody>,
    /// Function signature id.
    pub type_index: VMSharedSignatureIndex,
    /// The VM state associated with this function.
    ///
    /// For core wasm instances this will be `*mut VMContext` but for the
    /// upcoming implementation of the component model this will be something
    /// else. The actual definition of what this pointer points to depends on
    /// the definition of `func_ptr` and what compiled it.
    pub vmctx: *mut VMOpaqueContext,
    // If more elements are added here, remember to add offset_of tests below!
}

unsafe impl Send for VMCallerCheckedAnyfunc {}
unsafe impl Sync for VMCallerCheckedAnyfunc {}

#[cfg(test)]
mod test_vmcaller_checked_anyfunc {
    use super::VMCallerCheckedAnyfunc;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn check_vmcaller_checked_anyfunc_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMCallerCheckedAnyfunc>(),
            usize::from(offsets.ptr.size_of_vmcaller_checked_anyfunc())
        );
        assert_eq!(
            offset_of!(VMCallerCheckedAnyfunc, func_ptr),
            usize::from(offsets.ptr.vmcaller_checked_anyfunc_func_ptr())
        );
        assert_eq!(
            offset_of!(VMCallerCheckedAnyfunc, type_index),
            usize::from(offsets.ptr.vmcaller_checked_anyfunc_type_index())
        );
        assert_eq!(
            offset_of!(VMCallerCheckedAnyfunc, vmctx),
            usize::from(offsets.ptr.vmcaller_checked_anyfunc_vmctx())
        );
    }
}

macro_rules! define_builtin_array {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
        )*
    ) => {
        /// An array that stores addresses of builtin functions. We translate code
        /// to use indirect calls. This way, we don't have to patch the code.
        #[repr(C)]
        pub struct VMBuiltinFunctionsArray {
            $(
                $name: unsafe extern "C" fn(
                    $(define_builtin_array!(@ty $param)),*
                ) $( -> define_builtin_array!(@ty $result))?,
            )*
        }

        impl VMBuiltinFunctionsArray {
            pub const INIT: VMBuiltinFunctionsArray = VMBuiltinFunctionsArray {
                $($name: crate::libcalls::trampolines::$name,)*
            };
        }
    };

    (@ty i32) => (u32);
    (@ty i64) => (u64);
    (@ty reference) => (*mut u8);
    (@ty pointer) => (*mut u8);
    (@ty vmctx) => (*mut VMContext);
}

wasmtime_environ::foreach_builtin_function!(define_builtin_array);

/// The storage for a WebAssembly invocation argument
///
/// TODO: These could be packed more densely, rather than using the same size for every type.
#[derive(Debug, Copy, Clone)]
#[repr(C, align(16))]
pub struct VMInvokeArgument([u8; 16]);

#[cfg(test)]
mod test_vm_invoke_argument {
    use super::VMInvokeArgument;
    use std::mem::{align_of, size_of};
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn check_vm_invoke_argument_alignment() {
        assert_eq!(align_of::<VMInvokeArgument>(), 16);
    }

    #[test]
    fn check_vmglobal_definition_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMInvokeArgument>(),
            usize::from(offsets.ptr.size_of_vmglobal_definition())
        );
    }
}

impl VMInvokeArgument {
    /// Create a new invocation argument filled with zeroes
    pub fn new() -> Self {
        Self([0; 16])
    }
}

/// Structure used to control interrupting wasm code.
#[derive(Debug)]
#[repr(C)]
pub struct VMRuntimeLimits {
    /// Current stack limit of the wasm module.
    ///
    /// For more information see `crates/cranelift/src/lib.rs`.
    pub stack_limit: UnsafeCell<usize>,

    /// Indicator of how much fuel has been consumed and is remaining to
    /// WebAssembly.
    ///
    /// This field is typically negative and increments towards positive. Upon
    /// turning positive a wasm trap will be generated. This field is only
    /// modified if wasm is configured to consume fuel.
    pub fuel_consumed: UnsafeCell<i64>,

    /// Deadline epoch for interruption: if epoch-based interruption
    /// is enabled and the global (per engine) epoch counter is
    /// observed to reach or exceed this value, the guest code will
    /// yield if running asynchronously.
    pub epoch_deadline: UnsafeCell<u64>,

    /// The value of the frame pointer register when we last called from Wasm to
    /// the host.
    ///
    /// Maintained by our Wasm-to-host trampoline, and cleared just before
    /// calling into Wasm in `catch_traps`.
    ///
    /// This member is `0` when Wasm is actively running and has not called out
    /// to the host.
    ///
    /// Used to find the start of a a contiguous sequence of Wasm frames when
    /// walking the stack.
    pub last_wasm_exit_fp: UnsafeCell<usize>,

    /// The last Wasm program counter before we called from Wasm to the host.
    ///
    /// Maintained by our Wasm-to-host trampoline, and cleared just before
    /// calling into Wasm in `catch_traps`.
    ///
    /// This member is `0` when Wasm is actively running and has not called out
    /// to the host.
    ///
    /// Used when walking a contiguous sequence of Wasm frames.
    pub last_wasm_exit_pc: UnsafeCell<usize>,

    /// The last host stack pointer before we called into Wasm from the host.
    ///
    /// Maintained by our host-to-Wasm trampoline, and cleared just before
    /// calling into Wasm in `catch_traps`.
    ///
    /// This member is `0` when Wasm is actively running and has not called out
    /// to the host.
    ///
    /// When a host function is wrapped into a `wasmtime::Func`, and is then
    /// called from the host, then this member has the sentinal value of `-1 as
    /// usize`, meaning that this contiguous sequence of Wasm frames is the
    /// empty sequence, and it is not safe to dereference the
    /// `last_wasm_exit_fp`.
    ///
    /// Used to find the end of a contiguous sequence of Wasm frames when
    /// walking the stack.
    pub last_wasm_entry_sp: UnsafeCell<usize>,
}

// The `VMRuntimeLimits` type is a pod-type with no destructor, and we don't
// access any fields from other threads, so add in these trait impls which are
// otherwise not available due to the `fuel_consumed` and `epoch_deadline`
// variables in `VMRuntimeLimits`.
unsafe impl Send for VMRuntimeLimits {}
unsafe impl Sync for VMRuntimeLimits {}

impl Default for VMRuntimeLimits {
    fn default() -> VMRuntimeLimits {
        VMRuntimeLimits {
            stack_limit: UnsafeCell::new(usize::max_value()),
            fuel_consumed: UnsafeCell::new(0),
            epoch_deadline: UnsafeCell::new(0),
            last_wasm_exit_fp: UnsafeCell::new(0),
            last_wasm_exit_pc: UnsafeCell::new(0),
            last_wasm_entry_sp: UnsafeCell::new(0),
        }
    }
}

#[cfg(test)]
mod test_vmruntime_limits {
    use super::VMRuntimeLimits;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn field_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            offset_of!(VMRuntimeLimits, stack_limit),
            usize::from(offsets.ptr.vmruntime_limits_stack_limit())
        );
        assert_eq!(
            offset_of!(VMRuntimeLimits, fuel_consumed),
            usize::from(offsets.ptr.vmruntime_limits_fuel_consumed())
        );
        assert_eq!(
            offset_of!(VMRuntimeLimits, epoch_deadline),
            usize::from(offsets.ptr.vmruntime_limits_epoch_deadline())
        );
        assert_eq!(
            offset_of!(VMRuntimeLimits, last_wasm_exit_fp),
            usize::from(offsets.ptr.vmruntime_limits_last_wasm_exit_fp())
        );
        assert_eq!(
            offset_of!(VMRuntimeLimits, last_wasm_exit_pc),
            usize::from(offsets.ptr.vmruntime_limits_last_wasm_exit_pc())
        );
        assert_eq!(
            offset_of!(VMRuntimeLimits, last_wasm_entry_sp),
            usize::from(offsets.ptr.vmruntime_limits_last_wasm_entry_sp())
        );
    }
}

/// The VM "context", which is pointed to by the `vmctx` arg in Cranelift.
/// This has information about globals, memories, tables, and other runtime
/// state associated with the current instance.
///
/// The struct here is empty, as the sizes of these fields are dynamic, and
/// we can't describe them in Rust's type system. Sufficient memory is
/// allocated at runtime.
#[derive(Debug)]
#[repr(C, align(16))] // align 16 since globals are aligned to that and contained inside
pub struct VMContext {
    /// There's some more discussion about this within `wasmtime/src/lib.rs` but
    /// the idea is that we want to tell the compiler that this contains
    /// pointers which transitively refers to itself, to suppress some
    /// optimizations that might otherwise assume this doesn't exist.
    ///
    /// The self-referential pointer we care about is the `*mut Store` pointer
    /// early on in this context, which if you follow through enough levels of
    /// nesting, eventually can refer back to this `VMContext`
    pub _marker: marker::PhantomPinned,
}

impl VMContext {
    /// Helper function to cast between context types using a debug assertion to
    /// protect against some mistakes.
    #[inline]
    pub unsafe fn from_opaque(opaque: *mut VMOpaqueContext) -> *mut VMContext {
        // Note that in general the offset of the "magic" field is stored in
        // `VMOffsets::vmctx_magic`. Given though that this is a sanity check
        // about converting this pointer to another type we ideally don't want
        // to read the offset from potentially corrupt memory. Instead it would
        // be better to catch errors here as soon as possible.
        //
        // To accomplish this the `VMContext` structure is laid out with the
        // magic field at a statically known offset (here it's 0 for now). This
        // static offset is asserted in `VMOffsets::from` and needs to be kept
        // in sync with this line for this debug assertion to work.
        //
        // Also note that this magic is only ever invalid in the presence of
        // bugs, meaning we don't actually read the magic and act differently
        // at runtime depending what it is, so this is a debug assertion as
        // opposed to a regular assertion.
        debug_assert_eq!((*opaque).magic, VMCONTEXT_MAGIC);
        opaque.cast()
    }

    /// Return a mutable reference to the associated `Instance`.
    ///
    /// # Safety
    /// This is unsafe because it doesn't work on just any `VMContext`, it must
    /// be a `VMContext` allocated as part of an `Instance`.
    #[allow(clippy::cast_ptr_alignment)]
    #[inline]
    pub(crate) unsafe fn instance(&self) -> &Instance {
        &*((self as *const Self as *mut u8).offset(-Instance::vmctx_offset()) as *const Instance)
    }

    #[inline]
    pub(crate) unsafe fn instance_mut(&mut self) -> &mut Instance {
        &mut *((self as *const Self as *mut u8).offset(-Instance::vmctx_offset()) as *mut Instance)
    }

    /// Return a reference to the host state associated with this `Instance`.
    ///
    /// # Safety
    /// This is unsafe because it doesn't work on just any `VMContext`, it must
    /// be a `VMContext` allocated as part of an `Instance`.
    #[inline]
    pub unsafe fn host_state(&self) -> &dyn Any {
        self.instance().host_state()
    }
}

/// A "raw" and unsafe representation of a WebAssembly value.
///
/// This is provided for use with the `Func::new_unchecked` and
/// `Func::call_unchecked` APIs. In general it's unlikely you should be using
/// this from Rust, rather using APIs like `Func::wrap` and `TypedFunc::call`.
///
/// This is notably an "unsafe" way to work with `Val` and it's recommended to
/// instead use `Val` where possible. An important note about this union is that
/// fields are all stored in little-endian format, regardless of the endianness
/// of the host system.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Copy, Clone)]
pub union ValRaw {
    /// A WebAssembly `i32` value.
    ///
    /// Note that the payload here is a Rust `i32` but the WebAssembly `i32`
    /// type does not assign an interpretation of the upper bit as either signed
    /// or unsigned. The Rust type `i32` is simply chosen for convenience.
    ///
    /// This value is always stored in a little-endian format.
    i32: i32,

    /// A WebAssembly `i64` value.
    ///
    /// Note that the payload here is a Rust `i64` but the WebAssembly `i64`
    /// type does not assign an interpretation of the upper bit as either signed
    /// or unsigned. The Rust type `i64` is simply chosen for convenience.
    ///
    /// This value is always stored in a little-endian format.
    i64: i64,

    /// A WebAssembly `f32` value.
    ///
    /// Note that the payload here is a Rust `u32`. This is to allow passing any
    /// representation of NaN into WebAssembly without risk of changing NaN
    /// payload bits as its gets passed around the system. Otherwise though this
    /// `u32` value is the return value of `f32::to_bits` in Rust.
    ///
    /// This value is always stored in a little-endian format.
    f32: u32,

    /// A WebAssembly `f64` value.
    ///
    /// Note that the payload here is a Rust `u64`. This is to allow passing any
    /// representation of NaN into WebAssembly without risk of changing NaN
    /// payload bits as its gets passed around the system. Otherwise though this
    /// `u64` value is the return value of `f64::to_bits` in Rust.
    ///
    /// This value is always stored in a little-endian format.
    f64: u64,

    /// A WebAssembly `v128` value.
    ///
    /// The payload here is a Rust `u128` which has the same number of bits but
    /// note that `v128` in WebAssembly is often considered a vector type such
    /// as `i32x4` or `f64x2`. This means that the actual interpretation of the
    /// underlying bits is left up to the instructions which consume this value.
    ///
    /// This value is always stored in a little-endian format.
    v128: u128,

    /// A WebAssembly `funcref` value.
    ///
    /// The payload here is a pointer which is runtime-defined. This is one of
    /// the main points of unsafety about the `ValRaw` type as the validity of
    /// the pointer here is not easily verified and must be preserved by
    /// carefully calling the correct functions throughout the runtime.
    ///
    /// This value is always stored in a little-endian format.
    funcref: usize,

    /// A WebAssembly `externref` value.
    ///
    /// The payload here is a pointer which is runtime-defined. This is one of
    /// the main points of unsafety about the `ValRaw` type as the validity of
    /// the pointer here is not easily verified and must be preserved by
    /// carefully calling the correct functions throughout the runtime.
    ///
    /// This value is always stored in a little-endian format.
    externref: usize,
}

impl ValRaw {
    /// Creates a WebAssembly `i32` value
    #[inline]
    pub fn i32(i: i32) -> ValRaw {
        // Note that this is intentionally not setting the `i32` field, instead
        // setting the `i64` field with a zero-extended version of `i`. For more
        // information on this see the comments on `Lower for Result` in the
        // `wasmtime` crate. Otherwise though all `ValRaw` constructors are
        // otherwise constrained to guarantee that the initial 64-bits are
        // always initialized.
        ValRaw::u64((i as u32).into())
    }

    /// Creates a WebAssembly `i64` value
    #[inline]
    pub fn i64(i: i64) -> ValRaw {
        ValRaw { i64: i.to_le() }
    }

    /// Creates a WebAssembly `i32` value
    #[inline]
    pub fn u32(i: u32) -> ValRaw {
        // See comments in `ValRaw::i32` for why this is setting the upper
        // 32-bits as well.
        ValRaw::u64(i.into())
    }

    /// Creates a WebAssembly `i64` value
    #[inline]
    pub fn u64(i: u64) -> ValRaw {
        ValRaw::i64(i as i64)
    }

    /// Creates a WebAssembly `f32` value
    #[inline]
    pub fn f32(i: u32) -> ValRaw {
        // See comments in `ValRaw::i32` for why this is setting the upper
        // 32-bits as well.
        ValRaw::u64(i.into())
    }

    /// Creates a WebAssembly `f64` value
    #[inline]
    pub fn f64(i: u64) -> ValRaw {
        ValRaw { f64: i.to_le() }
    }

    /// Creates a WebAssembly `v128` value
    #[inline]
    pub fn v128(i: u128) -> ValRaw {
        ValRaw { v128: i.to_le() }
    }

    /// Creates a WebAssembly `funcref` value
    #[inline]
    pub fn funcref(i: usize) -> ValRaw {
        ValRaw { funcref: i.to_le() }
    }

    /// Creates a WebAssembly `externref` value
    #[inline]
    pub fn externref(i: usize) -> ValRaw {
        ValRaw {
            externref: i.to_le(),
        }
    }

    /// Gets the WebAssembly `i32` value
    #[inline]
    pub fn get_i32(&self) -> i32 {
        unsafe { i32::from_le(self.i32) }
    }

    /// Gets the WebAssembly `i64` value
    #[inline]
    pub fn get_i64(&self) -> i64 {
        unsafe { i64::from_le(self.i64) }
    }

    /// Gets the WebAssembly `i32` value
    #[inline]
    pub fn get_u32(&self) -> u32 {
        self.get_i32() as u32
    }

    /// Gets the WebAssembly `i64` value
    #[inline]
    pub fn get_u64(&self) -> u64 {
        self.get_i64() as u64
    }

    /// Gets the WebAssembly `f32` value
    #[inline]
    pub fn get_f32(&self) -> u32 {
        unsafe { u32::from_le(self.f32) }
    }

    /// Gets the WebAssembly `f64` value
    #[inline]
    pub fn get_f64(&self) -> u64 {
        unsafe { u64::from_le(self.f64) }
    }

    /// Gets the WebAssembly `v128` value
    #[inline]
    pub fn get_v128(&self) -> u128 {
        unsafe { u128::from_le(self.v128) }
    }

    /// Gets the WebAssembly `funcref` value
    #[inline]
    pub fn get_funcref(&self) -> usize {
        unsafe { usize::from_le(self.funcref) }
    }

    /// Gets the WebAssembly `externref` value
    #[inline]
    pub fn get_externref(&self) -> usize {
        unsafe { usize::from_le(self.externref) }
    }
}

/// Type definition of the trampoline used to enter WebAssembly from the host.
///
/// This function type is what's generated for the entry trampolines that are
/// compiled into a WebAssembly module's image. Note that trampolines are not
/// always used by Wasmtime since the `TypedFunc` API allows bypassing the
/// trampoline and directly calling the underlying wasm function (at the time of
/// this writing).
///
/// The trampoline's arguments here are:
///
/// * `*mut VMOpaqueContext` - this a contextual pointer defined within the
///   context of the receiving function pointer. For now this is always `*mut
///   VMContext` but with the component model it may be the case that this is a
///   different type of pointer.
///
/// * `*mut VMContext` - this is the "caller" context, which at this time is
///   always unconditionally core wasm (even in the component model). This
///   contextual pointer cannot be `NULL` and provides information necessary to
///   resolve the caller's context for the `Caller` API in Wasmtime.
///
/// * `*const VMFunctionBody` - this is the indirect function pointer which is
///   the actual target function to invoke. This function uses the System-V ABI
///   for its argumenst and a semi-custom ABI for the return values (one return
///   value is returned directly, multiple return values have the first one
///   returned directly and remaining ones returned indirectly through a
///   stack pointer). This function pointer may be Cranelift-compiled code or it
///   may also be a host-compiled trampoline (e.g. when a host function calls a
///   host function through the `wasmtime::Func` wrapper). The definition of the
///   first argument of this function depends on what this receiving function
///   pointer desires.
///
/// * `*mut ValRaw` - this is storage space for both arguments and results of
///   the function. The trampoline will read the arguments from this array to
///   pass to the function pointer provided. The results are then written to the
///   array afterwards (both reads and writes start at index 0). It's the
///   caller's responsibility to make sure this array is appropriately sized.
pub type VMTrampoline =
    unsafe extern "C" fn(*mut VMOpaqueContext, *mut VMContext, *const VMFunctionBody, *mut ValRaw);

/// An "opaque" version of `VMContext` which must be explicitly casted to a
/// target context.
///
/// This context is used to represent that contexts specified in
/// `VMCallerCheckedAnyfunc` can have any type and don't have an implicit
/// structure. Neither wasmtime nor cranelift-generated code can rely on the
/// structure of an opaque context in general and only the code which configured
/// the context is able to rely on a particular structure. This is because the
/// context pointer configured for `VMCallerCheckedAnyfunc` is guaranteed to be
/// the first parameter passed.
///
/// Note that Wasmtime currently has a layout where all contexts that are casted
/// to an opaque context start with a 32-bit "magic" which can be used in debug
/// mode to debug-assert that the casts here are correct and have at least a
/// little protection against incorrect casts.
pub struct VMOpaqueContext {
    pub(crate) magic: u32,
    _marker: marker::PhantomPinned,
}

impl VMOpaqueContext {
    /// Helper function to clearly indicate that casts are desired.
    #[inline]
    pub fn from_vmcontext(ptr: *mut VMContext) -> *mut VMOpaqueContext {
        ptr.cast()
    }

    /// Helper function to clearly indicate that casts are desired.
    #[inline]
    pub fn from_vm_host_func_context(ptr: *mut VMHostFuncContext) -> *mut VMOpaqueContext {
        ptr.cast()
    }
}
