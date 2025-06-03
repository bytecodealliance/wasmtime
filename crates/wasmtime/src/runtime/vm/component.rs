//! Runtime support for the component model in Wasmtime
//!
//! Currently this runtime support includes a `VMComponentContext` which is
//! similar in purpose to `VMContext`. The context is read from
//! cranelift-generated trampolines when entering the host from a wasm module.
//! Eventually it's intended that module-to-module calls, which would be
//! cranelift-compiled adapters, will use this `VMComponentContext` as well.

use crate::component::{Component, InstancePre, ResourceType, RuntimeImport};
use crate::prelude::*;
use crate::runtime::component::ComponentInstanceId;
use crate::runtime::vm::{
    Export, ExportFunction, ExportGlobal, ExportGlobalKind, SendSyncPtr, VMArrayCallFunction,
    VMContext, VMFuncRef, VMGlobalDefinition, VMMemoryDefinition, VMOpaqueContext, VMStore,
    VMStoreRawPtr, VMTableDefinition, VMTableImport, VMWasmCallFunction, ValRaw, VmPtr, VmSafe,
};
use crate::store::{InstanceId, StoreOpaque};
use alloc::alloc::Layout;
use alloc::sync::Arc;
use core::marker;
use core::mem;
use core::mem::offset_of;
use core::ops::Deref;
use core::ptr::{self, NonNull};
use wasmtime_environ::component::*;
use wasmtime_environ::{
    DefinedTableIndex, EntityIndex, Global, HostPtr, PrimaryMap, VMSharedTypeIndex, WasmValType,
};

#[allow(clippy::cast_possible_truncation)] // it's intended this is truncated on
// 32-bit platforms
const INVALID_PTR: usize = 0xdead_dead_beef_beef_u64 as usize;

mod libcalls;
mod resources;

pub use self::resources::{
    CallContexts, ResourceTable, ResourceTables, TypedResource, TypedResourceIndex,
};

/// Runtime representation of a component instance and all state necessary for
/// the instance itself.
///
/// This type never exists by-value, but rather it's always behind a pointer.
/// The size of the allocation for `ComponentInstance` includes the trailing
/// `VMComponentContext` which is variably sized based on the `offsets`
/// contained within.
#[repr(C)]
pub struct ComponentInstance {
    /// The index within the store of where to find this component instance.
    id: ComponentInstanceId,

    /// Size and offset information for the trailing `VMComponentContext`.
    offsets: VMComponentOffsets<HostPtr>,

    /// For more information about this see the documentation on
    /// `Instance::vmctx_self_reference`.
    vmctx_self_reference: SendSyncPtr<VMComponentContext>,

    /// The component that this instance was created from.
    //
    // NB: in the future if necessary it would be possible to avoid storing an
    // entire `Component` here and instead storing only information such as:
    //
    // * Some reference to `Arc<ComponentTypes>`
    // * Necessary references to closed-over modules which are exported from the
    //   component itself.
    //
    // Otherwise the full guts of this component should only ever be used during
    // the instantiation of this instance, meaning that after instantiation much
    // of the component can be thrown away (theoretically).
    component: Component,

    /// State of resources for this component.
    ///
    /// This is paired with other information to create a `ResourceTables` which
    /// is how this field is manipulated.
    instance_resource_tables: PrimaryMap<RuntimeComponentInstanceIndex, ResourceTable>,

    /// What all compile-time-identified core instances are mapped to within the
    /// `Store` that this component belongs to.
    instances: PrimaryMap<RuntimeInstanceIndex, InstanceId>,

    /// Storage for the type information about resources within this component
    /// instance.
    resource_types: Arc<PrimaryMap<ResourceIndex, ResourceType>>,

    /// Arguments that this instance used to be instantiated.
    ///
    /// Strong references are stored to these arguments since pointers are saved
    /// into the structures such as functions within the
    /// `OwnedComponentInstance` but it's our job to keep them alive.
    ///
    /// One purpose of this storage is to enable embedders to drop a `Linker`,
    /// for example, after a component is instantiated. In that situation if the
    /// arguments weren't held here then they might be dropped, and structures
    /// such as `.lowering()` which point back into the original function would
    /// become stale and use-after-free conditions when used. By preserving the
    /// entire list here though we're guaranteed that nothing is lost for the
    /// duration of the lifetime of this instance.
    imports: Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,

