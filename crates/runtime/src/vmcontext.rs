//! This file declares `VMContext` and several related structs which contain
//! fields that compiled wasm code accesses directly.

mod vm_host_func_context;

use crate::{GcStore, VMGcRef};
use sptr::Strict;
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::marker;
use std::mem;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::u32;
pub use vm_host_func_context::{VMArrayCallHostFuncContext, VMNativeCallHostFuncContext};
use wasmtime_environ::VMSharedTypeIndex;
use wasmtime_environ::WasmHeapType;
use wasmtime_environ::WasmValType;
use wasmtime_environ::{BuiltinFunctionIndex, DefinedMemoryIndex, Unsigned, VMCONTEXT_MAGIC};

/// A function pointer that exposes the array calling convention.
///
/// Regardless of the underlying Wasm function type, all functions using the
/// array calling convention have the same Rust signature.
///
/// Arguments:
///
/// * Callee `vmctx` for the function itself.
///
/// * Caller's `vmctx` (so that host functions can access the linear memory of
///   their Wasm callers).
///
/// * A pointer to a buffer of `ValRaw`s where both arguments are passed into
///   this function, and where results are returned from this function.
///
/// * The capacity of the `ValRaw` buffer. Must always be at least
///   `max(len(wasm_params), len(wasm_results))`.
pub type VMArrayCallFunction =
    unsafe extern "C" fn(*mut VMOpaqueContext, *mut VMOpaqueContext, *mut ValRaw, usize);

/// A function pointer that exposes the native calling convention.
///
/// Different Wasm function types end up mapping to different Rust function
/// types, so this isn't simply a type alias the way that `VMArrayCallFunction`
/// is.
///
/// This is the default calling convention for the target (e.g. System-V or
/// fast-call) except multiple return values are handled by returning the first
/// return value in a register and everything else through a return-pointer.
#[repr(transparent)]
pub struct VMNativeCallFunction(VMFunctionBody);

/// A function pointer that exposes the Wasm calling convention.
///
/// In practice, different Wasm function types end up mapping to different Rust
/// function types, so this isn't simply a type alias the way that
/// `VMArrayCallFunction` is. However, the exact details of the calling
/// convention are left to the Wasm compiler (e.g. Cranelift or Winch). Runtime
/// code never does anything with these function pointers except shuffle them
/// around and pass them back to Wasm.
#[repr(transparent)]
pub struct VMWasmCallFunction(VMFunctionBody);

/// An imported function.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct VMFunctionImport {
    /// Function pointer to use when calling this imported function from Wasm.
    pub wasm_call: NonNull<VMWasmCallFunction>,

    /// Function pointer to use when calling this imported function from native code.
    pub native_call: NonNull<VMNativeCallFunction>,

    /// Function pointer to use when calling this imported function with the
    /// "array" calling convention that `Func::new` et al use.
    pub array_call: VMArrayCallFunction,

    /// The VM state associated with this function.
    ///
    /// For Wasm functions defined by core wasm instances this will be `*mut
    /// VMContext`, but for lifted/lowered component model functions this will
    /// be a `VMComponentContext`, and for a host function it will be a
    /// `VMHostFuncContext`, etc.
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
            offset_of!(VMFunctionImport, wasm_call),
            usize::from(offsets.vmfunction_import_wasm_call())
        );
        assert_eq!(
            offset_of!(VMFunctionImport, native_call),
            usize::from(offsets.vmfunction_import_native_call())
        );
        assert_eq!(
            offset_of!(VMFunctionImport, array_call),
            usize::from(offsets.vmfunction_import_array_call())
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
    #[cfg(feature = "gc")]
    fn check_vmglobal_can_contain_gc_ref() {
        assert!(size_of::<crate::VMGcRef>() <= size_of::<VMGlobalDefinition>());
    }
}

impl VMGlobalDefinition {
    /// Construct a `VMGlobalDefinition`.
    pub fn new() -> Self {
        Self { storage: [0; 16] }
    }

