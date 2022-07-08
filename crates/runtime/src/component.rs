//! Runtime support for the component model in Wasmtime
//!
//! Currently this runtime support includes a `VMComponentContext` which is
//! similar in purpose to `VMContext`. The context is read from
//! cranelift-generated trampolines when entering the host from a wasm module.
//! Eventually it's intended that module-to-module calls, which would be
//! cranelift-compiled adapters, will use this `VMComponentContext` as well.

use crate::{
    Store, VMCallerCheckedAnyfunc, VMFunctionBody, VMMemoryDefinition, VMOpaqueContext,
    VMSharedSignatureIndex, ValRaw,
};
use memoffset::offset_of;
use std::alloc::{self, Layout};
use std::marker;
use std::mem;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use wasmtime_environ::component::{
    Component, LoweredIndex, RuntimeAlwaysTrapIndex, RuntimeComponentInstanceIndex,
    RuntimeMemoryIndex, RuntimePostReturnIndex, RuntimeReallocIndex, StringEncoding,
    VMComponentOffsets, VMCOMPONENT_FLAG_MAY_ENTER, VMCOMPONENT_FLAG_MAY_LEAVE,
    VMCOMPONENT_FLAG_NEEDS_POST_RETURN, VMCOMPONENT_MAGIC,
};
use wasmtime_environ::HostPtr;

const INVALID_PTR: usize = 0xdead_dead_beef_beef_u64 as usize;

/// Runtime representation of a component instance and all state necessary for
/// the instance itself.
///
/// This type never exists by-value, but rather it's always behind a pointer.
/// The size of the allocation for `ComponentInstance` includes the trailing
/// `VMComponentContext` which is variably sized based on the `offsets`
/// contained within.
#[repr(C)]
pub struct ComponentInstance {
    /// Size and offset information for the trailing `VMComponentContext`.
    offsets: VMComponentOffsets<HostPtr>,

    /// A zero-sized field which represents the end of the struct for the actual
    /// `VMComponentContext` to be allocated behind.
    vmctx: VMComponentContext,
}

/// Type signature for host-defined trampolines that are called from
/// WebAssembly.
///
/// This function signature is invoked from a cranelift-compiled trampoline that
/// adapts from the core wasm System-V ABI into the ABI provided here:
///
/// * `vmctx` - this is the first argument to the wasm import, and should always
///   end up being a `VMComponentContext`.
/// * `data` - this is the data pointer associated with the `VMLowering` for
///   which this function pointer was registered.
/// * `flags` - the component flags for may_enter/leave corresponding to the
///   component instance that the lowering happened within.
/// * `opt_memory` - this nullable pointer represents the memory configuration
///   option for the canonical ABI options.
/// * `opt_realloc` - this nullable pointer represents the realloc configuration
///   option for the canonical ABI options.
/// * `string_encoding` - this is the configured string encoding for the
///   canonical ABI this lowering corresponds to.
/// * `args_and_results` - pointer to stack-allocated space in the caller where
///   all the arguments are stored as well as where the results will be written
///   to. The size and initialized bytes of this depends on the core wasm type
///   signature that this callee corresponds to.
/// * `nargs_and_results` - the size, in units of `ValRaw`, of
///   `args_and_results`.
//
// FIXME: 8 arguments is probably too many. The `data` through `string-encoding`
// parameters should probably get packaged up into the `VMComponentContext`.
// Needs benchmarking one way or another though to figure out what the best
// balance is here.
pub type VMLoweringCallee = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    data: *mut u8,
    flags: *mut VMComponentFlags,
    opt_memory: *mut VMMemoryDefinition,
    opt_realloc: *mut VMCallerCheckedAnyfunc,
    string_encoding: StringEncoding,
    args_and_results: *mut ValRaw,
    nargs_and_results: usize,
);

/// Structure describing a lowered host function stored within a
/// `VMComponentContext` per-lowering.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VMLowering {
    /// The host function pointer that is invoked when this lowering is
    /// invoked.
    pub callee: VMLoweringCallee,
    /// The host data pointer (think void* pointer) to get passed to `callee`.
    pub data: *mut u8,
}

/// This is a marker type to represent the underlying allocation of a
/// `VMComponentContext`.
///
/// This type is similar to `VMContext` for core wasm and is allocated once per
/// component instance in Wasmtime. While the static size of this type is 0 the
/// actual runtime size is variable depending on the shape of the component that
/// this corresponds to. This structure always trails a `ComponentInstance`
/// allocation and the allocation/liftetime of this allocation is managed by
/// `ComponentInstance`.
#[repr(C)]
// Set an appropriate alignment for this structure where the most-aligned value
// internally right now is a pointer.
#[cfg_attr(target_pointer_width = "32", repr(align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(align(8)))]
pub struct VMComponentContext {
    /// For more information about this see the equivalent field in `VMContext`
    _marker: marker::PhantomPinned,
}

