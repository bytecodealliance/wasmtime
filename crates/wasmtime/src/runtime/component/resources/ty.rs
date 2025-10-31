//! This module defines the `ResourceType` type in the public API of Wasmtime,
//! which is all possible types of resources.

use crate::runtime::vm::component::ComponentInstance;
use crate::store::StoreId;
use core::any::TypeId;
use wasmtime_environ::component::{
    AbstractResourceIndex, ComponentTypes, DefinedResourceIndex, ResourceIndex,
};

/// Representation of a resource type in the component model.
///
/// Resources are currently always represented as 32-bit integers but they have
/// unique types across instantiations and the host. For example instantiating
/// the same component twice means that defined resource types in the component
/// will all be different. Values of this type can be compared to see if
/// resources have the same type.
///
/// Resource types can also be defined on the host in addition to guests. On the
/// host resource types are tied to a `T`, an arbitrary Rust type. Two host
/// resource types are the same if they point to the same `T`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResourceType {
    kind: ResourceTypeKind,
}

impl ResourceType {
    /// Creates a new host resource type corresponding to `T`.
    ///
    /// Note that `T` is a mostly a phantom type parameter here. It does not
    /// need to reflect the actual storage of the resource `T`. For example this
    /// is valid:
    ///
    /// ```rust
    /// use wasmtime::component::ResourceType;
    ///
    /// struct Foo;
    ///
    /// let ty = ResourceType::host::<Foo>();
    /// ```
    ///
    /// A resource type of type `ResourceType::host::<T>()` will match the type
    /// of the value produced by `Resource::<T>::new_{own,borrow}`.
    pub fn host<T: 'static>() -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Host(TypeId::of::<T>()),
        }
    }

    /// Creates a new host resource type which is identified by the `payload`
    /// runtime argument.
    ///
    /// The `payload` argument to this function is an arbitrary 32-bit value
    /// that the host can use to distinguish one resource from another.
    /// A resource type of type `ResourceType::host_dynamic(2)` will match the
    /// type of the value produced by `ResourceDynamic::new_{own,borrow}(_, 2)`,
    /// for example.
    ///
    /// This type of resource is disjoint from all other types of resources. For
    /// example any resource with type `ResourceType::host::<u32>()` will be a
    /// different type than all types created by this function.
    pub fn host_dynamic(payload: u32) -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::HostDynamic(payload),
        }
    }

    pub(crate) fn guest(
        store: StoreId,
        instance: &ComponentInstance,
        id: DefinedResourceIndex,
    ) -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Guest {
                store,
                instance: instance as *const _ as usize,
                id,
            },
        }
    }

    pub(crate) fn uninstantiated(types: &ComponentTypes, index: ResourceIndex) -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Uninstantiated {
                component: types as *const _ as usize,
                index,
            },
        }
    }

    pub(crate) fn abstract_(types: &ComponentTypes, index: AbstractResourceIndex) -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Abstract {
                component: types as *const _ as usize,
                index,
            },
        }
    }

    pub(crate) fn is_host<T: 'static>(&self) -> bool {
        match self.kind {
            ResourceTypeKind::Host(id) if id == TypeId::of::<T>() => true,
            _ => false,
        }
    }

    pub(crate) fn as_host_dynamic(&self) -> Option<u32> {
        match self.kind {
            ResourceTypeKind::HostDynamic(payload) => Some(payload),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ResourceTypeKind {
    Host(TypeId),
    HostDynamic(u32),
    Guest {
        store: StoreId,
        // For now this is the `*mut ComponentInstance` pointer within the store
        // that this guest corresponds to. It's used to distinguish different
        // instantiations of the same component within the store.
        instance: usize,
        id: DefinedResourceIndex,
    },
    Uninstantiated {
        // Like `instance` in `Guest` above this is a pointer and is used to
        // distinguish between two components. Technically susceptible to ABA
        // issues but the consequence is a nonexistent resource would be equal
        // to a new resource so there's not really any issue with that.
        component: usize,
        index: ResourceIndex,
    },
    /// The type of this resource is considered "abstract" meaning that it
    /// doesn't actually correspond to anything at runtime but instead it just
    /// needs to be kept distinct from everything but itself.
    Abstract {
        component: usize,
        index: AbstractResourceIndex,
    },
}
