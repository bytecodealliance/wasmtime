use crate::prelude::*;
use crate::store::{StoreData, StoredData};

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
    instances => Option<Box<crate::component::instance::InstanceData>>,
}