    /// Create a `VMGlobalDefinition` from a `ValRaw`.
    ///
    /// # Unsafety
    ///
    /// This raw value's type must match the given `WasmValType`.
    pub unsafe fn from_val_raw(wasm_ty: WasmValType, raw: ValRaw) -> Self {
        let mut global = Self::new();
        match wasm_ty {
            WasmValType::I32 => *global.as_i32_mut() = raw.get_i32(),
            WasmValType::I64 => *global.as_i64_mut() = raw.get_i64(),
            WasmValType::F32 => *global.as_f32_bits_mut() = raw.get_f32(),
            WasmValType::F64 => *global.as_f64_bits_mut() = raw.get_f64(),
            WasmValType::V128 => *global.as_u128_mut() = raw.get_v128(),
            WasmValType::Ref(r) => match r.heap_type {
                WasmHeapType::Extern => {
                    global.init_gc_ref(VMGcRef::from_raw_u32(raw.get_externref()))
                }
                WasmHeapType::Any | WasmHeapType::I31 | WasmHeapType::None => {
                    global.init_gc_ref(VMGcRef::from_raw_u32(raw.get_anyref()))
                }
                WasmHeapType::Func | WasmHeapType::Concrete(_) | WasmHeapType::NoFunc => {
                    *global.as_func_ref_mut() = raw.get_funcref().cast()
                }
            },
        }
        global
    }

    /// Get this global's value as a `ValRaw`.
    ///
    /// # Unsafety
    ///
    /// This global's value's type must match the given `WasmValType`.
    pub unsafe fn to_val_raw(&self, gc_store: &mut GcStore, wasm_ty: WasmValType) -> ValRaw {
        match wasm_ty {
            WasmValType::I32 => ValRaw::i32(*self.as_i32()),
            WasmValType::I64 => ValRaw::i64(*self.as_i64()),
            WasmValType::F32 => ValRaw::f32(*self.as_f32_bits()),
            WasmValType::F64 => ValRaw::f64(*self.as_f64_bits()),
            WasmValType::V128 => ValRaw::v128(*self.as_u128()),
            WasmValType::Ref(r) => match r.heap_type {
                WasmHeapType::Extern => ValRaw::externref(
                    self.as_gc_ref()
                        .map_or(0, |r| gc_store.clone_gc_ref(r).as_raw_u32()),
                ),
                WasmHeapType::Any | WasmHeapType::I31 | WasmHeapType::None => ValRaw::anyref(
                    self.as_gc_ref()
                        .map_or(0, |r| gc_store.clone_gc_ref(r).as_raw_u32()),
                ),
                WasmHeapType::Func | WasmHeapType::Concrete(_) | WasmHeapType::NoFunc => {
                    ValRaw::funcref(self.as_func_ref().cast())
                }
            },
        }
    }

    /// Return a reference to the value as an i32.
    pub unsafe fn as_i32(&self) -> &i32 {
        &*(self.storage.as_ref().as_ptr().cast::<i32>())
    }

