#[cfg(feature = "component-model-async")]
use crate::runtime::vm::VMStore;
use crate::runtime::vm::component::{ComponentInstance, OwnedComponentInstance};
use crate::store::{StoreData, StoreId, StoreOpaque};
#[cfg(feature = "component-model-async")]
use alloc::vec::Vec;
use core::pin::Pin;
use wasmtime_environ::PrimaryMap;
use wasmtime_environ::component::RuntimeComponentInstanceIndex;

#[derive(Default)]
pub struct ComponentStoreData {
    instances: PrimaryMap<ComponentInstanceId, Option<OwnedComponentInstance>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComponentInstanceId(u32);
wasmtime_environ::entity_impl!(ComponentInstanceId);

/// Represents a (`ComponentInstanceId`, `RuntimeComponentInstanceIndex`) pair.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RuntimeInstance {
    pub instance: ComponentInstanceId,
    pub index: RuntimeComponentInstanceIndex,
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
}

impl ComponentStoreData {
    pub fn next_component_instance_id(&self) -> ComponentInstanceId {
        self.instances.next_key()
    }

    #[cfg(feature = "component-model-async")]
    pub(crate) fn drop_fibers_and_futures(store: &mut dyn VMStore) {
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
}

impl StoreData {
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
    pub(crate) fn component_instance(&self, id: ComponentInstanceId) -> &ComponentInstance {
        self.store_data().component_instance(id)
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
    pub(crate) fn from_data_get_mut<'a>(
        &self,
        store: &'a mut StoreData,
    ) -> Pin<&'a mut ComponentInstance> {
        self.assert_belongs_to(store.id());
        store.component_instance_mut(self.instance)
    }
}
