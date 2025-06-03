use crate::prelude::*;
use crate::runtime::vm::component::{ComponentInstance, OwnedComponentInstance};
use crate::store::{StoreData, StoreId, StoreOpaque};
use core::ops::Index;
use core::ptr::NonNull;

#[derive(Default)]
pub struct ComponentStoreData {
    instances: Vec<Option<OwnedComponentInstance>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComponentInstanceId(usize);

impl ComponentInstanceId {
    pub fn from_index(idx: usize) -> ComponentInstanceId {
        ComponentInstanceId(idx)
    }

    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

impl ComponentStoreData {
    pub fn next_component_instance_id(&self) -> ComponentInstanceId {
        ComponentInstanceId(self.instances.len())
    }
}

impl StoreData {
    pub(crate) fn push_component_instance(
        &mut self,
        data: OwnedComponentInstance,
    ) -> ComponentInstanceId {
        let ret = self.components.next_component_instance_id();
        assert_eq!(data.id(), ret);
        self.components.instances.push(Some(data));
        ret
    }
}

impl StoreOpaque {
    pub(crate) fn component_instance(&self, id: ComponentInstanceId) -> &ComponentInstance {
        self.store_data().components.instances[id.0]
            .as_ref()
            .unwrap()
    }

    // FIXME: this method should not exist, future refactorings should delete it
    //
    // Specifically we're in the process of requiring that all APIs, even
    // libcalls and host functions, work with `&mut StoreThing` plus
    // `ComponentInstanceId` (or a store-tagged index). When doing so there
    // should be no need for raw pointers as access to a `ComponentInstance` is
    // 100% delegated through the store itself. Until that happens though this
    // will need to stick around as there's a few places that work with raw
    // pointers instead of safe pointers.
    pub(crate) fn component_instance_ptr(
        &self,
        id: ComponentInstanceId,
    ) -> NonNull<ComponentInstance> {
        self.store_data().components.instances[id.0]
            .as_ref()
            .unwrap()
            .instance_ptr()
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
    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    #[inline]
    pub(crate) fn instance(&self) -> ComponentInstanceId {
        self.instance
    }
}

impl Index<StoreComponentInstanceId> for StoreOpaque {
    type Output = ComponentInstance;

    fn index(&self, id: StoreComponentInstanceId) -> &Self::Output {
        id.assert_belongs_to(self.id());
        self.store_data().components.instances[id.instance.0]
            .as_ref()
            .unwrap()
    }
}
