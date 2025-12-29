//! This module defines the `ResourceAny` type in the public API of Wasmtime,
//! which represents a dynamically typed resource handle that could either be
//! owned by the guest or the host.
//!
//! This is in contrast with `Resource<T>`, for example, and `ResourceAny` has
//! more "state" behind it. Specifically a `ResourceAny` has a type and a
//! `HostResourceIndex` which points inside of a `HostResourceData` structure
//! inside of a store. The `ResourceAny::resource_drop` method, or a conversion
//! to `Resource<T>`, is required to be called to avoid leaking data within a
//! store.

use crate::component::func::{LiftContext, LowerContext, bad_type_info, desc};
use crate::component::matching::InstanceType;
use crate::component::resources::host::{HostResource, HostResourceType};
use crate::component::resources::{HostResourceIndex, HostResourceTables};
use crate::component::{ComponentType, Lift, Lower, Resource, ResourceDynamic, ResourceType};
use crate::prelude::*;
use crate::runtime::vm::ValRaw;
use crate::{AsContextMut, StoreContextMut, Trap};
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

/// Representation of a resource in the component model, either a guest-defined
/// or a host-defined resource.
///
/// This type is similar to [`Resource`] except that it can be used to represent
/// any resource, either host or guest. This type cannot be directly constructed
/// and is only available if the guest returns it to the host (e.g. a function
/// returning a guest-defined resource) or by a conversion from [`Resource`] via
/// [`ResourceAny::try_from_resource`].
/// This type also does not carry a static type parameter `T` for example and
/// does not have as much information about its type.
/// This means that it's possible to get runtime type-errors when
/// using this type because it cannot statically prevent mismatching resource
/// types.
///
/// Like [`Resource`] this type represents either an `own` or a `borrow`
/// resource internally. Unlike [`Resource`], however, a [`ResourceAny`] must
/// always be explicitly destroyed with the [`ResourceAny::resource_drop`]
/// method. This will update internal dynamic state tracking and invoke the
/// WebAssembly-defined destructor for a resource, if any.
///
/// Note that it is required to call `resource_drop` for all instances of
/// [`ResourceAny`]: even borrows. Both borrows and own handles have state
/// associated with them that must be discarded by the time they're done being
/// used.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ResourceAny {
    idx: HostResourceIndex,
    ty: ResourceType,
    owned: bool,
}

impl ResourceAny {
    pub(crate) fn new(idx: HostResourceIndex, ty: ResourceType, owned: bool) -> ResourceAny {
        ResourceAny { idx, ty, owned }
    }