    /// Return a mutable reference to the value as an i32.
    pub unsafe fn as_i32_mut(&mut self) -> &mut i32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<i32>())
    }

    /// Return a reference to the value as a u32.
    pub unsafe fn as_u32(&self) -> &u32 {
        &*(self.storage.as_ref().as_ptr().cast::<u32>())
    }

    /// Return a mutable reference to the value as an u32.
    pub unsafe fn as_u32_mut(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u32>())
    }

    /// Return a reference to the value as an i64.
    pub unsafe fn as_i64(&self) -> &i64 {
        &*(self.storage.as_ref().as_ptr().cast::<i64>())
    }

    /// Return a mutable reference to the value as an i64.
    pub unsafe fn as_i64_mut(&mut self) -> &mut i64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<i64>())
    }

    /// Return a reference to the value as an u64.
    pub unsafe fn as_u64(&self) -> &u64 {
        &*(self.storage.as_ref().as_ptr().cast::<u64>())
    }

    /// Return a mutable reference to the value as an u64.
    pub unsafe fn as_u64_mut(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u64>())
    }

    /// Return a reference to the value as an f32.
    pub unsafe fn as_f32(&self) -> &f32 {
        &*(self.storage.as_ref().as_ptr().cast::<f32>())
    }

    /// Return a mutable reference to the value as an f32.
    pub unsafe fn as_f32_mut(&mut self) -> &mut f32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<f32>())
    }

    /// Return a reference to the value as f32 bits.
    pub unsafe fn as_f32_bits(&self) -> &u32 {
        &*(self.storage.as_ref().as_ptr().cast::<u32>())
    }

    /// Return a mutable reference to the value as f32 bits.
    pub unsafe fn as_f32_bits_mut(&mut self) -> &mut u32 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u32>())
    }

    /// Return a reference to the value as an f64.
    pub unsafe fn as_f64(&self) -> &f64 {
        &*(self.storage.as_ref().as_ptr().cast::<f64>())
    }

    /// Return a mutable reference to the value as an f64.
    pub unsafe fn as_f64_mut(&mut self) -> &mut f64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<f64>())
    }

    /// Return a reference to the value as f64 bits.
    pub unsafe fn as_f64_bits(&self) -> &u64 {
        &*(self.storage.as_ref().as_ptr().cast::<u64>())
    }

    /// Return a mutable reference to the value as f64 bits.
    pub unsafe fn as_f64_bits_mut(&mut self) -> &mut u64 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u64>())
    }

    /// Return a reference to the value as an u128.
    pub unsafe fn as_u128(&self) -> &u128 {
        &*(self.storage.as_ref().as_ptr().cast::<u128>())
    }

    /// Return a mutable reference to the value as an u128.
    pub unsafe fn as_u128_mut(&mut self) -> &mut u128 {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<u128>())
    }

    /// Return a reference to the value as u128 bits.
    pub unsafe fn as_u128_bits(&self) -> &[u8; 16] {
        &*(self.storage.as_ref().as_ptr().cast::<[u8; 16]>())
    }

    /// Return a mutable reference to the value as u128 bits.
    pub unsafe fn as_u128_bits_mut(&mut self) -> &mut [u8; 16] {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<[u8; 16]>())
    }

    /// Return a reference to the global value as a borrowed GC reference.
    pub unsafe fn as_gc_ref(&self) -> Option<&VMGcRef> {
        let raw_ptr = self.storage.as_ref().as_ptr().cast::<Option<VMGcRef>>();
        let ret = (*raw_ptr).as_ref();
        assert!(cfg!(feature = "gc") || ret.is_none());
        ret
    }

    /// Initialize a global to the given GC reference.
    pub unsafe fn init_gc_ref(&mut self, gc_ref: Option<VMGcRef>) {
        assert!(cfg!(feature = "gc") || gc_ref.is_none());
        let raw_ptr = self.storage.as_mut().as_mut_ptr().cast::<Option<VMGcRef>>();
        ptr::write(raw_ptr, gc_ref);
    }

    /// Write a GC reference into this global value.
    pub unsafe fn write_gc_ref(&mut self, gc_store: &mut GcStore, gc_ref: Option<&VMGcRef>) {
        assert!(cfg!(feature = "gc") || gc_ref.is_none());

        let dest = &mut *(self.storage.as_mut().as_mut_ptr().cast::<Option<VMGcRef>>());
        assert!(cfg!(feature = "gc") || dest.is_none());

        gc_store.write_gc_ref(dest, gc_ref)
    }

    /// Return a reference to the value as a `VMFuncRef`.
    pub unsafe fn as_func_ref(&self) -> *mut VMFuncRef {
        *(self.storage.as_ref().as_ptr().cast::<*mut VMFuncRef>())
    }

    /// Return a mutable reference to the value as a `VMFuncRef`.
    pub unsafe fn as_func_ref_mut(&mut self) -> &mut *mut VMFuncRef {
        &mut *(self.storage.as_mut().as_mut_ptr().cast::<*mut VMFuncRef>())
    }
}

#[cfg(test)]
mod test_vmshared_type_index {
    use super::VMSharedTypeIndex;
    use std::mem::size_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn check_vmshared_type_index() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMSharedTypeIndex>(),
            usize::from(offsets.size_of_vmshared_type_index())
        );
    }
}

/// The VM caller-checked "funcref" record, for caller-side signature checking.
///
/// It consists of function pointer(s), a type id to be checked by the
/// caller, and the vmctx closure associated with this function.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VMFuncRef {
    /// Function pointer for this funcref if being called via the native calling
    /// convention.
    pub native_call: NonNull<VMNativeCallFunction>,

    /// Function pointer for this funcref if being called via the "array"
    /// calling convention that `Func::new` et al use.
    pub array_call: VMArrayCallFunction,

    /// Function pointer for this funcref if being called via the calling
    /// convention we use when compiling Wasm.
    ///
    /// Most functions come with a function pointer that we can use when they
    /// are called from Wasm. The notable exception is when we `Func::wrap` a
    /// host function, and we don't have a Wasm compiler on hand to compile a
    /// Wasm-to-native trampoline for the function. In this case, we leave
    /// `wasm_call` empty until the function is passed as an import to Wasm (or
    /// otherwise exposed to Wasm via tables/globals). At this point, we look up
    /// a Wasm-to-native trampoline for the function in the the Wasm's compiled
    /// module and use that fill in `VMFunctionImport::wasm_call`. **However**
    /// there is no guarantee that the Wasm module has a trampoline for this
    /// function's signature. The Wasm module only has trampolines for its
    /// types, and if this function isn't of one of those types, then the Wasm
    /// module will not have a trampoline for it. This is actually okay, because
    /// it means that the Wasm cannot actually call this function. But it does
    /// mean that this field needs to be an `Option` even though it is non-null
    /// the vast vast vast majority of the time.
    pub wasm_call: Option<NonNull<VMWasmCallFunction>>,

    /// Function signature's type id.
    pub type_index: VMSharedTypeIndex,

    /// The VM state associated with this function.
    ///
    /// The actual definition of what this pointer points to depends on the
    /// function being referenced: for core Wasm functions, this is a `*mut
    /// VMContext`, for host functions it is a `*mut VMHostFuncContext`, and for
    /// component functions it is a `*mut VMComponentContext`.
    pub vmctx: *mut VMOpaqueContext,
    // If more elements are added here, remember to add offset_of tests below!
}

