use crate::prelude::*;
use crate::runtime::vm::component::{ComponentInstance, OwnedComponentInstance};
use crate::store::{StoreData, StoreOpaque, StoredData};
use core::mem;
use core::ptr::NonNull;

macro_rules! component_store_data {
    ($($field:ident => $t:ty,)*) => (
        #[derive(Default)]
        pub struct ComponentStoreData {
            $($field: Vec<$t>,)*

            instances: Vec<Option<OwnedComponentInstance>>,
        }

        $(
            impl StoredData for $t {
                #[inline]
                fn list(data: &StoreData) -> &Vec<Self> {
                    &data.components.$field
                }
                #[inline]
                fn list_mut(data: &mut StoreData) -> &mut Vec<Self> {
                    &mut data.components.$field
                }
            }
        )*
    )
}

component_store_data! {
    funcs => crate::component::func::FuncData,
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

    // FIXME: this method should not exist, future refactorings should delete it
    //
    // Specifically we're in the process of removing `Stored<T>` and the only
    // location this API is required is for functions right now when they're
    // being created. That needs to be refactored anyway and removing `Stored`
    // should remove the need for this function.
    pub(crate) unsafe fn component_instance_replace(
        &mut self,
        id: ComponentInstanceId,
        instance: Option<OwnedComponentInstance>,
    ) -> Option<OwnedComponentInstance> {
        mem::replace(
            &mut self.store_data_mut().components.instances[id.0],
            instance,
        )
    }
}
