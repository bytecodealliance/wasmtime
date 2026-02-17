use crate::StoreContextMut;
#[cfg(feature = "component-model-async")]
use crate::component::concurrent::ConcurrentState;
use crate::component::matching::InstanceType;
use crate::component::resources::{HostResourceData, HostResourceIndex, HostResourceTables};
use crate::component::store::ComponentTaskState;
use crate::component::{Instance, ResourceType, RuntimeInstance};
use crate::prelude::*;
use crate::runtime::vm::VMFuncRef;
use crate::runtime::vm::component::{ComponentInstance, HandleTable, ResourceTables};
use crate::store::{StoreId, StoreOpaque};
use alloc::sync::Arc;
use core::pin::Pin;
use core::ptr::NonNull;
use wasmtime_environ::component::{
    CanonicalOptions, CanonicalOptionsDataModel, ComponentTypes, OptionsIndex,
    TypeResourceTableIndex,
};

/// A helper structure which is a "package" of the context used during lowering
/// values into a component (or storing them into memory).
///
/// This type is used by the `Lower` trait extensively and contains any
/// contextual information necessary related to the context in which the
/// lowering is happening.
#[doc(hidden)]
pub struct LowerContext<'a, T: 'static> {
    /// Lowering may involve invoking memory allocation functions so part of the
    /// context here is carrying access to the entire store that wasm is
    /// executing within. This store serves as proof-of-ability to actually
    /// execute wasm safely.
    pub store: StoreContextMut<'a, T>,

    /// Lowering always happens into a function that's been `canon lift`'d or
    /// `canon lower`'d, both of which specify a set of options for the
    /// canonical ABI. For example details like string encoding are contained
    /// here along with which memory pointers are relative to or what the memory
    /// allocation function is.
    options: OptionsIndex,

    /// Lowering happens within the context of a component instance and this
    /// field stores the type information of that component instance. This is
    /// used for type lookups and general type queries during the
    /// lifting/lowering process.
    pub types: &'a ComponentTypes,

    /// Index of the component instance that's being lowered into.
    instance: Instance,

    /// Whether to allow `options.realloc` to be used when lowering.
    allow_realloc: bool,
}

