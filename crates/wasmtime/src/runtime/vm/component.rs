//! Runtime support for the component model in Wasmtime
//!
//! Currently this runtime support includes a `VMComponentContext` which is
//! similar in purpose to `VMContext`. The context is read from
//! cranelift-generated trampolines when entering the host from a wasm module.
//! Eventually it's intended that module-to-module calls, which would be
//! cranelift-compiled adapters, will use this `VMComponentContext` as well.

use crate::prelude::*;
use crate::runtime::vm::{
    SendSyncPtr, VMArrayCallFunction, VMFuncRef, VMGlobalDefinition, VMMemoryDefinition,
    VMOpaqueContext, VMStore, VMWasmCallFunction, ValRaw,
};
use alloc::alloc::Layout;
use alloc::sync::Arc;
use core::any::Any;
use core::marker;
use core::mem;
use core::mem::offset_of;
use core::ops::Deref;
use core::ptr::{self, NonNull};
use sptr::Strict;
use wasmtime_environ::component::*;
use wasmtime_environ::{HostPtr, PrimaryMap, VMSharedTypeIndex};

#[allow(clippy::cast_possible_truncation)] // it's intended this is truncated on
                                           // 32-bit platforms
const INVALID_PTR: usize = 0xdead_dead_beef_beef_u64 as usize;

mod libcalls;
mod resources;
mod states;

pub use self::resources::{CallContexts, ResourceTable, ResourceTables};
pub use self::states::StateTable;

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

    /// For more information about this see the documentation on
    /// `Instance::vmctx_self_reference`.
    vmctx_self_reference: SendSyncPtr<VMComponentContext>,

    /// Runtime type information about this component.
    runtime_info: Arc<dyn ComponentRuntimeInfo>,

    /// State of resources for all `TypeResourceTableIndex` values for this
    /// component.
    ///
    /// This is paired with other information to create a `ResourceTables` which
    /// is how this field is manipulated.
    component_resource_tables: PrimaryMap<TypeResourceTableIndex, ResourceTable>,

    component_waitable_tables: PrimaryMap<RuntimeComponentInstanceIndex, StateTable<WaitableState>>,
    component_error_context_tables: PrimaryMap<TypeErrorContextTableIndex, StateTable<usize>>,

    /// Storage for the type information about resources within this component
    /// instance.
    ///
    /// This is actually `Arc<PrimaryMap<ResourceIndex, ResourceType>>` but that
    /// can't be in this crate because `ResourceType` isn't here. Not using `dyn
    /// Any` is left as an exercise for a future refactoring.
    resource_types: Arc<dyn Any + Send + Sync>,

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
/// * `ty` - the type index, relative to the tables in `vmctx`, that is the
///   type of the function being called.
/// * `caller_instance` - The (sub)component instance of the caller.
/// * `flags` - the component flags for may_enter/leave corresponding to the
///   component instance that the lowering happened within.
/// * `opt_memory` - this nullable pointer represents the memory configuration
///   option for the canonical ABI options.
/// * `opt_realloc` - this nullable pointer represents the realloc configuration
///   option for the canonical ABI options.
/// * `string_encoding` - this is the configured string encoding for the
///   canonical ABI this lowering corresponds to.
/// * `async_` - whether the caller is using the async ABI.
/// * `args_and_results` - pointer to stack-allocated space in the caller where
///   all the arguments are stored as well as where the results will be written
///   to. The size and initialized bytes of this depends on the core wasm type
///   signature that this callee corresponds to.
/// * `nargs_and_results` - the size, in units of `ValRaw`, of
///   `args_and_results`.
///
/// This function returns a `bool` which indicates whether the call succeeded
/// or not. On failure this function records trap information in TLS which
/// should be suitable for reading later.
//
// FIXME: 11 arguments is probably too many. The `data` through `string-encoding`
// parameters should probably get packaged up into the `VMComponentContext`.
// Needs benchmarking one way or another though to figure out what the best
// balance is here.
pub type VMLoweringCallee = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    data: *mut u8,
    ty: u32,
    caller_instance: u32,
    flags: *mut u8,
    opt_memory: *mut VMMemoryDefinition,
    opt_realloc: *mut VMFuncRef,
    string_encoding: u8,
    async_: u8,
    args_and_results: *mut mem::MaybeUninit<ValRaw>,
    nargs_and_results: usize,
) -> bool;

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

/// Type signature for the host-defined `task.backpressure` built-in function.
pub type VMTaskBackpressureCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    caller_instance: RuntimeComponentInstanceIndex,
    arg: u32,
) -> bool;

/// Type signature for the host-defined `task.return` built-in function.
pub type VMTaskReturnCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    ty: TypeTaskReturnIndex,
    args_and_results: *mut mem::MaybeUninit<ValRaw>,
    nargs_and_results: usize,
) -> bool;

/// Type signature for the host-defined `task.wait` and `task.poll` built-in functions.
pub type VMTaskWaitOrPollCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    caller_instance: RuntimeComponentInstanceIndex,
    async_: bool,
    memory: *mut VMMemoryDefinition,
    payload: u32,
) -> u64;

/// Type signature for the host-defined `task.yield` built-in function.
pub type VMTaskYieldCallback = extern "C" fn(vmctx: *mut VMOpaqueContext, async_: bool) -> bool;

/// Type signature for the host-defined `subtask.drop` built-in function.
pub type VMSubtaskDropCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    instance: RuntimeComponentInstanceIndex,
    arg: u32,
) -> bool;