/// Flags stored in a `VMComponentContext` with values defined by
/// `VMCOMPONENT_FLAG_*`
#[repr(transparent)]
pub struct VMComponentFlags(u8);

impl ComponentInstance {
    /// Returns the layout corresponding to what would be an allocation of a
    /// `ComponentInstance` for the `offsets` provided.
    ///
    /// The returned layout has space for both the `ComponentInstance` and the
    /// trailing `VMComponentContext`.
    fn alloc_layout(offsets: &VMComponentOffsets<HostPtr>) -> Layout {
        let size = mem::size_of::<Self>()
            .checked_add(usize::try_from(offsets.size_of_vmctx()).unwrap())
            .unwrap();
        let align = mem::align_of::<Self>();
        Layout::from_size_align(size, align).unwrap()
    }

    /// Initializes an uninitialized pointer to a `ComponentInstance` in
    /// addition to its trailing `VMComponentContext`.
    ///
    /// The `ptr` provided must be valid for `alloc_size` bytes and will be
    /// entirely overwritten by this function call. The `offsets` correspond to
    /// the shape of the component being instantiated and `store` is a pointer
    /// back to the Wasmtime store for host functions to have access to.
    unsafe fn new_at(
        ptr: *mut ComponentInstance,
        alloc_size: usize,
        offsets: VMComponentOffsets<HostPtr>,
        store: *mut dyn Store,
    ) {
        assert!(alloc_size >= Self::alloc_layout(&offsets).size());

        ptr::write(
            ptr,
            ComponentInstance {
                offsets,
                vmctx: VMComponentContext {
                    _marker: marker::PhantomPinned,
                },
            },
        );

        (*ptr).initialize_vmctx(store);
    }

    fn vmctx(&self) -> *mut VMComponentContext {
        &self.vmctx as *const VMComponentContext as *mut VMComponentContext
    }

    unsafe fn vmctx_plus_offset<T>(&self, offset: u32) -> *mut T {
        self.vmctx()
            .cast::<u8>()
            .add(usize::try_from(offset).unwrap())
            .cast()
    }

    /// Returns a pointer to the "may leave" flag for this instance specified
    /// for canonical lowering and lifting operations.
    pub fn flags(&self, instance: RuntimeComponentInstanceIndex) -> *mut VMComponentFlags {
        unsafe { self.vmctx_plus_offset(self.offsets.flags(instance)) }
    }

    /// Returns the store that this component was created with.
    pub fn store(&self) -> *mut dyn Store {
        unsafe {
            let ret = *self.vmctx_plus_offset::<*mut dyn Store>(self.offsets.store());
            assert!(!ret.is_null());
            ret
        }
    }