    /// Self-pointer back to `Store<T>` and its functions.
    store: VMStoreRawPtr,

    /// Cached ABI return value from the last-invoked function call along with
    /// the function index that was invoked.
    ///
    /// Used in `post_return_arg_set` and `post_return_arg_take` below.
    post_return_arg: Option<(ExportIndex, ValRaw)>,

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
/// * `caller_instance` - the `RuntimeComponentInstanceIndex` representing the
///   caller component instance, used to track the owner of an async host task.
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
    vmctx: NonNull<VMOpaqueContext>,
    data: NonNull<u8>,
    ty: u32,
    caller_instance: u32,
    flags: NonNull<VMGlobalDefinition>,
    opt_memory: *mut VMMemoryDefinition,
    opt_realloc: *mut VMFuncRef,
    string_encoding: u8,
    async_: u8,
    args_and_results: NonNull<mem::MaybeUninit<ValRaw>>,
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
    pub data: VmPtr<u8>,
}

// SAFETY: the above structure is repr(C) and only contains `VmSafe` fields.
unsafe impl VmSafe for VMLowering {}

/// This is a marker type to represent the underlying allocation of a
/// `VMComponentContext`.
///
/// This type is similar to `VMContext` for core wasm and is allocated once per
/// component instance in Wasmtime. While the static size of this type is 0 the
/// actual runtime size is variable depending on the shape of the component that
/// this corresponds to. This structure always trails a `ComponentInstance`
/// allocation and the allocation/lifetime of this allocation is managed by
/// `ComponentInstance`.
#[repr(C)]
// Set an appropriate alignment for this structure where the most-aligned value
// internally right now `VMGlobalDefinition` which has an alignment of 16 bytes.
#[repr(align(16))]
pub struct VMComponentContext {
    /// For more information about this see the equivalent field in `VMContext`
    _marker: marker::PhantomPinned,
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
        vmctx: NonNull<VMComponentContext>,
        f: impl FnOnce(&mut ComponentInstance) -> R,
    ) -> R {
        let mut ptr = vmctx
            .byte_sub(mem::size_of::<ComponentInstance>())
            .cast::<ComponentInstance>();
        f(ptr.as_mut())
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
        id: ComponentInstanceId,
        component: &Component,
        resource_types: Arc<PrimaryMap<ResourceIndex, ResourceType>>,
        imports: &Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,
        store: NonNull<dyn VMStore>,
    ) {
        assert!(alloc_size >= Self::alloc_layout(&offsets).size());

        let num_instances = component.env_component().num_runtime_component_instances;
        let mut instance_resource_tables =
            PrimaryMap::with_capacity(num_instances.try_into().unwrap());
        for _ in 0..num_instances {
            instance_resource_tables.push(ResourceTable::default());
        }

        ptr::write(
            ptr.as_ptr(),
            ComponentInstance {
                id,
                offsets,
                vmctx_self_reference: SendSyncPtr::new(
                    NonNull::new(
                        ptr.as_ptr()
                            .byte_add(mem::size_of::<ComponentInstance>())
                            .cast(),
                    )
                    .unwrap(),
                ),
                instance_resource_tables,
                instances: PrimaryMap::with_capacity(
                    component
                        .env_component()
                        .num_runtime_instances
                        .try_into()
                        .unwrap(),
                ),
                component: component.clone(),
                resource_types,
                imports: imports.clone(),
                store: VMStoreRawPtr(store),
                post_return_arg: None,
                vmctx: VMComponentContext {
                    _marker: marker::PhantomPinned,
                },
            },
        );

        (*ptr.as_ptr()).initialize_vmctx();
    }

    pub fn vmctx(&self) -> NonNull<VMComponentContext> {
        let addr = &raw const self.vmctx;
        let ret = self.vmctx_self_reference.as_ptr().with_addr(addr.addr());
        NonNull::new(ret).unwrap()
    }

    unsafe fn vmctx_plus_offset<T: VmSafe>(&self, offset: u32) -> *const T {
        self.vmctx()
            .as_ptr()
            .byte_add(usize::try_from(offset).unwrap())
            .cast()
    }

    unsafe fn vmctx_plus_offset_mut<T: VmSafe>(&mut self, offset: u32) -> *mut T {
        self.vmctx()
            .as_ptr()
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
        self.store.0.as_ptr()
    }

    /// Returns the runtime memory definition corresponding to the index of the
    /// memory provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_memory(&self, idx: RuntimeMemoryIndex) -> *mut VMMemoryDefinition {
        unsafe {
            let ret = *self.vmctx_plus_offset::<VmPtr<_>>(self.offsets.runtime_memory(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret.as_ptr()
        }
    }

    /// Returns the runtime table definition and associated instance `VMContext`
    /// corresponding to the index of the table provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_table(&self, idx: RuntimeTableIndex) -> VMTableImport {
        unsafe {
            let ret = *self.vmctx_plus_offset::<VMTableImport>(self.offsets.runtime_table(idx));
            debug_assert!(ret.from.as_ptr() as usize != INVALID_PTR);
            debug_assert!(ret.vmctx.as_ptr() as usize != INVALID_PTR);
            ret
        }
    }

    /// Returns the realloc pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_realloc(&self, idx: RuntimeReallocIndex) -> NonNull<VMFuncRef> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<VmPtr<_>>(self.offsets.runtime_realloc(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret.as_non_null()
        }
    }

    /// Returns the post-return pointer corresponding to the index provided.
    ///
    /// This can only be called after `idx` has been initialized at runtime
    /// during the instantiation process of a component.
    pub fn runtime_post_return(&self, idx: RuntimePostReturnIndex) -> NonNull<VMFuncRef> {
        unsafe {
            let ret = *self.vmctx_plus_offset::<VmPtr<_>>(self.offsets.runtime_post_return(idx));
            debug_assert!(ret.as_ptr() as usize != INVALID_PTR);
            ret.as_non_null()
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
            debug_assert!(ret.data.as_ptr() as usize != INVALID_PTR);
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
                mem::transmute::<Option<VmPtr<VMWasmCallFunction>>, usize>((*ret).wasm_call)
                    != INVALID_PTR
            );
            debug_assert!((*ret).vmctx.as_ptr() as usize != INVALID_PTR);
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
    pub fn set_runtime_memory(
        &mut self,
        idx: RuntimeMemoryIndex,
        ptr: NonNull<VMMemoryDefinition>,
    ) {
        unsafe {
            let storage = self.vmctx_plus_offset_mut::<VmPtr<VMMemoryDefinition>>(
                self.offsets.runtime_memory(idx),
            );
            debug_assert!((*storage).as_ptr() as usize == INVALID_PTR);
            *storage = ptr.into();
        }
    }

    /// Same as `set_runtime_memory` but for realloc function pointers.
    pub fn set_runtime_realloc(&mut self, idx: RuntimeReallocIndex, ptr: NonNull<VMFuncRef>) {
        unsafe {
            let storage =
                self.vmctx_plus_offset_mut::<VmPtr<VMFuncRef>>(self.offsets.runtime_realloc(idx));
            debug_assert!((*storage).as_ptr() as usize == INVALID_PTR);
            *storage = ptr.into();
        }
    }

    /// Same as `set_runtime_memory` but for async callback function pointers.
    pub fn set_runtime_callback(&mut self, idx: RuntimeCallbackIndex, ptr: NonNull<VMFuncRef>) {
        unsafe {
            let storage =
                self.vmctx_plus_offset_mut::<VmPtr<VMFuncRef>>(self.offsets.runtime_callback(idx));
            debug_assert!((*storage).as_ptr() as usize == INVALID_PTR);
            *storage = ptr.into();
        }
    }

    /// Same as `set_runtime_memory` but for post-return function pointers.
    pub fn set_runtime_post_return(
        &mut self,
        idx: RuntimePostReturnIndex,
        ptr: NonNull<VMFuncRef>,
    ) {
        unsafe {
            let storage = self
                .vmctx_plus_offset_mut::<VmPtr<VMFuncRef>>(self.offsets.runtime_post_return(idx));
            debug_assert!((*storage).as_ptr() as usize == INVALID_PTR);
            *storage = ptr.into();
        }
    }

    /// Stores the runtime table pointer at the index specified.
    ///
    /// This is intended to be called during the instantiation process of a
    /// component once a table is available, which may not be until part-way
    /// through component instantiation.
    ///
    /// Note that it should be a property of the component model that the `ptr`
    /// here is never needed prior to it being configured here in the instance.
    pub fn set_runtime_table(
        &mut self,
        idx: RuntimeTableIndex,
        ptr: NonNull<VMTableDefinition>,
        vmctx: NonNull<VMContext>,
        index: DefinedTableIndex,
    ) {
        unsafe {
            let storage =
                self.vmctx_plus_offset_mut::<VMTableImport>(self.offsets.runtime_table(idx));
            debug_assert!((*storage).vmctx.as_ptr() as usize == INVALID_PTR);
            debug_assert!((*storage).from.as_ptr() as usize == INVALID_PTR);
            *storage = VMTableImport {
                vmctx: vmctx.into(),
                from: ptr.into(),
                index,
            };
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
                wasm_call: Some(wasm_call.into()),
                array_call: array_call.into(),
                type_index,
                vmctx: vmctx.into(),
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
            *self.vmctx_plus_offset_mut(offset) = dtor.map(VmPtr::from);
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
            (*self.vmctx_plus_offset::<Option<VmPtr<VMFuncRef>>>(offset)).map(|p| p.as_non_null())
        }
    }

    unsafe fn initialize_vmctx(&mut self) {
        *self.vmctx_plus_offset_mut(self.offsets.magic()) = VMCOMPONENT_MAGIC;
        *self.vmctx_plus_offset_mut(self.offsets.builtins()) =
            VmPtr::from(NonNull::from(&libcalls::VMComponentBuiltins::INIT));
        *self.vmctx_plus_offset_mut(self.offsets.vm_store_context()) =
            VmPtr::from(self.store.0.as_ref().vm_store_context_ptr());

        for i in 0..self.offsets.num_runtime_component_instances {
            let i = RuntimeComponentInstanceIndex::from_u32(i);
            let mut def = VMGlobalDefinition::new();
            *def.as_i32_mut() = FLAG_MAY_ENTER | FLAG_MAY_LEAVE;
            self.instance_flags(i).as_raw().write(def);
        }

        // In debug mode set non-null bad values to all "pointer looking" bits
        // and pices related to lowering and such. This'll help detect any
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
            for i in 0..self.offsets.num_runtime_tables {
                let i = RuntimeTableIndex::from_u32(i);
                let offset = self.offsets.runtime_table(i);
                *self.vmctx_plus_offset_mut(offset) = INVALID_PTR;
            }
        }
    }

    /// Returns a reference to the component type information for this
    /// instance.
    pub fn component(&self) -> &Component {
        &self.component
    }

    /// Returns a reference to the resource type information.
    pub fn resource_types(&self) -> &Arc<PrimaryMap<ResourceIndex, ResourceType>> {
        &self.resource_types
    }

    /// Returns whether the resource that `ty` points to is owned by the
    /// instance that `ty` correspond to.
    ///
    /// This is used when lowering borrows to skip table management and instead
    /// thread through the underlying representation directly.
    pub fn resource_owned_by_own_instance(&self, ty: TypeResourceTableIndex) -> bool {
        let resource = &self.component.types()[ty];
        let component = self.component.env_component();
        let idx = match component.defined_resource_index(resource.ty) {
            Some(idx) => idx,
            None => return false,
        };
        resource.instance == component.defined_resource_instances[idx]
    }

    /// Implementation of the `resource.new` intrinsic for `i32`
    /// representations.
    pub fn resource_new32(&mut self, ty: TypeResourceTableIndex, rep: u32) -> Result<u32> {
        self.resource_tables()
            .resource_new(TypedResource::Component { ty, rep })
    }

    /// Implementation of the `resource.rep` intrinsic for `i32`
    /// representations.
    pub fn resource_rep32(&mut self, ty: TypeResourceTableIndex, index: u32) -> Result<u32> {
        self.resource_tables()
            .resource_rep(TypedResourceIndex::Component { ty, index })
    }

    /// Implementation of the `resource.drop` intrinsic.
    pub fn resource_drop(&mut self, ty: TypeResourceTableIndex, index: u32) -> Result<Option<u32>> {
        self.resource_tables()
            .resource_drop(TypedResourceIndex::Component { ty, index })
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
            guest: Some((&mut self.instance_resource_tables, self.component.types())),
        }
    }

    /// Returns the runtime state of resources associated with this component.
    #[inline]
    pub fn guest_tables(
        &mut self,
    ) -> (
        &mut PrimaryMap<RuntimeComponentInstanceIndex, ResourceTable>,
        &ComponentTypes,
    ) {
        (&mut self.instance_resource_tables, self.component.types())
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
        let resource = self.component.types()[ty].ty;
        let dtor = self.resource_destructor(resource);
        let component = self.component.env_component();
        let flags = component.defined_resource_index(resource).map(|i| {
            let instance = component.defined_resource_instances[i];
            self.instance_flags(instance)
        });
        (dtor, flags)
    }

    pub(crate) fn resource_transfer_own(
        &mut self,
        index: u32,
        src: TypeResourceTableIndex,
        dst: TypeResourceTableIndex,
    ) -> Result<u32> {
        let mut tables = self.resource_tables();
        let rep = tables.resource_lift_own(TypedResourceIndex::Component { ty: src, index })?;
        tables.resource_lower_own(TypedResource::Component { ty: dst, rep })
    }

    pub(crate) fn resource_transfer_borrow(
        &mut self,
        index: u32,
        src: TypeResourceTableIndex,
        dst: TypeResourceTableIndex,
    ) -> Result<u32> {
        let dst_owns_resource = self.resource_owned_by_own_instance(dst);
        let mut tables = self.resource_tables();
        let rep = tables.resource_lift_borrow(TypedResourceIndex::Component { ty: src, index })?;
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
        tables.resource_lower_borrow(TypedResource::Component { ty: dst, rep })
    }

    pub(crate) fn resource_enter_call(&mut self) {
        self.resource_tables().enter_call()
    }

    pub(crate) fn resource_exit_call(&mut self) -> Result<()> {
        self.resource_tables().exit_call()
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn future_transfer(
        &mut self,
        src_idx: u32,
        src: TypeFutureTableIndex,
        dst: TypeFutureTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn stream_transfer(
        &mut self,
        src_idx: u32,
        src: TypeStreamTableIndex,
        dst: TypeStreamTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn error_context_transfer(
        &mut self,
        src_idx: u32,
        src: TypeComponentLocalErrorContextTableIndex,
        dst: TypeComponentLocalErrorContextTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    /// Returns the store-local id that points to this component.
    pub fn id(&self) -> ComponentInstanceId {
        self.id
    }

    /// Pushes a new runtime instance that's been created into
    /// `self.instances`.
    pub fn push_instance_id(&mut self, id: InstanceId) -> RuntimeInstanceIndex {
        self.instances.push(id)
    }

    /// Translates a `CoreDef`, a definition of a core wasm item, to an
    /// [`Export`] which is the runtime core wasm definition.
    pub fn lookup_def(&self, store: &StoreOpaque, def: &CoreDef) -> Export {
        match def {
            CoreDef::Export(e) => self.lookup_export(store, e),
            CoreDef::Trampoline(idx) => Export::Function(ExportFunction {
                func_ref: self.trampoline_func_ref(*idx),
            }),
            CoreDef::InstanceFlags(idx) => Export::Global(ExportGlobal {
                definition: self.instance_flags(*idx).as_raw(),
                global: Global {
                    wasm_ty: WasmValType::I32,
                    mutability: true,
                },
                kind: ExportGlobalKind::ComponentFlags(self.vmctx(), *idx),
            }),
        }
    }

    /// Translates a `CoreExport<T>`, an export of some core instance within
    /// this component, to the actual runtime definition of that item.
    pub fn lookup_export<T>(&self, store: &StoreOpaque, item: &CoreExport<T>) -> Export
    where
        T: Copy + Into<EntityIndex>,
    {
        let id = self.instances[item.instance];
        let instance = store.instance(id);
        let idx = match &item.item {
            ExportItem::Index(idx) => (*idx).into(),

            // FIXME: ideally at runtime we don't actually do any name lookups
            // here. This will only happen when the host supplies an imported
            // module so while the structure can't be known at compile time we
            // do know at `InstancePre` time, for example, what all the host
            // imports are. In theory we should be able to, as part of
            // `InstancePre` construction, perform all name=>index mappings
            // during that phase so the actual instantiation of an `InstancePre`
            // skips all string lookups. This should probably only be
            // investigated if this becomes a performance issue though.
            ExportItem::Name(name) => instance.module().exports[name],
        };
        instance.instance().get_export_by_index(idx)
    }

    /// Looks up the value used for `import` at runtime.
    ///
    /// # Panics
    ///
    /// Panics of `import` is out of bounds for this component.
    pub(crate) fn runtime_import(&self, import: RuntimeImportIndex) -> &RuntimeImport {
        &self.imports[import]
    }

    /// Returns an `InstancePre<T>` which can be used to re-instantiated this
    /// component if desired.
    ///
    /// # Safety
    ///
    /// This function places no bounds on `T` so it's up to the caller to match
    /// that up appropriately with the store that this instance resides within.
    pub unsafe fn instance_pre<T>(&self) -> InstancePre<T> {
        // SAFETY: The `T` part of `new_unchecked` is forwarded as a contract of
        // this function, and otherwise the validity of the components of the
        // InstancePre should be guaranteed as it's what we were built with
        // ourselves.
        unsafe {
            InstancePre::new_unchecked(
                self.component.clone(),
                self.imports.clone(),
                self.resource_types.clone(),
            )
        }
    }

    /// Sets the cached argument for the canonical ABI option `post-return` to
    /// the `arg` specified.
    ///
    /// This function is used in conjunction with function calls to record,
    /// after a fuction call completes, the optional ABI return value. This
    /// return value is cached within this instance for future use when the
    /// `post_return` Rust-API-level function is invoked.
    ///
    /// Note that `index` here is the index of the export that was just
    /// invoked, and this is used to ensure that `post_return` is called on the
    /// same function afterwards. This restriction technically isn't necessary
    /// though and may be one we want to lift in the future.
    ///
    /// # Panics
    ///
    /// This function will panic if `post_return_arg` is already set to `Some`.
    pub fn post_return_arg_set(&mut self, index: ExportIndex, arg: ValRaw) {
        assert!(self.post_return_arg.is_none());
        self.post_return_arg = Some((index, arg));
    }

    /// Re-acquires the value originally saved via `post_return_arg_set`.
    ///
    /// This function will take a function `index` that's having its
    /// `post_return` function called. If an argument was previously stored and
    /// `index` matches the index that was stored then `Some(arg)` is returned.
    /// Otherwise `None` is returned.
    pub fn post_return_arg_take(&mut self, index: ExportIndex) -> Option<ValRaw> {
        let (expected_index, arg) = self.post_return_arg.take()?;
        if index != expected_index {
            self.post_return_arg = Some((expected_index, arg));
            None
        } else {
            Some(arg)
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
    ptr: SendSyncPtr<ComponentInstance>,
}

impl OwnedComponentInstance {
    /// Allocates a new `ComponentInstance + VMComponentContext` pair on the
    /// heap with `malloc` and configures it for the `component` specified.
    pub fn new(
        id: ComponentInstanceId,
        component: &Component,
        resource_types: Arc<PrimaryMap<ResourceIndex, ResourceType>>,
        imports: &Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,
        store: NonNull<dyn VMStore>,
    ) -> OwnedComponentInstance {
        let offsets = VMComponentOffsets::new(HostPtr, component.env_component());
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
                id,
                component,
                resource_types,
                imports,
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
    pub fn instance_ptr(&self) -> NonNull<ComponentInstance> {
        self.ptr.as_non_null()
    }

    /// See `ComponentInstance::set_runtime_memory`
    pub fn set_runtime_memory(
        &mut self,
        idx: RuntimeMemoryIndex,
        ptr: NonNull<VMMemoryDefinition>,
    ) {
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

    /// See `ComponentInstance::set_runtime_table`
    pub fn set_runtime_table(
        &mut self,
        idx: RuntimeTableIndex,
        ptr: NonNull<VMTableDefinition>,
        vmctx: NonNull<VMContext>,
        index: DefinedTableIndex,
    ) {
        unsafe {
            self.instance_mut()
                .set_runtime_table(idx, ptr, vmctx, index)
        }
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
    pub fn resource_types_mut(&mut self) -> &mut Arc<PrimaryMap<ResourceIndex, ResourceType>> {
        unsafe { &mut (*self.ptr.as_ptr()).resource_types }
    }

    /// See `ComponentInstance::push_instance_id`
    pub fn push_instance_id(&mut self, id: InstanceId) -> RuntimeInstanceIndex {
        unsafe { self.instance_mut().push_instance_id(id) }
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
    pub unsafe fn from_opaque(opaque: NonNull<VMOpaqueContext>) -> NonNull<VMComponentContext> {
        // See comments in `VMContext::from_opaque` for this debug assert
        debug_assert_eq!(opaque.as_ref().magic, VMCOMPONENT_MAGIC);
        opaque.cast()
    }
}

impl VMOpaqueContext {
    /// Helper function to clearly indicate the cast desired
    #[inline]
    pub fn from_vmcomponent(ptr: NonNull<VMComponentContext>) -> NonNull<VMOpaqueContext> {
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
    pub unsafe fn from_raw(ptr: NonNull<VMGlobalDefinition>) -> InstanceFlags {
        InstanceFlags(SendSyncPtr::from(ptr))
    }

    #[inline]
    pub unsafe fn may_leave(&self) -> bool {
        *self.as_raw().as_ref().as_i32() & FLAG_MAY_LEAVE != 0
    }

    #[inline]
    pub unsafe fn set_may_leave(&mut self, val: bool) {
        if val {
            *self.as_raw().as_mut().as_i32_mut() |= FLAG_MAY_LEAVE;
        } else {
            *self.as_raw().as_mut().as_i32_mut() &= !FLAG_MAY_LEAVE;
        }
    }

    #[inline]
    pub unsafe fn may_enter(&self) -> bool {
        *self.as_raw().as_ref().as_i32() & FLAG_MAY_ENTER != 0
    }

    #[inline]
    pub unsafe fn set_may_enter(&mut self, val: bool) {
        if val {
            *self.as_raw().as_mut().as_i32_mut() |= FLAG_MAY_ENTER;
        } else {
            *self.as_raw().as_mut().as_i32_mut() &= !FLAG_MAY_ENTER;
        }
    }

    #[inline]
    pub unsafe fn needs_post_return(&self) -> bool {
        *self.as_raw().as_ref().as_i32() & FLAG_NEEDS_POST_RETURN != 0
    }

    #[inline]
    pub unsafe fn set_needs_post_return(&mut self, val: bool) {
        if val {
            *self.as_raw().as_mut().as_i32_mut() |= FLAG_NEEDS_POST_RETURN;
        } else {
            *self.as_raw().as_mut().as_i32_mut() &= !FLAG_NEEDS_POST_RETURN;
        }
    }

    #[inline]
    pub fn as_raw(&self) -> NonNull<VMGlobalDefinition> {
        self.0.as_non_null()
    }
}
