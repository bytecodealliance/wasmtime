use crate::prelude::*;
use crate::runtime::component::concurrent::ConcurrentState;
use crate::runtime::component::{HostResourceData, Instance};
use crate::runtime::vm;
#[cfg(feature = "component-model-async")]
use crate::runtime::vm::VMStore;
use crate::runtime::vm::component::{
    CallContext, ComponentInstance, HandleTable, OwnedComponentInstance,
};
use crate::store::{StoreData, StoreId, StoreOpaque};
use crate::{Engine, StoreContextMut};
use core::pin::Pin;
use wasmtime_environ::PrimaryMap;
use wasmtime_environ::component::RuntimeComponentInstanceIndex;

/// Extensions to `Store` which are only relevant for component-related
/// information.
pub struct ComponentStoreData {
    /// All component instances, in a similar manner to how core wasm instances
    /// are managed.
    instances: PrimaryMap<ComponentInstanceId, Option<OwnedComponentInstance>>,

    /// Whether an instance belonging to this store has trapped.
    trapped: bool,

    /// Total number of component instances in this store, used to track
    /// resources in the instance allocator.
    num_component_instances: usize,

    /// Runtime state for components used in the handling of resources, borrow,
    /// and calls. These also interact with the `ResourceAny` type and its
    /// internal representation.
    component_host_table: HandleTable,
    host_resource_data: HostResourceData,

    /// Metadata/tasks/etc related to component-model-async and concurrency
    /// support.
    task_state: ComponentTaskState,
}

/// State tracking for tasks within components.
pub enum ComponentTaskState {
    /// Used when `Config::concurrency_support` is disabled. Here there are no
    /// async tasks but there's still state for borrows that needs managing.
    NotConcurrent(ComponentTasksNotConcurrent),

    /// Used when `Config::concurrency_support` is enabled and has
    /// full state for all async tasks.
    Concurrent(ConcurrentState),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComponentInstanceId(u32);
wasmtime_environ::entity_impl!(ComponentInstanceId);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RuntimeInstance {
    pub instance: ComponentInstanceId,
    pub index: RuntimeComponentInstanceIndex,
}

impl ComponentStoreData {
    pub fn new(engine: &Engine) -> ComponentStoreData {
        ComponentStoreData {
            instances: Default::default(),
            trapped: false,
            num_component_instances: 0,
            component_host_table: Default::default(),
            host_resource_data: Default::default(),
            task_state: if engine.tunables().concurrency_support {
                #[cfg(feature = "component-model-async")]
                {
                    ComponentTaskState::Concurrent(Default::default())
                }
                #[cfg(not(feature = "component-model-async"))]
                {
                    // This should be validated in `Config` where if
                    // `concurrency_support` is enabled but compile time support
                    // isn't available then an `Engine` isn't creatable.
                    unreachable!()
                }
            } else {
                ComponentTaskState::NotConcurrent(Default::default())
            },
        }
    }

    /// Hook used just before a `Store` is dropped to dispose of anything
    /// necessary.
    ///
    /// Used at this time to deallocate fibers related to concurrency support.
    pub fn run_manual_drop_routines<T>(store: StoreContextMut<T>) {
        // We need to drop the fibers of each component instance before
        // attempting to drop the instances themselves since the fibers may need
        // to be resumed and allowed to exit cleanly before we yank the state
        // out from under them.
        //
        // This will also drop any futures which might use a `&Accessor` fields
        // in their `Drop::drop` implementations, in which case they'll need to
        // be called from with in the context of a `tls::set` closure.
        #[cfg(feature = "component-model-async")]
        if store.0.component_data().task_state.is_concurrent() {
            ComponentStoreData::drop_fibers_and_futures(store.0);
        }
        #[cfg(not(feature = "component-model-async"))]
        let _ = store;
    }