    /// Attempts to convert an imported [`Resource`] into [`ResourceAny`].
    ///
    /// * `resource` is the resource to convert.
    /// * `store` is the store to place the returned resource into.
    ///
    /// The returned `ResourceAny` will not have a destructor attached to it
    /// meaning that if `resource_drop` is called then it will not invoked a
    /// host-defined destructor. This is similar to how `Resource<T>` does not
    /// have a destructor associated with it.
    ///
    /// # Errors
    ///
    /// This method will return an error if `resource` has already been "taken"
    /// and has ownership transferred elsewhere which can happen in situations
    /// such as when it's already lowered into a component.
    pub fn try_from_resource<T: 'static>(
        resource: Resource<T>,
        store: impl AsContextMut,
    ) -> Result<Self> {
        resource.try_into_resource_any(store)
    }

    /// See [`Resource::try_from_resource_any`]
    pub fn try_into_resource<T: 'static>(self, store: impl AsContextMut) -> Result<Resource<T>> {
        Resource::try_from_resource_any(self, store)
    }

    /// See [`ResourceDynamic::try_from_resource_any`]
    pub fn try_into_resource_dynamic(self, store: impl AsContextMut) -> Result<ResourceDynamic> {
        ResourceDynamic::try_from_resource_any(self, store)
    }

    /// See [`Resource::try_from_resource_any`]
    pub(crate) fn try_into_host_resource<T, D>(
        self,
        mut store: impl AsContextMut,
    ) -> Result<HostResource<T, D>>
    where
        T: HostResourceType<D>,
        D: PartialEq + Send + Sync + Copy + 'static,
    {
        let store = store.as_context_mut();
        let mut tables = HostResourceTables::new_host(store.0);
        let ResourceAny { idx, ty, owned } = self;
        let ty = T::typecheck(ty).ok_or_else(|| crate::format_err!("resource type mismatch"))?;
        if owned {
            let rep = tables.host_resource_lift_own(idx)?;
            Ok(HostResource::new_own(rep, ty))
        } else {
            // For borrowed handles, first acquire the `rep` via lifting the
            // borrow. Afterwards though remove any dynamic state associated
            // with this borrow. `Resource<T>` doesn't participate in dynamic
            // state tracking and it's assumed embedders know what they're
            // doing, so the drop call will clear out that a borrow is active
            //
            // Note that the result of `drop` should always be `None` as it's a
            // borrowed handle, so assert so.
            let rep = tables.host_resource_lift_borrow(idx)?;
            let res = tables.host_resource_drop(idx)?;
            assert!(res.is_none());
            Ok(HostResource::new_borrow(rep, ty))
        }
    }

    /// Returns the corresponding type associated with this resource, either a
    /// host-defined type or a guest-defined type.
    ///
    /// This can be compared against [`ResourceType::host`] for example to see
    /// if it's a host-resource or against a type extracted with
    /// [`Instance::get_resource`] to see if it's a guest-defined resource.
    ///
    /// [`Instance::get_resource`]: crate::component::Instance::get_resource
    pub fn ty(&self) -> ResourceType {
        self.ty
    }

    /// Returns whether this is an owned resource, and if not it's a borrowed
    /// resource.
    pub fn owned(&self) -> bool {
        self.owned
    }

    /// Destroy this resource and release any state associated with it.
    ///
    /// This is required to be called (or the async version) for all instances
    /// of [`ResourceAny`] to ensure that state associated with this resource is
    /// properly cleaned up. For owned resources this may execute the
    /// guest-defined destructor if applicable (or the host-defined destructor
    /// if one was specified).
    pub fn resource_drop(self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `resource_drop_async` when async support is enabled on the config"
        );
        self.resource_drop_impl(&mut store.as_context_mut())
    }

    /// Same as [`ResourceAny::resource_drop`] except for use with async stores
    /// to execute the destructor asynchronously.
    #[cfg(feature = "async")]
    pub async fn resource_drop_async(self, mut store: impl AsContextMut<Data: Send>) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `resource_drop_async` without enabling async support in the config"
        );
        store
            .on_fiber(|store| self.resource_drop_impl(store))
            .await?
    }

    fn resource_drop_impl<T: 'static>(self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        // Attempt to remove `self.idx` from the host table in `store`.
        //
        // This could fail if the index is invalid or if this is removing an
        // `Own` entry which is currently being borrowed.
        let pair = HostResourceTables::new_host(store.0).host_resource_drop(self.idx)?;

        let (rep, slot) = match (pair, self.owned) {
            (Some(pair), true) => pair,

            // A `borrow` was removed from the table and no further
            // destruction, e.g. the destructor, is required so we're done.
            (None, false) => return Ok(()),

            _ => unreachable!(),
        };

        // Implement the reentrance check required by the canonical ABI. Note
        // that this happens whether or not a destructor is present.
        //
        // Note that this should be safe because the raw pointer access in
        // `flags` is valid due to `store` being the owner of the flags and
        // flags are never destroyed within the store.
        if let Some(flags) = slot.flags {
            unsafe {
                if !flags.may_enter() {
                    bail!(Trap::CannotEnterComponent);
                }
            }
        }

        let dtor = match slot.dtor {
            Some(dtor) => dtor.as_non_null(),
            None => return Ok(()),
        };
        let mut args = [ValRaw::u32(rep)];

        // This should be safe because `dtor` has been checked to belong to the
        // `store` provided which means it's valid and still alive. Additionally
        // destructors have al been previously type-checked and are guaranteed
        // to take one i32 argument and return no results, so the parameters
        // here should be configured correctly.
        unsafe { crate::Func::call_unchecked_raw(store, dtor, NonNull::from(&mut args)) }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = cx.host_resource_lift_own(self.idx)?;
                cx.guest_resource_lower_own(t, rep)
            }
            InterfaceType::Borrow(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = cx.host_resource_lift_borrow(self.idx)?;
                cx.guest_resource_lower_borrow(t, rep)
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Own(t) => {
                let ty = cx.resource_type(t);
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                let idx = cx.host_resource_lower_own(rep, dtor, flags)?;
                Ok(ResourceAny {
                    idx,
                    ty,
                    owned: true,
                })
            }
            InterfaceType::Borrow(t) => {
                let ty = cx.resource_type(t);
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                let idx = cx.host_resource_lower_borrow(rep)?;
                Ok(ResourceAny {
                    idx,
                    ty,
                    owned: false,
                })
            }
            _ => bad_type_info(),
        }
    }
}

unsafe impl ComponentType for ResourceAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Own(_) | InterfaceType::Borrow(_) => Ok(()),
            other => bail!("expected `own` or `borrow`, found `{}`", desc(other)),
        }
    }
}

unsafe impl Lower for ResourceAny {
    fn linear_lower_to_flat<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .linear_lower_to_flat(cx, InterfaceType::U32, dst)
    }

    fn linear_lower_to_memory<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .linear_lower_to_memory(cx, InterfaceType::U32, offset)
    }
}

unsafe impl Lift for ResourceAny {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let index = u32::linear_lift_from_flat(cx, InterfaceType::U32, src)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let index = u32::linear_lift_from_memory(cx, InterfaceType::U32, bytes)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }
}
