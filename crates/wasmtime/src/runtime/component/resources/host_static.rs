//! This module defines the `Resource<T>` type in the public API of Wasmtime.
//!
//! The purpose of this type is to represent a typed resource on the host where
//! the runtime representation is just a 32-bit integer plus some minor state
//! tracking. Notably the `T` enables statically differentiating resources from
//! one another and enables up-front type-checking where the lift/lower
//! operations need not do any type-checking at all.
//!
//! The actual `T` type parameter is just a guide, no `T` value is ever needed.

use crate::{AsContextMut, StoreContextMut};
use crate::component::func::{LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::resources::host::{HostResource, HostResourceType};
use crate::component::{ComponentType, Lift, Lower, ResourceAny, ResourceType, Val};
use crate::prelude::*;
use core::fmt;
use core::mem::MaybeUninit;
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

/// A host-defined resource in the component model with a statically ascribed
/// type parameter.
///
/// This type can be thought of as roughly a newtype wrapper around `u32` for
/// use as a resource with the component model. The main guarantee that the
/// component model provides is that the `u32` is not forgeable by guests and
/// there are guaranteed semantics about when a `u32` may be in use by the guest
/// and when it's guaranteed no longer needed. This means that it is safe for
/// embedders to consider the internal `u32` representation "trusted" and use it
/// for things like table indices with infallible accessors that panic on
/// out-of-bounds. This should only panic for embedder bugs, not because of any
/// possible behavior in the guest.
///
/// A `Resource<T>` value dynamically represents both an `(own $t)` in the
/// component model as well as a `(borrow $t)`. It can be inspected via
/// [`Resource::owned`] to test whether it is an owned handle. An owned handle
/// which is not actively borrowed can be destroyed at any time as it's
/// guaranteed that the guest does not have access to it. A borrowed handle, on
/// the other hand, may be accessed by the guest so it's not necessarily
/// guaranteed to be able to be destroyed.
///
/// Note that the "own" and "borrow" here refer to the component model, not
/// Rust. The semantics of Rust ownership and borrowing are slightly different
/// than the component model's (but spiritually the same) in that more dynamic
/// tracking is employed as part of the component model. This means that it's
/// possible to get runtime errors when using a `Resource<T>`. For example it is
/// an error to call [`Resource::new_borrow`] and pass that to a component
/// function expecting `(own $t)` and this is not statically disallowed.
///
/// The [`Resource`] type implements both the [`Lift`] and [`Lower`] trait to be
/// used with typed functions in the component model or as part of aggregate
/// structures and datatypes.
///
/// Note that [`Resource`] and [`ResourceDynamic`] are similar and for more
/// discussion of choosing one over the other see the documentation on
/// [`ResourceDynamic`]
///
/// [`ResourceDynamic`]: crate::component::ResourceDynamic
///
/// # Destruction of a resource
///
/// Resources in the component model are optionally defined with a destructor,
/// but this host resource type does not specify a destructor. It is left up to
/// the embedder to be able to determine how best to a destroy a resource when
/// it is owned.
///
/// Note, though, that while [`Resource`] itself does not specify destructors
/// it's still possible to do so via the [`Linker::resource`] definition. When a
/// resource type is defined for a guest component a destructor can be specified
/// which can be used to hook into resource destruction triggered by the guest.
///
/// This means that there are two ways that resource destruction is handled:
///
/// * Host resources destroyed by the guest can hook into the
///   [`Linker::resource`] destructor closure to handle resource destruction.
///   This could, for example, remove table entries.
///
/// * Host resources owned by the host itself have no automatic means of
///   destruction. The host can make its own determination that its own resource
///   is not lent out to the guest and at that time choose to destroy or
///   deallocate it.
///
/// # Dynamic behavior of a resource
///
/// A host-defined [`Resource`] does not necessarily represent a static value.
/// Its internals may change throughout its usage to track the state associated
/// with the resource. The internal 32-bit host-defined representation never
/// changes, however.
///
/// For example if there's a component model function of the form:
///
/// ```wasm
/// (func (param "a" (borrow $t)) (param "b" (own $t)))
/// ```
///
/// Then that can be extracted in Rust with:
///
/// ```rust,ignore
/// let func = instance.get_typed_func::<(&Resource<T>, &Resource<T>), ()>(&mut store, "name")?;
/// ```
///
/// Here the exact same resource can be provided as both arguments but that is
/// not valid to do so because the same resource cannot be actively borrowed and
/// passed by-value as the second parameter at the same time. The internal state
/// in `Resource<T>` will track this information and provide a dynamic runtime
/// error in this situation.
///
/// Mostly it's important to be aware that there is dynamic state associated
/// with a [`Resource<T>`] to provide errors in situations that cannot be
/// statically ruled out.
///
/// # Borrows and host responsibilities
///
/// Borrows to resources in the component model are guaranteed to be transient
/// such that if a borrow is passed as part of a function call then when the
/// function returns it's guaranteed that the guest no longer has access to the
/// resource. This guarantee, however, must be manually upheld by the host when
/// it receives its own borrow.
///
/// As mentioned above the [`Resource<T>`] type can represent a borrowed value
/// in addition to an owned value. This means a guest can provide the host with
/// a borrow, such as an argument to an imported function:
///
/// ```rust,ignore
/// linker.root().func_wrap("name", |_cx, (r,): (Resource<MyType>,)| {
///     assert!(!r.owned());
///     // .. here `r` is a borrowed value provided by the guest and the host
///     // shouldn't continue to access it beyond the scope of this call
/// })?;
/// ```
///
/// In this situation the host should take care to not attempt to persist the
/// resource beyond the scope of the call. It's the host's resource so it
/// technically can do what it wants with it but nothing is statically
/// preventing `r` to stay pinned to the lifetime of the closure invocation.
/// It's considered a mistake that the host performed if `r` is persisted too
/// long and accessed at the wrong time.
///
/// [`Linker::resource`]: crate::component::LinkerInstance::resource
pub struct Resource<T: 'static>(HostResource<Static<T>, ()>);

struct Static<T>(T);

impl<T: 'static> HostResourceType<()> for Static<T> {
    fn resource_type((): ()) -> ResourceType {
        ResourceType::host::<T>()
    }

    fn typecheck(ty: ResourceType) -> Option<()> {
        if ty.is_host::<T>() { Some(()) } else { None }
    }
}