unsafe impl Send for VMFuncRef {}
unsafe impl Sync for VMFuncRef {}

#[cfg(test)]
mod test_vm_func_ref {
    use super::VMFuncRef;
    use memoffset::offset_of;
    use std::mem::size_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn check_vm_func_ref_offsets() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            size_of::<VMFuncRef>(),
            usize::from(offsets.ptr.size_of_vm_func_ref())
        );
        assert_eq!(
            offset_of!(VMFuncRef, native_call),
            usize::from(offsets.ptr.vm_func_ref_native_call())
        );
        assert_eq!(
            offset_of!(VMFuncRef, array_call),
            usize::from(offsets.ptr.vm_func_ref_array_call())
        );
        assert_eq!(
            offset_of!(VMFuncRef, wasm_call),
            usize::from(offsets.ptr.vm_func_ref_wasm_call())
        );
        assert_eq!(
            offset_of!(VMFuncRef, type_index),
            usize::from(offsets.ptr.vm_func_ref_type_index())
        );
        assert_eq!(
            offset_of!(VMFuncRef, vmctx),
            usize::from(offsets.ptr.vm_func_ref_vmctx())
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
            #[allow(unused_doc_comments)]
            pub const INIT: VMBuiltinFunctionsArray = VMBuiltinFunctionsArray {
                $(
                    $name: crate::libcalls::raw::$name,
                )*
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

const _: () = {
    assert!(
        mem::size_of::<VMBuiltinFunctionsArray>()
            == mem::size_of::<usize>()
                * (BuiltinFunctionIndex::builtin_functions_total_number() as usize)
    )
};

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
    fn vmctx_runtime_limits_offset() {
        let module = Module::new();
        let offsets = VMOffsets::new(size_of::<*mut u8>() as u8, &module);
        assert_eq!(
            offsets.vmctx_runtime_limits(),
            offsets.ptr.vmcontext_runtime_limits().into()
        );
    }

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
    /// The payload here is a Rust `[u8; 16]` which has the same number of bits
    /// but note that `v128` in WebAssembly is often considered a vector type
    /// such as `i32x4` or `f64x2`. This means that the actual interpretation
    /// of the underlying bits is left up to the instructions which consume
    /// this value.
    ///
    /// This value is always stored in a little-endian format.
    v128: [u8; 16],

    /// A WebAssembly `funcref` value (or one of its subtypes).
    ///
    /// The payload here is a pointer which is runtime-defined. This is one of
    /// the main points of unsafety about the `ValRaw` type as the validity of
    /// the pointer here is not easily verified and must be preserved by
    /// carefully calling the correct functions throughout the runtime.
    ///
    /// This value is always stored in a little-endian format.
    funcref: *mut c_void,

    /// A WebAssembly `externref` value (or one of its subtypes).
    ///
    /// The payload here is a compressed pointer value which is
    /// runtime-defined. This is one of the main points of unsafety about the
    /// `ValRaw` type as the validity of the pointer here is not easily verified
    /// and must be preserved by carefully calling the correct functions
    /// throughout the runtime.
    ///
    /// This value is always stored in a little-endian format.
    externref: u32,

    /// A WebAssembly `anyref` value (or one of its subtypes).
    ///
    /// The payload here is a compressed pointer value which is
    /// runtime-defined. This is one of the main points of unsafety about the
    /// `ValRaw` type as the validity of the pointer here is not easily verified
    /// and must be preserved by carefully calling the correct functions
    /// throughout the runtime.
    ///
    /// This value is always stored in a little-endian format.
    anyref: u32,
}

// The `ValRaw` type is matched as `wasmtime_val_raw_t` in the C API so these
// are some simple assertions about the shape of the type which are additionally
// matched in C.
const _: () = {
    assert!(std::mem::size_of::<ValRaw>() == 16);
    assert!(std::mem::align_of::<ValRaw>() == 8);
};

// This type is just a bag-of-bits so it's up to the caller to figure out how
// to safely deal with threading concerns and safely access interior bits.
unsafe impl Send for ValRaw {}
unsafe impl Sync for ValRaw {}

impl std::fmt::Debug for ValRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct Hex<T>(T);
        impl<T: std::fmt::LowerHex> std::fmt::Debug for Hex<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let bytes = std::mem::size_of::<T>();
                let hex_digits_per_byte = 2;
                let hex_digits = bytes * hex_digits_per_byte;
                write!(f, "0x{:0width$x}", self.0, width = hex_digits)
            }
        }

        unsafe {
            f.debug_struct("ValRaw")
                .field("i32", &Hex(self.i32))
                .field("i64", &Hex(self.i64))
                .field("f32", &Hex(self.f32))
                .field("f64", &Hex(self.f64))
                .field("v128", &Hex(u128::from_le_bytes(self.v128)))
                .field("funcref", &self.funcref)
                .field("externref", &Hex(self.externref))
                .field("anyref", &Hex(self.anyref))
                .finish()
        }
    }
}