#[doc(hidden)]
impl<'a, T: 'static> LowerContext<'a, T> {
    /// Creates a new lowering context from the specified parameters.
    pub fn new(
        store: StoreContextMut<'a, T>,
        options: OptionsIndex,
        instance: Instance,
    ) -> LowerContext<'a, T> {
        // Debug-assert that if we can't block that blocking is indeed allowed.
        // This'll catch when this is accidentally created outside of a fiber
        // when we need to be on a fiber.
        if cfg!(debug_assertions) && !store.0.can_block() {
            store.0.validate_sync_call().unwrap();
        }
        let (component, store) = instance.component_and_store_mut(store.0);
        LowerContext {
            store: StoreContextMut(store),
            options,
            types: component.types(),
            instance,
            allow_realloc: true,
        }
    }

    /// Like `new`, except disallows use of `options.realloc`.
    ///
    /// The returned object will panic if its `realloc` method is called.
    ///
    /// This is meant for use when lowering "flat" values (i.e. values which
    /// require no allocations) into already-allocated memory or into stack
    /// slots, in which case the lowering may safely be done outside of a fiber
    /// since there is no need to make any guest calls.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn new_without_realloc(
        store: StoreContextMut<'a, T>,
        options: OptionsIndex,
        instance: Instance,
    ) -> LowerContext<'a, T> {
        let (component, store) = instance.component_and_store_mut(store.0);
        LowerContext {
            store: StoreContextMut(store),
            options,
            types: component.types(),
            instance,
            allow_realloc: false,
        }
    }

    /// Returns the `&ComponentInstance` that's being lowered into.
    pub fn instance(&self) -> &ComponentInstance {
        self.instance.id().get(self.store.0)
    }

    /// Returns the `&mut ComponentInstance` that's being lowered into.
    pub fn instance_mut(&mut self) -> Pin<&mut ComponentInstance> {
        self.instance.id().get_mut(self.store.0)
    }

    /// Returns the canonical options that are being used during lifting.
    pub fn options(&self) -> &CanonicalOptions {
        &self.instance().component().env_component().options[self.options]
    }

    /// Returns a view into memory as a mutable slice of bytes.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lowering
    /// (e.g. it wasn't present during the specification of canonical options).
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.instance.options_memory_mut(self.store.0, self.options)
    }

    /// Invokes the memory allocation function (which is style after `realloc`)
    /// with the specified parameters.
    ///
    /// # Panics
    ///
    /// This will panic if realloc hasn't been configured for this lowering via
    /// its canonical options.
    pub fn realloc(
        &mut self,
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<usize> {
        assert!(self.allow_realloc);

        let (component, store) = self.instance.component_and_store_mut(self.store.0);
        let instance = self.instance.id().get(store);
        let options = &component.env_component().options[self.options];
        let realloc_ty = component.realloc_func_ty();
        let realloc = match options.data_model {
            CanonicalOptionsDataModel::Gc {} => unreachable!(),
            CanonicalOptionsDataModel::LinearMemory(m) => m.realloc.unwrap(),
        };
        let realloc = instance.runtime_realloc(realloc);

        let params = (
            u32::try_from(old)?,
            u32::try_from(old_size)?,
            old_align,
            u32::try_from(new_size)?,
        );

        type ReallocFunc = crate::TypedFunc<(u32, u32, u32, u32), u32>;

        // Invoke the wasm malloc function using its raw and statically known
        // signature.
        let result = unsafe {
            ReallocFunc::call_raw(&mut StoreContextMut(store), &realloc_ty, realloc, params)?
        };

        if result % old_align != 0 {
            bail!("realloc return: result not aligned");
        }
        let result = usize::try_from(result)?;

        if self
            .as_slice_mut()
            .get_mut(result..)
            .and_then(|s| s.get_mut(..new_size))
            .is_none()
        {
            bail!("realloc return: beyond end of memory")
        }

        Ok(result)
    }

    /// Returns a fixed mutable slice of memory `N` bytes large starting at
    /// offset `N`, panicking on out-of-bounds.
    ///
    /// It should be previously verified that `offset` is in-bounds via
    /// bounds-checks.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lowering
    /// (e.g. it wasn't present during the specification of canonical options).
    pub fn get<const N: usize>(&mut self, offset: usize) -> &mut [u8; N] {
        // FIXME: this bounds check shouldn't actually be necessary, all
        // callers of `ComponentType::store` have already performed a bounds
        // check so we're guaranteed that `offset..offset+N` is in-bounds. That
        // being said we at least should do bounds checks in debug mode and
        // it's not clear to me how to easily structure this so that it's
        // "statically obvious" the bounds check isn't necessary.
        //
        // For now I figure we can leave in this bounds check and if it becomes
        // an issue we can optimize further later, probably with judicious use
        // of `unsafe`.
        self.as_slice_mut()[offset..].first_chunk_mut().unwrap()
    }

    /// Lowers an `own` resource into the guest, converting the `rep` specified
    /// into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    pub fn guest_resource_lower_own(
        &mut self,
        ty: TypeResourceTableIndex,
        rep: u32,
    ) -> Result<u32> {
        self.resource_tables().guest_resource_lower_own(rep, ty)
    }

    /// Lowers a `borrow` resource into the guest, converting the `rep` to a
    /// guest-local index in the `ty` table specified.
    pub fn guest_resource_lower_borrow(
        &mut self,
        ty: TypeResourceTableIndex,
        rep: u32,
    ) -> Result<u32> {
        // Implement `lower_borrow`'s special case here where if a borrow is
        // inserted into a table owned by the instance which implemented the
        // original resource then no borrow tracking is employed and instead the
        // `rep` is returned "raw".
        //
        // This check is performed by comparing the owning instance of `ty`
        // against the owning instance of the resource that `ty` is working
        // with.
        if self.instance().resource_owned_by_own_instance(ty) {
            return Ok(rep);
        }
        self.resource_tables().guest_resource_lower_borrow(rep, ty)
    }

    /// Lifts a host-owned `own` resource at the `idx` specified into the
    /// representation of that resource.
    pub fn host_resource_lift_own(&mut self, idx: HostResourceIndex) -> Result<u32> {
        self.resource_tables().host_resource_lift_own(idx)
    }

    /// Lifts a host-owned `borrow` resource at the `idx` specified into the
    /// representation of that resource.
    pub fn host_resource_lift_borrow(&mut self, idx: HostResourceIndex) -> Result<u32> {
        self.resource_tables().host_resource_lift_borrow(idx)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    ///
    /// Note that this is a special case for `Resource<T>`. Most of the time a
    /// host value shouldn't be lowered with a lowering context.
    pub fn host_resource_lower_own(
        &mut self,
        rep: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        instance: Option<RuntimeInstance>,
    ) -> Result<HostResourceIndex> {
        self.resource_tables()
            .host_resource_lower_own(rep, dtor, instance)
    }

    /// Returns the underlying resource type for the `ty` table specified.
    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        self.instance_type().resource_type(ty)
    }

    /// Returns the instance type information corresponding to the instance that
    /// this context is lowering into.
    pub fn instance_type(&self) -> InstanceType<'_> {
        InstanceType::new(self.instance())
    }

    fn resource_tables(&mut self) -> HostResourceTables<'_> {
        let (tables, data) = self
            .store
            .0
            .component_resource_tables_and_host_resource_data(Some(self.instance));
        HostResourceTables::from_parts(tables, data)
    }

    /// See [`HostResourceTables::validate_scope_exit`].
    #[inline]
    pub fn validate_scope_exit(&mut self) -> Result<()> {
        self.resource_tables().validate_scope_exit()
    }
}