    pub fn next_component_instance_id(&self) -> ComponentInstanceId {
        self.instances.next_key()
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn drop_fibers_and_futures(store: &mut dyn VMStore) {
        // Skip cleanup if concurrency support is not enabled.
        // This prevents panics when the component-model-async feature is compiled in
        // but the store was created without concurrency support (concurrent_state is None).
        if !store.concurrency_support() {
            return;
        }

        let mut fibers = Vec::new();
        let mut futures = Vec::new();
        store
            .concurrent_state_mut()
            .take_fibers_and_futures(&mut fibers, &mut futures);

        for mut fiber in fibers {
            fiber.dispose(store);
        }

        crate::component::concurrent::tls::set(store, move || drop(futures));
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn assert_instance_states_empty(&mut self) {
        for (_, instance) in self.instances.iter_mut() {
            let Some(instance) = instance.as_mut() else {
                continue;
            };

            assert!(
                instance
                    .get_mut()
                    .instance_states()
                    .0
                    .iter_mut()
                    .all(|(_, state)| state.handle_table().is_empty()
                        && state.concurrent_state().pending_is_empty())
            );
        }
    }

    pub fn decrement_allocator_resources(&mut self, allocator: &dyn vm::InstanceAllocator) {
        for _ in 0..self.num_component_instances {
            allocator.decrement_component_instance_count();
        }
    }
}

/// A type used to represent an allocated `ComponentInstance` located within a
/// store.
///
/// This type is held in various locations as a "safe index" into a store. This
/// encapsulates a `StoreId` which owns the instance as well as the index within
/// the store's list of which instance it's pointing to.
///
/// This type can notably be used to index into a `StoreOpaque` to project out
/// the `ComponentInstance` that is associated with this id.
#[repr(C)] // used by reference in the C API
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StoreComponentInstanceId {
    store_id: StoreId,
    instance: ComponentInstanceId,
}

impl StoreComponentInstanceId {
    pub(crate) fn new(
        store_id: StoreId,
        instance: ComponentInstanceId,
    ) -> StoreComponentInstanceId {
        StoreComponentInstanceId { store_id, instance }
    }

    #[inline]
    pub fn assert_belongs_to(&self, store: StoreId) {
        self.store_id.assert_belongs_to(store)
    }

    #[inline]
    pub(crate) fn store_id(&self) -> StoreId {
        self.store_id
    }

    #[inline]
    pub(crate) fn instance(&self) -> ComponentInstanceId {
        self.instance
    }

    /// Looks up the `vm::ComponentInstance` within `store` that this id points
    /// to.
    ///
    /// # Panics
    ///
    /// Panics if `self` does not belong to `store`.
    pub(crate) fn get<'a>(&self, store: &'a StoreOpaque) -> &'a ComponentInstance {
        self.assert_belongs_to(store.id());
        store.component_instance(self.instance)
    }

    /// Mutable version of `get` above.
    ///
    /// # Panics
    ///
    /// Panics if `self` does not belong to `store`.
    pub(crate) fn get_mut<'a>(&self, store: &'a mut StoreOpaque) -> Pin<&'a mut ComponentInstance> {
        self.from_data_get_mut(store.store_data_mut())
    }

    /// Return a mutable `ComponentInstance` and a `ModuleRegistry`
    /// from the store.
    ///
    /// # Panics
    ///
    /// Panics if `self` does not belong to `store`.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn get_mut_and_registry<'a>(
        &self,
        store: &'a mut StoreOpaque,
    ) -> (
        Pin<&'a mut ComponentInstance>,
        &'a crate::module::ModuleRegistry,
    ) {
        let (store_data, registry) = store.store_data_mut_and_registry();
        let instance = self.from_data_get_mut(store_data);
        (instance, registry)
    }

    /// Same as `get_mut`, but borrows less of a store.
    fn from_data_get_mut<'a>(&self, store: &'a mut StoreData) -> Pin<&'a mut ComponentInstance> {
        self.assert_belongs_to(store.id());
        store.component_instance_mut(self.instance)
    }
}

impl StoreData {
    pub(crate) fn push_component_instance(
        &mut self,
        data: OwnedComponentInstance,
    ) -> ComponentInstanceId {
        let expected = data.get().id();
        let ret = self.components.instances.push(Some(data));
        assert_eq!(expected, ret);
        ret
    }

    pub(crate) fn component_instance(&self, id: ComponentInstanceId) -> &ComponentInstance {
        self.components.instances[id].as_ref().unwrap().get()
    }

    pub(crate) fn component_instance_mut(
        &mut self,
        id: ComponentInstanceId,
    ) -> Pin<&mut ComponentInstance> {
        self.components.instances[id].as_mut().unwrap().get_mut()
    }
}

impl StoreOpaque {
    pub(crate) fn trapped(&self) -> bool {
        self.store_data().components.trapped
    }

