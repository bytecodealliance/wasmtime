use crate::component::instance::InstanceData;
use crate::prelude::*;
use crate::runtime::vm::component::ComponentInstance;
use crate::store::{StoreData, StoreOpaque, Stored, StoredData};

macro_rules! component_store_data {
    ($($field:ident => $t:ty,)*) => (
        #[derive(Default)]
        pub struct ComponentStoreData {
            $($field: Vec<$t>,)*
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
    instances => Option<Box<InstanceData>>,
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
        data: Box<InstanceData>,
    ) -> Stored<Option<Box<InstanceData>>> {
        assert_eq!(
            data.instance().id(),
            self.components.next_component_instance_id()
        );
        self.insert(Some(data))
    }
}

impl StoreOpaque {
    pub(crate) fn component_instance(&self, id: ComponentInstanceId) -> &ComponentInstance {
        self.store_data().components.instances[id.0]
            .as_ref()
            .unwrap()
            .instance()
    }
}