/// Contextual information used when lifting a type from a component into the
/// host.
///
/// This structure is the analogue of `LowerContext` except used during lifting
/// operations (or loading from memory).
#[doc(hidden)]
pub struct LiftContext<'a> {
    store_id: StoreId,
    /// Like lowering, lifting always has options configured.
    options: OptionsIndex,

    /// Instance type information, like with lowering.
    pub types: &'a Arc<ComponentTypes>,

    memory: &'a [u8],

    instance: Pin<&'a mut ComponentInstance>,
    instance_handle: Instance,

    host_table: &'a mut HandleTable,
    host_resource_data: &'a mut HostResourceData,

    task_state: &'a mut ComponentTaskState,
}

#[doc(hidden)]
impl<'a> LiftContext<'a> {
    /// Creates a new lifting context given the provided context.
    #[inline]
    pub fn new(
        store: &'a mut StoreOpaque,
        options: OptionsIndex,
        instance_handle: Instance,
    ) -> LiftContext<'a> {
        let store_id = store.id();
        // From `&mut StoreOpaque` provided the goal here is to project out
        // three different disjoint fields owned by the store: memory,
        // `CallContexts`, and `HandleTable`. There's no native API for that
        // so it's hacked around a bit. This unsafe pointer cast could be fixed
        // with more methods in more places, but it doesn't seem worth doing it
        // at this time.
        let memory =
            instance_handle.options_memory(unsafe { &*(store as *const StoreOpaque) }, options);
        let (task_state, host_table, host_resource_data, instance) =
            store.lift_context_parts(instance_handle);
        let (component, instance) = instance.component_and_self();

        LiftContext {
            store_id,
            memory,
            options,
            types: component.types(),
            instance,
            instance_handle,
            task_state,
            host_table,
            host_resource_data,
        }
    }

    /// Returns the canonical options that are being used during lifting.
    pub fn options(&self) -> &CanonicalOptions {
        &self.instance.component().env_component().options[self.options]
    }

    /// Returns the `OptionsIndex` being used during lifting.
    pub fn options_index(&self) -> OptionsIndex {
        self.options
    }

    /// Returns the entire contents of linear memory for this set of lifting
    /// options.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lifting
    /// operation.
    pub fn memory(&self) -> &'a [u8] {
        self.memory
    }

    /// Returns an identifier for the store from which this `LiftContext` was
    /// created.
    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    /// Returns the component instance that is being lifted from.
    pub fn instance_mut(&mut self) -> Pin<&mut ComponentInstance> {
        self.instance.as_mut()
    }
    /// Returns the component instance that is being lifted from.
    pub fn instance_handle(&self) -> Instance {
        self.instance_handle
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn concurrent_state_mut(&mut self) -> &mut ConcurrentState {
        self.task_state.concurrent_state_mut()
    }

    /// Lifts an `own` resource from the guest at the `idx` specified into its
    /// representation.
    ///
    /// Additionally returns a destructor/instance flags to go along with the
    /// representation so the host knows how to destroy this resource.
    pub fn guest_resource_lift_own(
        &mut self,
        ty: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<(u32, Option<NonNull<VMFuncRef>>, Option<RuntimeInstance>)> {
        let idx = self.resource_tables().guest_resource_lift_own(idx, ty)?;
        let (dtor, instance) = self.instance.dtor_and_instance(ty);
        Ok((idx, dtor, instance))
    }

    /// Lifts a `borrow` resource from the guest at the `idx` specified.
    pub fn guest_resource_lift_borrow(
        &mut self,
        ty: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<u32> {
        self.resource_tables().guest_resource_lift_borrow(idx, ty)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    pub fn host_resource_lower_own(
        &mut self,
        rep: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        instance: Option<RuntimeInstance>,
    ) -> Result<HostResourceIndex> {
        self.resource_tables()
            .host_resource_lower_own(rep, dtor, instance)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    pub fn host_resource_lower_borrow(&mut self, rep: u32) -> Result<HostResourceIndex> {
        self.resource_tables().host_resource_lower_borrow(rep)
    }

    /// Returns the underlying type of the resource table specified by `ty`.
    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        self.instance_type().resource_type(ty)
    }

    /// Returns instance type information for the component instance that is
    /// being lifted from.
    pub fn instance_type(&self) -> InstanceType<'_> {
        InstanceType::new(&self.instance)
    }

    fn resource_tables(&mut self) -> HostResourceTables<'_> {
        HostResourceTables::from_parts(
            ResourceTables {
                host_table: self.host_table,
                task_state: self.task_state,
                guest: Some(self.instance.as_mut().instance_states()),
            },
            self.host_resource_data,
        )
    }

    /// See [`HostResourceTables::validate_scope_exit`].
    #[inline]
    pub fn validate_scope_exit(&mut self) -> Result<()> {
        self.resource_tables().validate_scope_exit()
    }
}