    /// Returns the runtime memory definition corresponding to the index of the
    /// memory provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_memory(&self, idx: RuntimeMemoryIndex) -> *mut VMMemoryDefinition {
        unsafe {
            let ret = *self.vmctx_plus_offset(self.offsets.runtime_memory(idx));
            debug_assert!(ret as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the realloc pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_realloc(&self, idx: RuntimeReallocIndex) -> NonNull<VMCallerCheckedAnyfunc> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<NonNull<_>>(self.offsets.runtime_realloc(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the post-return pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_post_return(
        &self,
        idx: RuntimePostReturnIndex,
    ) -> NonNull<VMCallerCheckedAnyfunc> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<NonNull<_>>(self.offsets.runtime_post_return(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the host information for the lowered function at the index
    /// specified.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn lowering(&self, idx: LoweredIndex) -> VMLowering {
        unsafe {
            let ret = *self.vmctx_plus_offset::<VMLowering>(self.offsets.lowering(idx));
            debug_assert!(ret.callee as usize != INVALID_PTR);
            debug_assert!(ret.data as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the core wasm function pointer corresponding to the lowering
    /// index specified.
    ///
    /// The returned function is suitable to pass directly to a wasm module
    /// instantiation and the function is a cranelift-compiled trampoline.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn lowering_anyfunc(&self, idx: LoweredIndex) -> NonNull<VMCallerCheckedAnyfunc> {
        unsafe {
            let ret = self
                .vmctx_plus_offset::<VMCallerCheckedAnyfunc>(self.offsets.lowering_anyfunc(idx));
            debug_assert!((*ret).func_ptr.as_ptr() as usize != INVALID_PTR);
            debug_assert!((*ret).vmctx as usize != INVALID_PTR);
            NonNull::new(ret).unwrap()
        }
    }

    /// Same as `lowering_anyfunc` except for the functions that always trap.
    pub fn always_trap_anyfunc(
        &self,
        idx: RuntimeAlwaysTrapIndex,
    ) -> NonNull<VMCallerCheckedAnyfunc> {
        unsafe {
            let ret = self
                .vmctx_plus_offset::<VMCallerCheckedAnyfunc>(self.offsets.always_trap_anyfunc(idx));
            debug_assert!((*ret).func_ptr.as_ptr() as usize != INVALID_PTR);
            debug_assert!((*ret).vmctx as usize != INVALID_PTR);
            NonNull::new(ret).unwrap()
        }
    }

    /// Stores the runtime memory pointer at the index specified.
    ///
    /// This is intended to be called during the instantiation process of a
    /// component once a memory is available, which may not be until part-way
    /// through component instantiation.
    ///
    /// Note that it should be a property of the component model that the `ptr`
    /// here is never needed prior to it being configured here in the instance.
    pub fn set_runtime_memory(&mut self, idx: RuntimeMemoryIndex, ptr: *mut VMMemoryDefinition) {
        unsafe {
            debug_assert!(!ptr.is_null());
            let storage = self.vmctx_plus_offset(self.offsets.runtime_memory(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr;
        }
    }

    /// Same as `set_runtime_memory` but for realloc function pointers.
    pub fn set_runtime_realloc(
        &mut self,
        idx: RuntimeReallocIndex,
        ptr: NonNull<VMCallerCheckedAnyfunc>,
    ) {
        unsafe {
            let storage = self.vmctx_plus_offset(self.offsets.runtime_realloc(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr.as_ptr();
        }
    }

    /// Same as `set_runtime_memory` but for post-return function pointers.
    pub fn set_runtime_post_return(
        &mut self,
        idx: RuntimePostReturnIndex,
        ptr: NonNull<VMCallerCheckedAnyfunc>,
    ) {
        unsafe {
            let storage = self.vmctx_plus_offset(self.offsets.runtime_post_return(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr.as_ptr();
        }
    }

    /// Configures a lowered host function with all the pieces necessary.
    ///
    /// * `idx` - the index that's being configured
    /// * `lowering` - the host-related closure information to get invoked when
    ///   the lowering is called.
    /// * `anyfunc_func_ptr` - the cranelift-compiled trampoline which will
    ///   read the `VMComponentContext` and invoke `lowering` provided. This
    ///   function pointer will be passed to wasm if wasm needs to instantiate
    ///   something.
    /// * `anyfunc_type_index` - the signature index for the core wasm type
    ///   registered within the engine already.
    pub fn set_lowering(
        &mut self,
        idx: LoweredIndex,
        lowering: VMLowering,
        anyfunc_func_ptr: NonNull<VMFunctionBody>,
        anyfunc_type_index: VMSharedSignatureIndex,
    ) {
        unsafe {
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.lowering_callee(idx)) == INVALID_PTR
            );
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.lowering_data(idx)) == INVALID_PTR
            );
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.lowering_anyfunc(idx)) == INVALID_PTR
            );
            *self.vmctx_plus_offset(self.offsets.lowering(idx)) = lowering;
            let vmctx = self.vmctx();
            *self.vmctx_plus_offset(self.offsets.lowering_anyfunc(idx)) = VMCallerCheckedAnyfunc {
                func_ptr: anyfunc_func_ptr,
                type_index: anyfunc_type_index,
                vmctx: VMOpaqueContext::from_vmcomponent(vmctx),
            };
        }
    }

    /// Same as `set_lowering` but for the "always trap" functions.
    pub fn set_always_trap(
        &mut self,
        idx: RuntimeAlwaysTrapIndex,
        func_ptr: NonNull<VMFunctionBody>,
        type_index: VMSharedSignatureIndex,
    ) {
        unsafe {
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.always_trap_anyfunc(idx))
                    == INVALID_PTR
            );
            let vmctx = self.vmctx();
            *self.vmctx_plus_offset(self.offsets.always_trap_anyfunc(idx)) =
                VMCallerCheckedAnyfunc {
                    func_ptr,
                    type_index,
                    vmctx: VMOpaqueContext::from_vmcomponent(vmctx),
                };
        }
    }

    unsafe fn initialize_vmctx(&mut self, store: *mut dyn Store) {
        *self.vmctx_plus_offset(self.offsets.magic()) = VMCOMPONENT_MAGIC;
        *self.vmctx_plus_offset(self.offsets.store()) = store;
        for i in 0..self.offsets.num_runtime_component_instances {
            let i = RuntimeComponentInstanceIndex::from_u32(i);
            *self.flags(i) = VMComponentFlags::new();
        }

        // In debug mode set non-null bad values to all "pointer looking" bits
        // and pices related to lowering and such. This'll help detect any
        // erroneous usage and enable debug assertions above as well to prevent
        // loading these before they're configured or setting them twice.
        if cfg!(debug_assertions) {
            for i in 0..self.offsets.num_lowerings {
                let i = LoweredIndex::from_u32(i);
                let offset = self.offsets.lowering_callee(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
                let offset = self.offsets.lowering_data(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
                let offset = self.offsets.lowering_anyfunc(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_always_trap {
                let i = RuntimeAlwaysTrapIndex::from_u32(i);
                let offset = self.offsets.always_trap_anyfunc(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_memories {
                let i = RuntimeMemoryIndex::from_u32(i);
                let offset = self.offsets.runtime_memory(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_reallocs {
                let i = RuntimeReallocIndex::from_u32(i);
                let offset = self.offsets.runtime_realloc(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_post_returns {
                let i = RuntimePostReturnIndex::from_u32(i);
                let offset = self.offsets.runtime_post_return(i);
                *self.vmctx_plus_offset(offset) = INVALID_PTR;
            }
        }
    }
}

impl VMComponentContext {
    /// Moves the `self` pointer backwards to the `ComponentInstance` pointer
    /// that this `VMComponentContext` trails.
    pub fn instance(&self) -> *mut ComponentInstance {
        unsafe {
            (self as *const Self as *mut u8)
                .offset(-(offset_of!(ComponentInstance, vmctx) as isize))
                as *mut ComponentInstance
        }
    }
}

/// An owned version of `ComponentInstance` which is akin to
/// `Box<ComponentInstance>`.
///
/// This type can be dereferenced to `ComponentInstance` to access the
/// underlying methods.
pub struct OwnedComponentInstance {
    ptr: ptr::NonNull<ComponentInstance>,
}

// Using `NonNull` turns off auto-derivation of these traits but the owned usage
// here enables these trait impls so long as `ComponentInstance` itself
// implements these traits.
unsafe impl Send for OwnedComponentInstance where ComponentInstance: Send {}
unsafe impl Sync for OwnedComponentInstance where ComponentInstance: Sync {}

impl OwnedComponentInstance {
    /// Allocates a new `ComponentInstance + VMComponentContext` pair on the
    /// heap with `malloc` and configures it for the `component` specified.
    pub fn new(component: &Component, store: *mut dyn Store) -> OwnedComponentInstance {
        let offsets = VMComponentOffsets::new(HostPtr, component);
        let layout = ComponentInstance::alloc_layout(&offsets);
        unsafe {
            // Technically it is not required to `alloc_zeroed` here. The
            // primary reason for doing this is because a component context
            // start is a "partly initialized" state where pointers and such are
            // configured as the instantiation process continues. The component
            // model should guarantee that we never access uninitialized memory
            // in the context, but to help protect against possible bugs a
            // zeroed allocation is done here to try to contain
            // use-before-initialized issues.
            let ptr = alloc::alloc_zeroed(layout) as *mut ComponentInstance;
            let ptr = ptr::NonNull::new(ptr).unwrap();

            ComponentInstance::new_at(ptr.as_ptr(), layout.size(), offsets, store);

            OwnedComponentInstance { ptr }
        }
    }

    // Note that this is technically unsafe due to the fact that it enables
    // `mem::swap`-ing two component instances which would get all the offsets
    // mixed up and cause issues. This is scoped to just this module though as a
    // convenience to forward to `&mut` methods on `ComponentInstance`.
    unsafe fn instance_mut(&mut self) -> &mut ComponentInstance {
        &mut *self.ptr.as_ptr()
    }

    /// See `ComponentInstance::set_runtime_memory`
    pub fn set_runtime_memory(&mut self, idx: RuntimeMemoryIndex, ptr: *mut VMMemoryDefinition) {
        unsafe { self.instance_mut().set_runtime_memory(idx, ptr) }
    }

    /// See `ComponentInstance::set_runtime_realloc`
    pub fn set_runtime_realloc(
        &mut self,
        idx: RuntimeReallocIndex,
        ptr: NonNull<VMCallerCheckedAnyfunc>,
    ) {
        unsafe { self.instance_mut().set_runtime_realloc(idx, ptr) }
    }

    /// See `ComponentInstance::set_runtime_post_return`
    pub fn set_runtime_post_return(
        &mut self,
        idx: RuntimePostReturnIndex,
        ptr: NonNull<VMCallerCheckedAnyfunc>,
    ) {
        unsafe { self.instance_mut().set_runtime_post_return(idx, ptr) }
    }

    /// See `ComponentInstance::set_lowering`
    pub fn set_lowering(
        &mut self,
        idx: LoweredIndex,
        lowering: VMLowering,
        anyfunc_func_ptr: NonNull<VMFunctionBody>,
        anyfunc_type_index: VMSharedSignatureIndex,
    ) {
        unsafe {
            self.instance_mut()
                .set_lowering(idx, lowering, anyfunc_func_ptr, anyfunc_type_index)
        }
    }

    /// See `ComponentInstance::set_always_trap`
    pub fn set_always_trap(
        &mut self,
        idx: RuntimeAlwaysTrapIndex,
        func_ptr: NonNull<VMFunctionBody>,
        type_index: VMSharedSignatureIndex,
    ) {
        unsafe {
            self.instance_mut()
                .set_always_trap(idx, func_ptr, type_index)
        }
    }
}

impl Deref for OwnedComponentInstance {
    type Target = ComponentInstance;
    fn deref(&self) -> &ComponentInstance {
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl Drop for OwnedComponentInstance {
    fn drop(&mut self) {
        let layout = ComponentInstance::alloc_layout(&self.offsets);
        unsafe {
            ptr::drop_in_place(self.ptr.as_ptr());
            alloc::dealloc(self.ptr.as_ptr().cast(), layout);
        }
    }
}

impl VMComponentContext {
    /// Helper function to cast between context types using a debug assertion to
    /// protect against some mistakes.
    #[inline]
    pub unsafe fn from_opaque(opaque: *mut VMOpaqueContext) -> *mut VMComponentContext {
        // See comments in `VMContext::from_opaque` for this debug assert
        debug_assert_eq!((*opaque).magic, VMCOMPONENT_MAGIC);
        opaque.cast()
    }
}

impl VMOpaqueContext {
    /// Helper function to clearly indicate the cast desired
    #[inline]
    pub fn from_vmcomponent(ptr: *mut VMComponentContext) -> *mut VMOpaqueContext {
        ptr.cast()
    }
}

#[allow(missing_docs)]
impl VMComponentFlags {
    fn new() -> VMComponentFlags {
        VMComponentFlags(VMCOMPONENT_FLAG_MAY_LEAVE | VMCOMPONENT_FLAG_MAY_ENTER)
    }

    #[inline]
    pub fn may_leave(&self) -> bool {
        self.0 & VMCOMPONENT_FLAG_MAY_LEAVE != 0
    }

    #[inline]
    pub fn set_may_leave(&mut self, val: bool) {
        if val {
            self.0 |= VMCOMPONENT_FLAG_MAY_LEAVE;
        } else {
            self.0 &= !VMCOMPONENT_FLAG_MAY_LEAVE;
        }
    }

    #[inline]
    pub fn may_enter(&self) -> bool {
        self.0 & VMCOMPONENT_FLAG_MAY_ENTER != 0
    }

    #[inline]
    pub fn set_may_enter(&mut self, val: bool) {
        if val {
            self.0 |= VMCOMPONENT_FLAG_MAY_ENTER;
        } else {
            self.0 &= !VMCOMPONENT_FLAG_MAY_ENTER;
        }
    }

    #[inline]
    pub fn needs_post_return(&self) -> bool {
        self.0 & VMCOMPONENT_FLAG_NEEDS_POST_RETURN != 0
    }

    #[inline]
    pub fn set_needs_post_return(&mut self, val: bool) {
        if val {
            self.0 |= VMCOMPONENT_FLAG_NEEDS_POST_RETURN;
        } else {
            self.0 &= !VMCOMPONENT_FLAG_NEEDS_POST_RETURN;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn size_of_vmcomponent_flags() {
        let component = Component::default();
        let offsets = VMComponentOffsets::new(size_of::<*mut u8>() as u8, &component);
        assert_eq!(
            size_of::<VMComponentFlags>(),
            usize::from(offsets.size_of_vmcomponent_flags())
        );
    }
}