impl<T> Resource<T>
where
    T: 'static,
{
    /// Creates a new owned resource with the `rep` specified.
    ///
    /// The returned value is suitable for passing to a guest as either a
    /// `(borrow $t)` or `(own $t)`.
    pub fn new_own(rep: u32) -> Resource<T> {
        Resource(HostResource::new_own(rep, ()))
    }

    /// Creates a new borrowed resource which isn't actually rooted in any
    /// ownership.
    ///
    /// This can be used to pass to a guest as a borrowed resource and the
    /// embedder will know that the `rep` won't be in use by the guest
    /// afterwards. Exactly how the lifetime of `rep` works is up to the
    /// embedder.
    pub fn new_borrow(rep: u32) -> Resource<T> {
        Resource(HostResource::new_borrow(rep, ()))
    }

    /// Returns the underlying 32-bit representation used to originally create
    /// this resource.
    pub fn rep(&self) -> u32 {
        self.0.rep()
    }

    /// Returns whether this is an owned resource or not.
    ///
    /// Owned resources can be safely destroyed by the embedder at any time, and
    /// borrowed resources have an owner somewhere else on the stack so can only
    /// be accessed, not destroyed.
    pub fn owned(&self) -> bool {
        self.0.owned()
    }

    /// Attempts to convert a [`ResourceAny`] into [`Resource`].
    ///
    /// This method will check that `resource` has type
    /// `ResourceType::host::<T>()` and then convert it into a typed version of
    /// the resource.
    ///
    /// # Errors
    ///
    /// This function will return an error if `resource` does not have type
    /// `ResourceType::host::<T>()`. This function may also return an error if
    /// `resource` is no longer valid, for example it was previously converted.
    ///
    /// # Panics
    ///
    /// This function will panic if `resource` does not belong to the `store`
    /// specified.
    pub fn try_from_resource_any(resource: ResourceAny, store: impl AsContextMut) -> Result<Self> {
        Ok(Resource(resource.try_into_host_resource(store)?))
    }

    /// See [`ResourceAny::try_from_resource`]
    pub fn try_into_resource_any(self, store: impl AsContextMut) -> Result<ResourceAny> {
        self.0.try_into_resource_any(store)
    }
}

unsafe impl<T: 'static> ComponentType for Resource<T> {
    const ABI: CanonicalAbiInfo = HostResource::<Static<T>, ()>::ABI;
    type Lower = crate::ValRaw;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        HostResource::<Static<T>, ()>::typecheck(ty, types)
    }

    fn to_val<S>(&self, store: StoreContextMut<S>) -> Result<Val> {
        Ok(Val::Resource(self.0.try_as_resource_any(store)?))
    }
}

unsafe impl<T: 'static> Lower for Resource<T> {
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

unsafe impl<T: 'static> Lift for Resource<T> {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let host_resource = HostResource::linear_lift_from_flat(cx, ty, src)?;
        Ok(Resource(host_resource))
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let host_resource = HostResource::linear_lift_from_memory(cx, ty, bytes)?;
        Ok(Resource(host_resource))
    }
}

impl<T> fmt::Debug for Resource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