/// Type signature for the host-defined built-in function to represent starting
/// a call to an async-lowered import in a FACT-generated module.
pub type VMAsyncEnterCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    start: *mut VMFuncRef,
    return_: *mut VMFuncRef,
    caller_instance: RuntimeComponentInstanceIndex,
    task_return_type: TypeTaskReturnIndex,
    params: u32,
    results: u32,
) -> bool;

/// Type signature for the host-defined built-in function to represent
/// completing a call to an async-lowered import in a FACT-generated module.
pub type VMAsyncExitCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    callback: *mut VMFuncRef,
    caller_instance: RuntimeComponentInstanceIndex,
    callee: *mut VMFuncRef,
    callee_instance: RuntimeComponentInstanceIndex,
    param_count: u32,
    result_count: u32,
    flags: u32,
) -> u64;

/// Type signature for the host-defined `future.new` built-in function.
pub type VMFutureNewCallback =
    extern "C" fn(vmctx: *mut VMOpaqueContext, ty: TypeFutureTableIndex) -> u64;

/// Type signature for the host-defined `future.read` and `future.write`
/// built-in functions.
pub type VMFutureTransmitCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeFutureTableIndex,
    future: u32,
    address: u32,
) -> u64;

/// Type signature for the host-defined `future.cancel-read` and
/// `future.cancel-write` built-in functions.
pub type VMFutureCancelCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    async_: bool,
    handle: u32,
) -> u64;

/// Type signature for the host-defined `future.close-readable` built-in function.
pub type VMFutureCloseReadableCallback =
    extern "C" fn(vmctx: *mut VMOpaqueContext, ty: TypeFutureTableIndex, handle: u32) -> bool;

/// Type signature for the host-defined `future.close-writable` built-in function.
pub type VMFutureCloseWritableCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    handle: u32,
    error: u32,
) -> bool;

/// Type signature for the host-defined `stream.new` built-in function.
pub type VMStreamNewCallback =
    extern "C" fn(vmctx: *mut VMOpaqueContext, ty: TypeStreamTableIndex) -> u64;

/// Type signature for the host-defined `stream.read` and `stream.write`
/// built-in functions
pub type VMStreamTransmitCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeStreamTableIndex,
    stream: u32,
    address: u32,
    count: u32,
) -> u64;

/// Type signature for the host-defined `stream.read` ans `stream.write`
/// built-in functions for when the payload is trivially `memcpy`-able.
pub type VMFlatStreamTransmitCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    ty: TypeStreamTableIndex,
    payload_size: u32,
    payload_align: u32,
    stream: u32,
    address: u32,
    count: u32,
) -> u64;

/// Type signature for the host-defined `stream.close-readable` built-in function.
pub type VMStreamCloseReadableCallback =
    extern "C" fn(vmctx: *mut VMOpaqueContext, ty: TypeStreamTableIndex, handle: u32) -> bool;

/// Type signature for the host-defined `stream.close-writable` built-in function.
pub type VMStreamCloseWritableCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    handle: u32,
    error: u32,
) -> bool;

/// Type signature for the host-defined `stream.cancel-read` and
/// `stream.cancel-write` built-in functions.
pub type VMStreamCancelCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    async_: bool,
    handle: u32,
) -> u64;

/// Type signature for the host-defined `error-context.new` built-in function.
pub type VMErrorContextNewCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeErrorContextTableIndex,
    address: u32,
    count: u32,
) -> u64;

/// Type signature for the host-defined `error-context.debug-message` built-in
/// function.
pub type VMErrorContextDebugMessageCallback = extern "C" fn(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeErrorContextTableIndex,
    handle: u32,
    address: u32,
) -> bool;

/// Type signature for the host-defined `error-context.drop` built-in function.
pub type VMErrorContextDropCallback =
    extern "C" fn(vmctx: *mut VMOpaqueContext, ty: TypeErrorContextTableIndex, handle: u32) -> bool;

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
// internally right now `VMGlobalDefinition` which has an alignment of 16 bytes.
#[repr(align(16))]
pub struct VMComponentContext {
    /// For more information about this see the equivalent field in `VMContext`
    _marker: marker::PhantomPinned,
}

/// Represents the state of a stream or future handle.
#[derive(Debug, Eq, PartialEq)]
pub enum StreamFutureState {
    /// Both the read and write ends are owned by the same component instance.
    Local,
    /// Only the write end is owned by this component instance.
    Write,
    /// Only the read end is owned by this component instance.
    Read,
    /// A read or write is in progress.
    Busy,
}

/// Represents the state of a waitable handle.
#[derive(Debug)]
pub enum WaitableState {
    /// Represents a task handle.
    Task,
    /// Represents a stream handle.
    Stream(TypeStreamTableIndex, StreamFutureState),
    /// Represents a future handle.
    Future(TypeFutureTableIndex, StreamFutureState),
}