    pub(crate) fn set_trapped(&mut self) {
        self.store_data_mut().components.trapped = true;
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn component_data(&self) -> &ComponentStoreData {
        &self.store_data().components
    }

    pub(crate) fn component_data_mut(&mut self) -> &mut ComponentStoreData {
        &mut self.store_data_mut().components
    }

    pub(crate) fn component_task_state_mut(&mut self) -> &mut ComponentTaskState {
        &mut self.component_data_mut().task_state
    }

    pub(crate) fn push_component_instance(&mut self, instance: Instance) {
        // We don't actually need the instance itself right now, but it seems
        // like something we will almost certainly eventually want to keep
        // around, so force callers to provide it.
        let _ = instance;

        self.component_data_mut().num_component_instances += 1;
    }

    pub(crate) fn component_instance(&self, id: ComponentInstanceId) -> &ComponentInstance {
        self.store_data().component_instance(id)
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn component_instance_mut(
        &mut self,
        id: ComponentInstanceId,
    ) -> Pin<&mut ComponentInstance> {
        self.store_data_mut().component_instance_mut(id)
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn concurrent_state_mut(&mut self) -> &mut ConcurrentState {
        debug_assert!(self.concurrency_support());
        self.component_data_mut().task_state.concurrent_state_mut()
    }

    #[inline]
    #[cfg(feature = "component-model-async")]
    pub(crate) fn concurrency_support(&self) -> bool {
        let support = self.component_data().task_state.is_concurrent();
        debug_assert_eq!(support, self.engine().tunables().concurrency_support);
        support
    }

    pub(crate) fn lift_context_parts(
        &mut self,
        instance: Instance,
    ) -> (
        &mut ComponentTaskState,
        &mut HandleTable,
        &mut HostResourceData,
        Pin<&mut ComponentInstance>,
    ) {
        let instance = instance.id();
        instance.assert_belongs_to(self.id());
        let data = self.component_data_mut();
        (
            &mut data.task_state,
            &mut data.component_host_table,
            &mut data.host_resource_data,
            data.instances[instance.instance]
                .as_mut()
                .unwrap()
                .get_mut(),
        )
    }

    pub(crate) fn component_resource_tables(
        &mut self,
        instance: Option<Instance>,
    ) -> vm::component::ResourceTables<'_> {
        self.component_resource_tables_and_host_resource_data(instance)
            .0
    }

    pub(crate) fn component_resource_tables_and_host_resource_data(
        &mut self,
        instance: Option<Instance>,
    ) -> (
        vm::component::ResourceTables<'_>,
        &mut crate::component::HostResourceData,
    ) {
        let store_id = self.id();
        let data = self.component_data_mut();
        let guest = instance.map(|i| {
            let i = i.id();
            i.assert_belongs_to(store_id);
            data.instances[i.instance]
                .as_mut()
                .unwrap()
                .get_mut()
                .instance_states()
        });

        (
            vm::component::ResourceTables {
                host_table: &mut data.component_host_table,
                task_state: &mut data.task_state,
                guest,
            },
            &mut data.host_resource_data,
        )
    }

    pub(crate) fn enter_call_not_concurrent(&mut self) {
        let state = match &mut self.component_data_mut().task_state {
            ComponentTaskState::NotConcurrent(state) => state,
            ComponentTaskState::Concurrent(_) => unreachable!(),
        };
        state.scopes.push(CallContext::default());
    }

    pub(crate) fn exit_call_not_concurrent(&mut self) {
        let state = match &mut self.component_data_mut().task_state {
            ComponentTaskState::NotConcurrent(state) => state,
            ComponentTaskState::Concurrent(_) => unreachable!(),
        };
        state.scopes.pop();
    }
}

#[derive(Default)]
pub struct ComponentTasksNotConcurrent {
    scopes: Vec<CallContext>,
}

impl ComponentTaskState {
    pub fn call_context(&mut self, id: u32) -> &mut CallContext {
        match self {
            ComponentTaskState::NotConcurrent(state) => &mut state.scopes[id as usize],
            ComponentTaskState::Concurrent(state) => state.call_context(id),
        }
    }

    pub fn current_call_context_scope_id(&self) -> u32 {
        match self {
            ComponentTaskState::NotConcurrent(state) => {
                u32::try_from(state.scopes.len() - 1).unwrap()
            }
            ComponentTaskState::Concurrent(state) => state.current_call_context_scope_id(),
        }
    }

    pub fn concurrent_state_mut(&mut self) -> &mut ConcurrentState {
        match self {
            ComponentTaskState::Concurrent(state) => state,
            ComponentTaskState::NotConcurrent(_) => {
                panic!("expected concurrent state to be present")
            }
        }
    }

    #[cfg(feature = "component-model-async")]
    fn is_concurrent(&self) -> bool {
        match self {
            ComponentTaskState::Concurrent(_) => true,
            ComponentTaskState::NotConcurrent(_) => false,
        }
    }
}
