use crate::component::func::{LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::resources::host::{HostResource, HostResourceType};
use crate::component::{ComponentType, Lift, Lower, ResourceAny, ResourceType, Val};
use crate::prelude::*;
use crate::{AsContextMut, StoreContextMut};
use core::fmt;
use core::mem::MaybeUninit;
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

/// A host-defined resource in the component model with a dynamic runtime value
/// representing its type.
///
/// This type represents a host-owned resource in the same manner as
/// [`Resource`], and almost all of the documentation on that type is applicable
/// to usage of this type as well. Where the two differ is how embedders use
/// these types in a particular embedding.
///
/// # Use cases for [`Resource`]
///
/// The [`Resource`] type is intended to be used by Rust embedders and provides
/// a `T` type parameter to provide a layer of type safety when using the
/// resource. The type parameter prevents mixing up resources of the same type
/// by accident. The [`bindgen!`] macro, for example, uses [`Resource`] by
/// default to represent all resources imported by a wasm guest.
///
/// As the documentation on [`Resource`] indicates the `T` type parameter on
/// [`Resource<T>`] is a hint and `T` isn't stored inside. This means that the
/// host can write a function which takes [`Resource<T>`], for example, and
/// prevent mistakes of passing a wrong-typed-resource to that function.
/// Typically the 32-bit value that [`Resource<T>`] wraps is an index into some
/// sort of table the host manages and the index points to a value of type `T`.
/// The `T` type parameter can assist in writing helper functions to access
/// these types.
///
/// The downside of [`Resource`], however, is that all resource types must be
/// statically assigned at compile time. It's not possible to manufacture more
/// types at runtime in some more dynamic situations. That's where
/// [`ResourceDynamic`] comes in.
///
/// # Use cases for [`ResourceDynamic`]
///
/// The general idea of [`ResourceDynamic`] is very similar to [`Resource`] --
/// it represents a "trusted" 32-bit value that the host defines and assigns
/// meaning to. There is no destructor on [`ResourceDynamic`] and the host has
/// to know how to destroy the associated state, if any, that
/// [`ResourceDynamic`] references. The difference with [`Resource`] is that is
/// has runtime type information instead of static type information, meaning
/// that it's possible to mix these up at compile by accident.
///
/// However a [`ResourceDynamic`] can be constructed dynamically at runtime with
/// a runtime-defined type. For example an embedding that provides generic
/// access to types in the host may want to take advantage of the dynamic nature
/// of this type. Resources of type [`ResourceDynamic`] have a type of
/// [`ResourceType::host_dynamic(ty)`](ResourceType::host_dynamic) where `ty` is
/// the value provided to the constructors of [`ResourceDynamic`].
///
/// A [`ResourceDynamic`] implements [`Lift`] and [`Lower`] in the same manner
/// as [`Resource`], but the implementations may fail after type-checking unlike
/// with [`Resource`] (due to the dynamic nature of the type which can't be
/// fully-checked during type-checking).
///
/// [`Resource`]: crate::component::Resource
/// [`Resource<T>`]: crate::component::Resource
/// [`bindgen!`]: crate::component::bindgen
pub struct ResourceDynamic(HostResource<Dynamic, u32>);

struct Dynamic;

impl HostResourceType<u32> for Dynamic {
    fn resource_type(ty: u32) -> ResourceType {
        ResourceType::host_dynamic(ty)
    }

    fn typecheck(ty: ResourceType) -> Option<u32> {
        ty.as_host_dynamic()
    }
}

impl ResourceDynamic {
    /// Creates a new owned resource with the `rep` specified.
    ///
    /// This is the same as [`Resource::new_own`] except that `ty` is an extra
    /// parameter for the host-defined type information.
    ///
    /// [`Resource::new_own`]: crate::component::Resource::new_own
    pub fn new_own(rep: u32, ty: u32) -> ResourceDynamic {
        ResourceDynamic(HostResource::new_own(rep, ty))
    }

    /// Creates a new borrowed resource which isn't actually rooted in any
    /// ownership.
    ///
    /// This is the same as [`Resource::new_borrow`] except that `ty` is an extra
    /// parameter for the host-defined type information.
    ///
    /// [`Resource::new_borrow`]: crate::component::Resource::new_borrow
    pub fn new_borrow(rep: u32, ty: u32) -> ResourceDynamic {
        ResourceDynamic(HostResource::new_borrow(rep, ty))
    }

    /// Returns the underlying 32-bit representation used to originally create
    /// this resource.
    ///
    /// This is the same as [`Resource::rep`].
    ///
    /// [`Resource::rep`]: crate::component::Resource::rep
    pub fn rep(&self) -> u32 {
        self.0.rep()
    }

    /// Returns the 32-bit integer indicating the type of this resource.
    ///
    /// This will return the same 32-bit integer provided to the
    /// [`ResourceDynamic::new_own`] constructor. The meaning of this integer is
    /// left to the host and this only serves as an accessor to provide the
    /// value back to the host.
    pub fn ty(&self) -> u32 {
        self.0.ty()
    }

    /// Returns whether this is an owned resource or not.
    ///
    /// This is the same as [`Resource::owned`].
    ///
    /// [`Resource::owned`]: crate::component::Resource::owned
    pub fn owned(&self) -> bool {
        self.0.owned()
    }

    /// Attempts to convert a [`ResourceAny`] into [`ResourceDynamic`].
    ///
    /// This is the same as [`Resource::try_from_resource_any`].
    ///
    /// [`Resource::try_from_resource_any`]: crate::component::Resource::try_from_resource_any
    pub fn try_from_resource_any(resource: ResourceAny, store: impl AsContextMut) -> Result<Self> {
        Ok(ResourceDynamic(resource.try_into_host_resource(store)?))
    }

    /// See [`ResourceAny::try_from_resource`]
    pub fn try_into_resource_any(self, store: impl AsContextMut) -> Result<ResourceAny> {
        self.0.try_into_resource_any(store)
    }
}

unsafe impl ComponentType for ResourceDynamic {
    const ABI: CanonicalAbiInfo = HostResource::<Dynamic, u32>::ABI;
    type Lower = crate::ValRaw;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        HostResource::<Dynamic, u32>::typecheck(ty, types)
    }

    fn to_val<S>(&self, store: StoreContextMut<S>) -> Result<Val> {
        Ok(Val::Resource(self.0.try_as_resource_any(store)?))
    }
}

unsafe impl Lower for ResourceDynamic {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.0.linear_lower_to_flat(cx, ty, dst)
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.0.linear_lower_to_memory(cx, ty, offset)
    }
}

unsafe impl Lift for ResourceDynamic {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let host_resource = HostResource::linear_lift_from_flat(cx, ty, src)?;
        Ok(ResourceDynamic(host_resource))
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let host_resource = HostResource::linear_lift_from_memory(cx, ty, bytes)?;
        Ok(ResourceDynamic(host_resource))
    }
}

impl fmt::Debug for ResourceDynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