impl ComponentInstance {
    /// Converts the `vmctx` provided into a `ComponentInstance` and runs the
    /// provided closure with that instance.
    ///
    /// # Unsafety
    ///
    /// This is `unsafe` because `vmctx` cannot be guaranteed to be a valid
    /// pointer and it cannot be proven statically that it's safe to get a
    /// mutable reference at this time to the instance from `vmctx`.
    pub unsafe fn from_vmctx<R>(
        vmctx: *mut VMComponentContext,
        f: impl FnOnce(&mut ComponentInstance) -> R,
    ) -> R {
        let ptr = vmctx
            .byte_sub(mem::size_of::<ComponentInstance>())
            .cast::<ComponentInstance>();
        f(&mut *ptr)
    }

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
        ptr: NonNull<ComponentInstance>,
        alloc_size: usize,
        offsets: VMComponentOffsets<HostPtr>,
        runtime_info: Arc<dyn ComponentRuntimeInfo>,
        resource_types: Arc<dyn Any + Send + Sync>,
        store: *mut dyn VMStore,
    ) {
        assert!(alloc_size >= Self::alloc_layout(&offsets).size());

        let num_resource_tables = runtime_info.component().num_resource_tables;
        let mut component_resource_tables = PrimaryMap::with_capacity(num_resource_tables);
        for _ in 0..num_resource_tables {
            component_resource_tables.push(ResourceTable::default());
        }

        let num_waitable_tables = runtime_info.component().num_runtime_component_instances;
        let mut component_waitable_tables =
            PrimaryMap::with_capacity(usize::try_from(num_waitable_tables).unwrap());
        for _ in 0..num_waitable_tables {
            component_waitable_tables.push(StateTable::default());
        }

        let num_error_context_tables = runtime_info.component().num_error_context_tables;
        let mut component_error_context_tables =
            PrimaryMap::with_capacity(num_error_context_tables);
        for _ in 0..num_error_context_tables {
            component_error_context_tables.push(StateTable::default());
        }

        ptr::write(
            ptr.as_ptr(),
            ComponentInstance {
                offsets,
                vmctx_self_reference: SendSyncPtr::new(
                    NonNull::new(
                        ptr.as_ptr()
                            .byte_add(mem::size_of::<ComponentInstance>())
                            .cast(),
                    )
                    .unwrap(),
                ),
                component_resource_tables,
                component_waitable_tables,
                component_error_context_tables,
                runtime_info,
                resource_types,
                vmctx: VMComponentContext {
                    _marker: marker::PhantomPinned,
                },
            },
        );

        (*ptr.as_ptr()).initialize_vmctx(store);
    }

    fn vmctx(&self) -> *mut VMComponentContext {
        let addr = core::ptr::addr_of!(self.vmctx);
        Strict::with_addr(self.vmctx_self_reference.as_ptr(), Strict::addr(addr))
    }

    unsafe fn vmctx_plus_offset<T>(&self, offset: u32) -> *const T {
        self.vmctx()
            .byte_add(usize::try_from(offset).unwrap())
            .cast()
    }

    unsafe fn vmctx_plus_offset_mut<T>(&mut self, offset: u32) -> *mut T {
        self.vmctx()
            .byte_add(usize::try_from(offset).unwrap())
            .cast()
    }

    /// Returns a pointer to the "may leave" flag for this instance specified
    /// for canonical lowering and lifting operations.
    #[inline]
    pub fn instance_flags(&self, instance: RuntimeComponentInstanceIndex) -> InstanceFlags {
        unsafe {
            let ptr = self
                .vmctx_plus_offset::<VMGlobalDefinition>(self.offsets.instance_flags(instance))
                .cast_mut();
            InstanceFlags(SendSyncPtr::new(NonNull::new(ptr).unwrap()))
        }
    }

    /// Returns the store that this component was created with.
    pub fn store(&self) -> *mut dyn VMStore {
        unsafe {
            let ret = *self.vmctx_plus_offset::<*mut dyn VMStore>(self.offsets.store());
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
    pub fn runtime_realloc(&self, idx: RuntimeReallocIndex) -> NonNull<VMFuncRef> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<NonNull<_>>(self.offsets.runtime_realloc(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the async callback pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_callback(&self, idx: RuntimeCallbackIndex) -> NonNull<VMFuncRef> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<NonNull<_>>(self.offsets.runtime_callback(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the post-return pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_post_return(&self, idx: RuntimePostReturnIndex) -> NonNull<VMFuncRef> {
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

    /// Returns the core wasm `funcref` corresponding to the trampoline
    /// specified.
    ///
    /// The returned function is suitable to pass directly to a wasm module
    /// instantiation and the function contains cranelift-compiled trampolines.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn trampoline_func_ref(&self, idx: TrampolineIndex) -> NonNull<VMFuncRef> {
        unsafe {
            let offset = self.offsets.trampoline_func_ref(idx);
            let ret = self.vmctx_plus_offset::<VMFuncRef>(offset);
            debug_assert!(
                mem::transmute::<Option<NonNull<VMWasmCallFunction>>, usize>((*ret).wasm_call)
                    != INVALID_PTR
            );
            debug_assert!((*ret).vmctx as usize != INVALID_PTR);
            NonNull::new(ret.cast_mut()).unwrap()
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
            let storage = self.vmctx_plus_offset_mut(self.offsets.runtime_memory(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr;
        }
    }

    /// Same as `set_runtime_memory` but for realloc function pointers.
    pub fn set_runtime_realloc(&mut self, idx: RuntimeReallocIndex, ptr: NonNull<VMFuncRef>) {
        unsafe {
            let storage = self.vmctx_plus_offset_mut(self.offsets.runtime_realloc(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr.as_ptr();
        }
    }

    /// Same as `set_runtime_memory` but for async callback function pointers.
    pub fn set_runtime_callback(&mut self, idx: RuntimeCallbackIndex, ptr: NonNull<VMFuncRef>) {
        unsafe {
            let storage = self.vmctx_plus_offset_mut(self.offsets.runtime_callback(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr.as_ptr();
        }
    }

    /// Same as `set_runtime_memory` but for post-return function pointers.
    pub fn set_runtime_post_return(
        &mut self,
        idx: RuntimePostReturnIndex,
        ptr: NonNull<VMFuncRef>,
    ) {
        unsafe {
            let storage = self.vmctx_plus_offset_mut(self.offsets.runtime_post_return(idx));
            debug_assert!(*storage as usize == INVALID_PTR);
            *storage = ptr.as_ptr();
        }
    }

    /// Configures host runtime lowering information associated with imported f
    /// functions for the `idx` specified.
    pub fn set_lowering(&mut self, idx: LoweredIndex, lowering: VMLowering) {
        unsafe {
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.lowering_callee(idx)) == INVALID_PTR
            );
            debug_assert!(
                *self.vmctx_plus_offset::<usize>(self.offsets.lowering_data(idx)) == INVALID_PTR
            );
            *self.vmctx_plus_offset_mut(self.offsets.lowering(idx)) = lowering;
        }
    }

    /// Same as `set_lowering` but for the resource.drop functions.
    pub fn set_trampoline(
        &mut self,
        idx: TrampolineIndex,
        wasm_call: NonNull<VMWasmCallFunction>,
        array_call: NonNull<VMArrayCallFunction>,
        type_index: VMSharedTypeIndex,
    ) {
        unsafe {
            let offset = self.offsets.trampoline_func_ref(idx);
            debug_assert!(*self.vmctx_plus_offset::<usize>(offset) == INVALID_PTR);
            let vmctx = VMOpaqueContext::from_vmcomponent(self.vmctx());
            *self.vmctx_plus_offset_mut(offset) = VMFuncRef {
                wasm_call: Some(wasm_call),
                array_call,
                type_index,
                vmctx,
            };
        }
    }

    /// Configures the destructor for a resource at the `idx` specified.
    ///
    /// This is required to be called for each resource as it's defined within a
    /// component during the instantiation process.
    pub fn set_resource_destructor(
        &mut self,
        idx: ResourceIndex,
        dtor: Option<NonNull<VMFuncRef>>,
    ) {
        unsafe {
            let offset = self.offsets.resource_destructor(idx);
            debug_assert!(*self.vmctx_plus_offset::<usize>(offset) == INVALID_PTR);
            *self.vmctx_plus_offset_mut(offset) = dtor;
        }
    }

    /// Returns the destructor, if any, for `idx`.
    ///
    /// This is only valid to call after `set_resource_destructor`, or typically
    /// after instantiation.
    pub fn resource_destructor(&self, idx: ResourceIndex) -> Option<NonNull<VMFuncRef>> {
        unsafe {
            let offset = self.offsets.resource_destructor(idx);
            debug_assert!(*self.vmctx_plus_offset::<usize>(offset) != INVALID_PTR);
            *self.vmctx_plus_offset(offset)
        }
    }

    /// Set the host-provided callbacks for various async-, future-, stream-,
    /// and error-context-related built-in functions.
    pub fn set_async_callbacks(
        &mut self,
        task_backpressure: VMTaskBackpressureCallback,
        task_return: VMTaskReturnCallback,
        task_wait: VMTaskWaitOrPollCallback,
        task_poll: VMTaskWaitOrPollCallback,
        task_yield: VMTaskYieldCallback,
        subtask_drop: VMSubtaskDropCallback,
        async_enter: VMAsyncEnterCallback,
        async_exit: VMAsyncExitCallback,
        future_new: VMFutureNewCallback,
        future_write: VMFutureTransmitCallback,
        future_read: VMFutureTransmitCallback,
        future_cancel_write: VMFutureCancelCallback,
        future_cancel_read: VMFutureCancelCallback,
        future_close_writable: VMFutureCloseWritableCallback,
        future_close_readable: VMFutureCloseReadableCallback,
        stream_new: VMStreamNewCallback,
        stream_write: VMStreamTransmitCallback,
        stream_read: VMStreamTransmitCallback,
        stream_cancel_write: VMStreamCancelCallback,
        stream_cancel_read: VMStreamCancelCallback,
        stream_close_writable: VMStreamCloseWritableCallback,
        stream_close_readable: VMStreamCloseReadableCallback,
        flat_stream_write: VMFlatStreamTransmitCallback,
        flat_stream_read: VMFlatStreamTransmitCallback,
        error_context_new: VMErrorContextNewCallback,
        error_context_debug_message: VMErrorContextDebugMessageCallback,
        error_context_drop: VMErrorContextDropCallback,
    ) {
        unsafe {
            *self.vmctx_plus_offset_mut(self.offsets.task_backpressure()) = task_backpressure;
            *self.vmctx_plus_offset_mut(self.offsets.task_return()) = task_return;
            *self.vmctx_plus_offset_mut(self.offsets.task_wait()) = task_wait;
            *self.vmctx_plus_offset_mut(self.offsets.task_poll()) = task_poll;
            *self.vmctx_plus_offset_mut(self.offsets.task_yield()) = task_yield;
            *self.vmctx_plus_offset_mut(self.offsets.subtask_drop()) = subtask_drop;
            *self.vmctx_plus_offset_mut(self.offsets.async_enter()) = async_enter;
            *self.vmctx_plus_offset_mut(self.offsets.async_exit()) = async_exit;
            *self.vmctx_plus_offset_mut(self.offsets.future_new()) = future_new;
            *self.vmctx_plus_offset_mut(self.offsets.future_write()) = future_write;
            *self.vmctx_plus_offset_mut(self.offsets.future_read()) = future_read;
            *self.vmctx_plus_offset_mut(self.offsets.future_cancel_write()) = future_cancel_write;
            *self.vmctx_plus_offset_mut(self.offsets.future_cancel_read()) = future_cancel_read;
            *self.vmctx_plus_offset_mut(self.offsets.future_close_writable()) =
                future_close_writable;
            *self.vmctx_plus_offset_mut(self.offsets.future_close_readable()) =
                future_close_readable;
            *self.vmctx_plus_offset_mut(self.offsets.stream_new()) = stream_new;
            *self.vmctx_plus_offset_mut(self.offsets.stream_write()) = stream_write;
            *self.vmctx_plus_offset_mut(self.offsets.stream_read()) = stream_read;
            *self.vmctx_plus_offset_mut(self.offsets.stream_cancel_write()) = stream_cancel_write;
            *self.vmctx_plus_offset_mut(self.offsets.stream_cancel_read()) = stream_cancel_read;
            *self.vmctx_plus_offset_mut(self.offsets.stream_close_writable()) =
                stream_close_writable;
            *self.vmctx_plus_offset_mut(self.offsets.stream_close_readable()) =
                stream_close_readable;
            *self.vmctx_plus_offset_mut(self.offsets.flat_stream_write()) = flat_stream_write;
            *self.vmctx_plus_offset_mut(self.offsets.flat_stream_read()) = flat_stream_read;
            *self.vmctx_plus_offset_mut(self.offsets.error_context_debug_message()) =
                error_context_new;
            *self.vmctx_plus_offset_mut(self.offsets.error_context_debug_message()) =
                error_context_debug_message;
            *self.vmctx_plus_offset_mut(self.offsets.error_context_drop()) = error_context_drop;
        }
    }

    unsafe fn initialize_vmctx(&mut self, store: *mut dyn VMStore) {
        *self.vmctx_plus_offset_mut(self.offsets.magic()) = VMCOMPONENT_MAGIC;
        *self.vmctx_plus_offset_mut(self.offsets.builtins()) = &libcalls::VMComponentBuiltins::INIT;
        *self.vmctx_plus_offset_mut(self.offsets.store()) = store;
        *self.vmctx_plus_offset_mut(self.offsets.limits()) = (*store).vmruntime_limits();

        for i in 0..self.offsets.num_runtime_component_instances {
            let i = RuntimeComponentInstanceIndex::from_u32(i);
            let mut def = VMGlobalDefinition::new();
            *def.as_i32_mut() = FLAG_MAY_ENTER | FLAG_MAY_LEAVE;
            *self.instance_flags(i).as_raw() = def;
        }

        // In debug mode set non-null bad values to all "pointer looking" bits
        // and pieces related to lowering and such. This'll help detect any
        // erroneous usage and enable debug assertions above as well to prevent
        // loading these before they're configured or setting them twice.
        if cfg!(debug_assertions) {
            for i in 0..self.offsets.num_lowerings {
                let i = LoweredIndex::from_u32(i);
                let offset = self.offsets.lowering_callee(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
                let offset = self.offsets.lowering_data(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_trampolines {
                let i = TrampolineIndex::from_u32(i);
                let offset = self.offsets.trampoline_func_ref(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_memories {
                let i = RuntimeMemoryIndex::from_u32(i);
                let offset = self.offsets.runtime_memory(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_reallocs {
                let i = RuntimeReallocIndex::from_u32(i);
                let offset = self.offsets.runtime_realloc(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_callbacks {
                let i = RuntimeCallbackIndex::from_u32(i);
                let offset = self.offsets.runtime_callback(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_runtime_post_returns {
                let i = RuntimePostReturnIndex::from_u32(i);
                let offset = self.offsets.runtime_post_return(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
            for i in 0..self.offsets.num_resources {
                let i = ResourceIndex::from_u32(i);
                let offset = self.offsets.resource_destructor(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
        }
    }

    /// Returns a reference to the component type information for this instance.
    pub fn component(&self) -> &Component {
        self.runtime_info.component()
    }

    /// Returns the type information that this instance is instantiated with.
    pub fn component_types(&self) -> &Arc<ComponentTypes> {
        self.runtime_info.component_types()
    }

    /// Get the canonical ABI's `realloc` function's runtime type.
    pub fn realloc_func_ty(&self) -> &Arc<dyn Any + Send + Sync> {
        self.runtime_info.realloc_func_type()
    }

    /// Returns a reference to the resource type information as a `dyn Any`.
    ///
    /// Wasmtime is the one which then downcasts this to the appropriate type.
    pub fn resource_types(&self) -> &Arc<dyn Any + Send + Sync> {
        &self.resource_types
    }

    /// Returns whether the resource that `ty` points to is owned by the
    /// instance that `ty` correspond to.
    ///
    /// This is used when lowering borrows to skip table management and instead
    /// thread through the underlying representation directly.
    pub fn resource_owned_by_own_instance(&self, ty: TypeResourceTableIndex) -> bool {
        let resource = &self.component_types()[ty];
        let component = self.component();
        let idx = match component.defined_resource_index(resource.ty) {
            Some(idx) => idx,
            None => return false,
        };
        resource.instance == component.defined_resource_instances[idx]
    }

    /// Implementation of the `resource.new` intrinsic for `i32`
    /// representations.
    pub fn resource_new32(&mut self, resource: TypeResourceTableIndex, rep: u32) -> Result<u32> {
        self.resource_tables().resource_new(Some(resource), rep)
    }

    /// Implementation of the `resource.rep` intrinsic for `i32`
    /// representations.
    pub fn resource_rep32(&mut self, resource: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        self.resource_tables().resource_rep(Some(resource), idx)
    }

    /// Implementation of the `resource.drop` intrinsic.
    pub fn resource_drop(
        &mut self,
        resource: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<Option<u32>> {
        self.resource_tables().resource_drop(Some(resource), idx)
    }

    /// NB: this is intended to be a private method. This does not have
    /// `host_table` information at this time meaning it's only suitable for
    /// working with resources specified to this component which is currently
    /// all that this is used for.
    ///
    /// If necessary though it's possible to enhance the `Store` trait to thread
    /// through the relevant information and get `host_table` to be `Some` here.
    fn resource_tables(&mut self) -> ResourceTables<'_> {
        ResourceTables {
            host_table: None,
            calls: unsafe { (&mut *self.store()).component_calls() },
            tables: Some(&mut self.component_resource_tables),
        }
    }

    /// Returns the runtime state of resources associated with this component.
    #[inline]
    pub fn component_resource_tables(
        &mut self,
    ) -> &mut PrimaryMap<TypeResourceTableIndex, ResourceTable> {
        &mut self.component_resource_tables
    }

    /// Retrieves the tables for tracking waitable handles and their states with respect
    /// to the components which own them.
    pub fn component_waitable_tables(
        &mut self,
    ) -> &mut PrimaryMap<RuntimeComponentInstanceIndex, StateTable<WaitableState>> {
        &mut self.component_waitable_tables
    }

    /// Retrieves the tables for tracking error-context handles and their reference
    /// counts with respect to the components which own them.
    pub fn component_error_context_tables(
        &mut self,
    ) -> &mut PrimaryMap<TypeErrorContextTableIndex, StateTable<usize>> {
        &mut self.component_error_context_tables
    }

    /// Returns the destructor and instance flags for the specified resource
    /// table type.
    ///
    /// This will lookup the origin definition of the `ty` table and return the
    /// destructor/flags for that.
    pub fn dtor_and_flags(
        &self,
        ty: TypeResourceTableIndex,
    ) -> (Option<NonNull<VMFuncRef>>, Option<InstanceFlags>) {
        let resource = self.component_types()[ty].ty;
        let dtor = self.resource_destructor(resource);
        let component = self.component();
        let flags = component.defined_resource_index(resource).map(|i| {
            let instance = component.defined_resource_instances[i];
            self.instance_flags(instance)
        });
        (dtor, flags)
    }

    pub(crate) fn resource_transfer_own(
        &mut self,
        idx: u32,
        src: TypeResourceTableIndex,
        dst: TypeResourceTableIndex,
    ) -> Result<u32> {
        let mut tables = self.resource_tables();
        let rep = tables.resource_lift_own(Some(src), idx)?;
        tables.resource_lower_own(Some(dst), rep)
    }

    pub(crate) fn resource_transfer_borrow(
        &mut self,
        idx: u32,
        src: TypeResourceTableIndex,
        dst: TypeResourceTableIndex,
    ) -> Result<u32> {
        let dst_owns_resource = self.resource_owned_by_own_instance(dst);
        let mut tables = self.resource_tables();
        let rep = tables.resource_lift_borrow(Some(src), idx)?;
        // Implement `lower_borrow`'s special case here where if a borrow's
        // resource type is owned by `dst` then the destination receives the
        // representation directly rather than a handle to the representation.
        //
        // This can perhaps become a different libcall in the future to avoid
        // this check at runtime since we know at compile time whether the
        // destination type owns the resource, but that's left as a future
        // refactoring if truly necessary.
        if dst_owns_resource {
            return Ok(rep);
        }
        tables.resource_lower_borrow(Some(dst), rep)
    }

    pub(crate) fn resource_enter_call(&mut self) {
        self.resource_tables().enter_call()
    }

    pub(crate) fn resource_exit_call(&mut self) -> Result<()> {
        self.resource_tables().exit_call()
    }

    pub(crate) fn future_transfer(
        &mut self,
        src_idx: u32,
        src: TypeFutureTableIndex,
        dst: TypeFutureTableIndex,
    ) -> Result<u32> {
        let src_instance = self.component_types()[src].instance;
        let dst_instance = self.component_types()[dst].instance;
        let [src_table, dst_table] = self
            .component_waitable_tables
            .get_many_mut([src_instance, dst_instance])
            .unwrap();
        let (rep, WaitableState::Future(src_ty, src_state)) =
            src_table.get_mut_by_index(src_idx)?
        else {
            bail!("invalid future handle");
        };
        if *src_ty != src {
            bail!("invalid future handle");
        }
        match src_state {
            StreamFutureState::Local => {
                *src_state = StreamFutureState::Write;
                assert!(dst_table.get_mut_by_rep(rep).is_none());
                dst_table.insert(rep, WaitableState::Future(dst, StreamFutureState::Read))
            }
            StreamFutureState::Read => {
                src_table.remove_by_index(src_idx)?;
                if let Some((dst_idx, dst_state)) = dst_table.get_mut_by_rep(rep) {
                    let WaitableState::Future(dst_ty, dst_state) = dst_state else {
                        unreachable!();
                    };
                    assert_eq!(*dst_ty, dst);
                    assert_eq!(*dst_state, StreamFutureState::Write);
                    *dst_state = StreamFutureState::Local;
                    Ok(dst_idx)
                } else {
                    dst_table.insert(rep, WaitableState::Future(dst, StreamFutureState::Read))
                }
            }
            StreamFutureState::Write => bail!("cannot transfer write end of future"),
            StreamFutureState::Busy => bail!("cannot transfer busy future"),
        }
    }

    pub(crate) fn stream_transfer(
        &mut self,
        src_idx: u32,
        src: TypeStreamTableIndex,
        dst: TypeStreamTableIndex,
    ) -> Result<u32> {
        let src_instance = self.component_types()[src].instance;
        let dst_instance = self.component_types()[dst].instance;
        let [src_table, dst_table] = self
            .component_waitable_tables
            .get_many_mut([src_instance, dst_instance])
            .unwrap();
        let (rep, WaitableState::Stream(src_ty, src_state)) =
            src_table.get_mut_by_index(src_idx)?
        else {
            bail!("invalid stream handle");
        };
        if *src_ty != src {
            bail!("invalid stream handle");
        }
        match src_state {
            StreamFutureState::Local => {
                *src_state = StreamFutureState::Write;
                assert!(dst_table.get_mut_by_rep(rep).is_none());
                dst_table.insert(rep, WaitableState::Stream(dst, StreamFutureState::Read))
            }
            StreamFutureState::Read => {
                src_table.remove_by_index(src_idx)?;
                if let Some((dst_idx, dst_state)) = dst_table.get_mut_by_rep(rep) {
                    let WaitableState::Stream(dst_ty, dst_state) = dst_state else {
                        unreachable!();
                    };
                    assert_eq!(*dst_ty, dst);
                    assert_eq!(*dst_state, StreamFutureState::Write);
                    *dst_state = StreamFutureState::Local;
                    Ok(dst_idx)
                } else {
                    dst_table.insert(rep, WaitableState::Stream(dst, StreamFutureState::Read))
                }
            }
            StreamFutureState::Write => bail!("cannot transfer write end of stream"),
            StreamFutureState::Busy => bail!("cannot transfer busy stream"),
        }
    }

    pub(crate) fn error_context_transfer(
        &mut self,
        src_idx: u32,
        src: TypeErrorContextTableIndex,
        dst: TypeErrorContextTableIndex,
    ) -> Result<u32> {
        let (rep, _) = self.component_error_context_tables[src].get_mut_by_index(src_idx)?;
        let dst = &mut self.component_error_context_tables[dst];

        if let Some((dst_idx, dst_state)) = dst.get_mut_by_rep(rep) {
            *dst_state += 1;
            Ok(dst_idx)
        } else {
            dst.insert(rep, 1)
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
    pub(crate) ptr: SendSyncPtr<ComponentInstance>,
}

impl OwnedComponentInstance {
    /// Allocates a new `ComponentInstance + VMComponentContext` pair on the
    /// heap with `malloc` and configures it for the `component` specified.
    pub fn new(
        runtime_info: Arc<dyn ComponentRuntimeInfo>,
        resource_types: Arc<dyn Any + Send + Sync>,
        store: *mut dyn VMStore,
    ) -> OwnedComponentInstance {
        let component = runtime_info.component();
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
            let ptr = alloc::alloc::alloc_zeroed(layout) as *mut ComponentInstance;
            let ptr = NonNull::new(ptr).unwrap();

            ComponentInstance::new_at(
                ptr,
                layout.size(),
                offsets,
                runtime_info,
                resource_types,
                store,
            );

            let ptr = SendSyncPtr::new(ptr);
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

    /// Returns the underlying component instance's raw pointer.
    pub fn instance_ptr(&self) -> *mut ComponentInstance {
        self.ptr.as_ptr()
    }

    /// See `ComponentInstance::set_runtime_memory`
    pub fn set_runtime_memory(&mut self, idx: RuntimeMemoryIndex, ptr: *mut VMMemoryDefinition) {
        unsafe { self.instance_mut().set_runtime_memory(idx, ptr) }
    }

    /// See `ComponentInstance::set_runtime_realloc`
    pub fn set_runtime_realloc(&mut self, idx: RuntimeReallocIndex, ptr: NonNull<VMFuncRef>) {
        unsafe { self.instance_mut().set_runtime_realloc(idx, ptr) }
    }

    /// See `ComponentInstance::set_runtime_callback`
    pub fn set_runtime_callback(&mut self, idx: RuntimeCallbackIndex, ptr: NonNull<VMFuncRef>) {
        unsafe { self.instance_mut().set_runtime_callback(idx, ptr) }
    }

    /// See `ComponentInstance::set_runtime_post_return`
    pub fn set_runtime_post_return(
        &mut self,
        idx: RuntimePostReturnIndex,
        ptr: NonNull<VMFuncRef>,
    ) {
        unsafe { self.instance_mut().set_runtime_post_return(idx, ptr) }
    }

    /// See `ComponentInstance::set_lowering`
    pub fn set_lowering(&mut self, idx: LoweredIndex, lowering: VMLowering) {
        unsafe { self.instance_mut().set_lowering(idx, lowering) }
    }

    /// See `ComponentInstance::set_resource_drop`
    pub fn set_trampoline(
        &mut self,
        idx: TrampolineIndex,
        wasm_call: NonNull<VMWasmCallFunction>,
        array_call: NonNull<VMArrayCallFunction>,
        type_index: VMSharedTypeIndex,
    ) {
        unsafe {
            self.instance_mut()
                .set_trampoline(idx, wasm_call, array_call, type_index)
        }
    }

    /// See `ComponentInstance::set_resource_destructor`
    pub fn set_resource_destructor(
        &mut self,
        idx: ResourceIndex,
        dtor: Option<NonNull<VMFuncRef>>,
    ) {
        unsafe { self.instance_mut().set_resource_destructor(idx, dtor) }
    }

    /// See `ComponentInstance::resource_types`
    pub fn resource_types_mut(&mut self) -> &mut Arc<dyn Any + Send + Sync> {
        unsafe { &mut (*self.ptr.as_ptr()).resource_types }
    }

    /// See `ComponentInstance::set_async_callbacks`
    pub fn set_async_callbacks(
        &mut self,
        task_backpressure: VMTaskBackpressureCallback,
        task_return: VMTaskReturnCallback,
        task_wait: VMTaskWaitOrPollCallback,
        task_poll: VMTaskWaitOrPollCallback,
        task_yield: VMTaskYieldCallback,
        subtask_drop: VMSubtaskDropCallback,
        async_enter: VMAsyncEnterCallback,
        async_exit: VMAsyncExitCallback,
        future_new: VMFutureNewCallback,
        future_write: VMFutureTransmitCallback,
        future_read: VMFutureTransmitCallback,
        future_cancel_write: VMFutureCancelCallback,
        future_cancel_read: VMFutureCancelCallback,
        future_close_writable: VMFutureCloseWritableCallback,
        future_close_readable: VMFutureCloseReadableCallback,
        stream_new: VMStreamNewCallback,
        stream_write: VMStreamTransmitCallback,
        stream_read: VMStreamTransmitCallback,
        stream_cancel_write: VMStreamCancelCallback,
        stream_cancel_read: VMStreamCancelCallback,
        stream_close_writable: VMStreamCloseWritableCallback,
        stream_close_readable: VMStreamCloseReadableCallback,
        flat_stream_write: VMFlatStreamTransmitCallback,
        flat_stream_read: VMFlatStreamTransmitCallback,
        error_context_new: VMErrorContextNewCallback,
        error_context_debug_message: VMErrorContextDebugMessageCallback,
        error_context_drop: VMErrorContextDropCallback,
    ) {
        unsafe {
            self.instance_mut().set_async_callbacks(
                task_backpressure,
                task_return,
                task_wait,
                task_poll,
                task_yield,
                subtask_drop,
                async_enter,
                async_exit,
                future_new,
                future_write,
                future_read,
                future_cancel_write,
                future_cancel_read,
                future_close_writable,
                future_close_readable,
                stream_new,
                stream_write,
                stream_read,
                stream_cancel_write,
                stream_cancel_read,
                stream_close_writable,
                stream_close_readable,
                flat_stream_write,
                flat_stream_read,
                error_context_new,
                error_context_debug_message,
                error_context_drop,
            )
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
            alloc::alloc::dealloc(self.ptr.as_ptr().cast(), layout);
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
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct InstanceFlags(SendSyncPtr<VMGlobalDefinition>);

#[allow(missing_docs)]
impl InstanceFlags {
    /// Wraps the given pointer as an `InstanceFlags`
    ///
    /// # Unsafety
    ///
    /// This is a raw pointer argument which needs to be valid for the lifetime
    /// that `InstanceFlags` is used.
    pub unsafe fn from_raw(ptr: *mut u8) -> InstanceFlags {
        InstanceFlags(SendSyncPtr::new(NonNull::new(ptr.cast()).unwrap()))
    }

    #[inline]
    pub unsafe fn may_leave(&self) -> bool {
        *(*self.as_raw()).as_i32() & FLAG_MAY_LEAVE != 0
    }

    #[inline]
    pub unsafe fn set_may_leave(&mut self, val: bool) {
        if val {
            *(*self.as_raw()).as_i32_mut() |= FLAG_MAY_LEAVE;
        } else {
            *(*self.as_raw()).as_i32_mut() &= !FLAG_MAY_LEAVE;
        }
    }

    #[inline]
    pub unsafe fn may_enter(&self) -> bool {
        *(*self.as_raw()).as_i32() & FLAG_MAY_ENTER != 0
    }

    #[inline]
    pub unsafe fn set_may_enter(&mut self, val: bool) {
        if val {
            *(*self.as_raw()).as_i32_mut() |= FLAG_MAY_ENTER;
        } else {
            *(*self.as_raw()).as_i32_mut() &= !FLAG_MAY_ENTER;
        }
    }

    #[inline]
    pub unsafe fn needs_post_return(&self) -> bool {
        *(*self.as_raw()).as_i32() & FLAG_NEEDS_POST_RETURN != 0
    }

    #[inline]
    pub unsafe fn set_needs_post_return(&mut self, val: bool) {
        if val {
            *(*self.as_raw()).as_i32_mut() |= FLAG_NEEDS_POST_RETURN;
        } else {
            *(*self.as_raw()).as_i32_mut() &= !FLAG_NEEDS_POST_RETURN;
        }
    }

    #[inline]
    pub fn as_raw(&self) -> *mut VMGlobalDefinition {
        self.0.as_ptr()
    }
}

/// Runtime information about a component stored locally for reflection.
pub trait ComponentRuntimeInfo: Send + Sync + 'static {
    /// Returns the type information about the compiled component.
    fn component(&self) -> &Component;

    /// Returns a handle to the tables of type information for this component.
    fn component_types(&self) -> &Arc<ComponentTypes>;

    /// Get the `wasmtime::FuncType` for the canonical ABI's `realloc` function.
    fn realloc_func_type(&self) -> &Arc<dyn Any + Send + Sync>;
}