impl ValRaw {
    /// Create a null reference that is compatible with any of
    /// `{any,extern,func}ref`.
    pub fn null() -> ValRaw {
        unsafe {
            let raw = mem::MaybeUninit::<Self>::zeroed().assume_init();
            debug_assert_eq!(raw.get_anyref(), 0);
            debug_assert_eq!(raw.get_externref(), 0);
            debug_assert_eq!(raw.get_funcref(), ptr::null_mut());
            raw
        }
    }

    /// Creates a WebAssembly `i32` value
    #[inline]
    pub fn i32(i: i32) -> ValRaw {
        // Note that this is intentionally not setting the `i32` field, instead
        // setting the `i64` field with a zero-extended version of `i`. For more
        // information on this see the comments on `Lower for Result` in the
        // `wasmtime` crate. Otherwise though all `ValRaw` constructors are
        // otherwise constrained to guarantee that the initial 64-bits are
        // always initialized.
        ValRaw::u64(i.unsigned().into())
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
        ValRaw {
            v128: i.to_le_bytes(),
        }
    }

    /// Creates a WebAssembly `funcref` value
    #[inline]
    pub fn funcref(i: *mut c_void) -> ValRaw {
        ValRaw {
            funcref: Strict::map_addr(i, |i| i.to_le()),
        }
    }

    /// Creates a WebAssembly `externref` value
    #[inline]
    pub fn externref(e: u32) -> ValRaw {
        assert!(cfg!(feature = "gc") || e == 0);
        ValRaw {
            externref: e.to_le(),
        }
    }

    /// Creates a WebAssembly `anyref` value
    #[inline]
    pub fn anyref(r: u32) -> ValRaw {
        assert!(cfg!(feature = "gc") || r == 0);
        ValRaw { anyref: r.to_le() }
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
        self.get_i32().unsigned()
    }

    /// Gets the WebAssembly `i64` value
    #[inline]
    pub fn get_u64(&self) -> u64 {
        self.get_i64().unsigned()
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
        unsafe { u128::from_le_bytes(self.v128) }
    }

    /// Gets the WebAssembly `funcref` value
    #[inline]
    pub fn get_funcref(&self) -> *mut c_void {
        unsafe { Strict::map_addr(self.funcref, |i| usize::from_le(i)) }
    }

    /// Gets the WebAssembly `externref` value
    #[inline]
    pub fn get_externref(&self) -> u32 {
        let externref = u32::from_le(unsafe { self.externref });
        assert!(cfg!(feature = "gc") || externref == 0);
        externref
    }

    /// Gets the WebAssembly `anyref` value
    #[inline]
    pub fn get_anyref(&self) -> u32 {
        let anyref = u32::from_le(unsafe { self.anyref });
        assert!(cfg!(feature = "gc") || anyref == 0);
        anyref
    }
}

/// An "opaque" version of `VMContext` which must be explicitly casted to a
/// target context.
///
/// This context is used to represent that contexts specified in
/// `VMFuncRef` can have any type and don't have an implicit
/// structure. Neither wasmtime nor cranelift-generated code can rely on the
/// structure of an opaque context in general and only the code which configured
/// the context is able to rely on a particular structure. This is because the
/// context pointer configured for `VMFuncRef` is guaranteed to be
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
    pub fn from_vm_array_call_host_func_context(
        ptr: *mut VMArrayCallHostFuncContext,
    ) -> *mut VMOpaqueContext {
        ptr.cast()
    }

    /// Helper function to clearly indicate that casts are desired.
    #[inline]
    pub fn from_vm_native_call_host_func_context(
        ptr: *mut VMNativeCallHostFuncContext,
    ) -> *mut VMOpaqueContext {
        ptr.cast()
    }
}
