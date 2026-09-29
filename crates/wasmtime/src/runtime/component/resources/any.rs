//! This module defines the `ResourceAny` type in the public API of Wasmtime,
//! which represents a dynamically typed resource handle that could either be
//! owned by the guest or the host.
//!
//! This is in contrast with `Resource<T>`, for example, and `ResourceAny` has
//! more "state" behind it. Most `ResourceAny` values have a type and a
//! `HostResourceIndex` which points inside of a store. These must be dropped
//! or converted to a typed resource to release that state. A synthetic borrow
//! converted from `Resource::new_borrow` instead holds its representation
//! directly and has no host table entry.

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum ResourceAnyIndex {
    Table(HostResourceIndex),
    Borrow(u32),
}

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
/// resource internally. A [`ResourceAny`] with a host table entry must be
/// explicitly destroyed with [`ResourceAny::resource_drop`] (or converted to
/// a typed resource). This updates dynamic state tracking and invokes the
/// WebAssembly-defined destructor for a resource, if any.
///
/// Borrows lifted from a component have host table state and must be dropped.
/// Synthetic borrows converted from [`Resource::new_borrow`] have no host table
/// state; calling `resource_drop` on one is harmless but unnecessary.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ResourceAny {
    idx: ResourceAnyIndex,
    ty: ResourceType,
    owned: bool,
}

impl ResourceAny {
    pub(crate) fn new(idx: HostResourceIndex, ty: ResourceType, owned: bool) -> ResourceAny {
        ResourceAny {
            idx: ResourceAnyIndex::Table(idx),
            ty,
            owned,
        }
    }

    pub(crate) fn new_borrow(rep: u32, ty: ResourceType) -> ResourceAny {
        ResourceAny {
            idx: ResourceAnyIndex::Borrow(rep),
            ty,
            owned: false,
        }
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
    ///
    /// This function will return an [`OutOfMemory`][crate::OutOfMemory] error when
    /// memory allocation fails. See the `OutOfMemory` type's documentation for
    /// details on Wasmtime's out-of-memory handling.
    pub fn try_from_resource<T: 'static>(
        resource: Resource<T>,
        store: impl AsContextMut,
    ) -> Result<Self> {
        resource.try_into_resource_any(store)
    }

    /// See [`Resource::try_from_resource_any`]
    ///
    /// # Errors
    ///
    /// This function will return an [`OutOfMemory`][crate::OutOfMemory] error when
    /// memory allocation fails. See the `OutOfMemory` type's documentation for
    /// details on Wasmtime's out-of-memory handling.
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
        let ResourceAny { idx, ty, owned } = self;
        let ty = T::typecheck(ty).ok_or_else(|| crate::format_err!("resource type mismatch"))?;
        match idx {
            ResourceAnyIndex::Borrow(rep) => {
                assert!(!owned);
                Ok(HostResource::new_borrow(rep, ty))
            }
            ResourceAnyIndex::Table(idx) => {
                let store = store.as_context_mut();
                let mut tables = HostResourceTables::new_host(store.0)?;
                if owned {
                    let rep = tables.host_resource_lift_own(idx)?;
                    Ok(HostResource::new_own(rep, ty))
                } else {
                    // Typed borrows have no dynamic state. Remove the table
                    // entry after lifting its representation.
                    let rep = tables.host_resource_lift_borrow(idx)?;
                    let res = tables.host_resource_drop(idx)?;
                    assert!(res.is_none());
                    Ok(HostResource::new_borrow(rep, ty))
                }
            }
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
    /// This is required for resources with host table state. For synthetic
    /// borrows converted from [`Resource::new_borrow`] it has no effect.
    /// For owned resources this may execute the guest-defined destructor if
    /// applicable (or the host-defined destructor if one was specified).
    ///
    /// Exactly one of the following must be called for each [`ResourceAny`],
    /// depending on how the store is being driven:
    ///
    /// * [`ResourceAny::resource_drop`] for synchronous stores.
    /// * `ResourceAny::resource_drop_async` for [async](crate#async) stores
    ///   when a `StoreContextMut` is available.
    /// * `ResourceAny::resource_drop_concurrent` when only an `Accessor` is
    ///   available, such as inside `Store::run_concurrent` or an
    ///   `AccessorTask`.
    ///
    /// # Errors
    ///
    /// This function will return an [`OutOfMemory`][crate::OutOfMemory] error when
    /// memory allocation fails. See the `OutOfMemory` type's documentation for
    /// details on Wasmtime's out-of-memory handling.
    pub fn resource_drop(self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        store.0.validate_sync_call()?;
        self.resource_drop_impl(&mut store)
    }

    /// Same as [`ResourceAny::resource_drop`] except for use with async stores
    /// to execute the destructor [asynchronously](crate#async).
    ///
    /// # Errors
    ///
    /// This function will return an [`OutOfMemory`][crate::OutOfMemory] error when
    /// memory allocation fails. See the `OutOfMemory` type's documentation for
    /// details on Wasmtime's out-of-memory handling.
    #[cfg(feature = "async")]
    pub async fn resource_drop_async(self, mut store: impl AsContextMut<Data: Send>) -> Result<()> {
        let mut store = store.as_context_mut();
        store
            .on_fiber(|store| self.resource_drop_impl(store))
            .await?
    }

    /// Same as [`ResourceAny::resource_drop`] except for use with an
    /// [`Accessor`](crate::component::Accessor) while a store is executing
    /// [`Store::run_concurrent`](crate::Store::run_concurrent).
    ///
    /// The resource drop is queued for execution on the store's worker fiber.
    /// This method must be awaited while the store's concurrent event loop is
    /// running so the queued drop can make progress.
    ///
    /// Once this future has been polled and the drop has been queued, dropping
    /// the future does not cancel the resource drop.
    ///
    /// # Errors
    ///
    /// This function will return an [`OutOfMemory`][crate::OutOfMemory] error when
    /// memory allocation fails. See the `OutOfMemory` type's documentation for
    /// details on Wasmtime's out-of-memory handling.
    #[cfg(feature = "component-model-async")]
    pub async fn resource_drop_concurrent(
        self,
        accessor: impl crate::component::AsAccessor,
    ) -> Result<()> {
        let receiver = accessor.as_accessor().with(|mut store| -> Result<_> {
            let mut store = store.as_context_mut();
            let (sender, receiver) = futures::channel::oneshot::channel();
            let token = crate::store::StoreToken::new(store.as_context_mut());
            store.0.queue_task(move |store| {
                _ = sender.send(self.resource_drop_impl(&mut token.as_context_mut(store)));
                Ok(())
            })?;
            Ok(receiver)
        })?;
        receiver
            .await
            .map_err(|_| format_err!("resource drop task canceled"))?
    }

    fn resource_drop_impl<T: 'static>(self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        // Attempt to remove `self.idx` from the host table in `store`.
        //
        // This could fail if the index is invalid or if this is removing an
        // `Own` entry which is currently being borrowed.
        let idx = match self.idx {
            ResourceAnyIndex::Table(idx) => idx,
            ResourceAnyIndex::Borrow(_) => return Ok(()),
        };
        let pair = HostResourceTables::new_host(store.0)?.host_resource_drop(idx)?;

        let (rep, slot) = match (pair, self.owned) {
            (Some(pair), true) => pair,

            // A `borrow` was removed from the table and no further
            // destruction, e.g. the destructor, is required so we're done.
            (None, false) => return Ok(()),

            _ => unreachable!(),
        };

        if slot.instance.is_some() && !store.0.may_enter() {
            bail!(Trap::CannotEnterComponent);
        }

        let dtor = match slot.dtor {
            Some(dtor) => dtor.as_non_null(),
            None => return Ok(()),
        };
        let mut args = [ValRaw::u32(rep)];

        // Setup async-level task infrastructure for this call. This, for
        // example, prevents the destructor from blocking.
        //
        // Note that if `slot.instance` is `None` then this is skipped. That
        // means that this is a host resource being destroyed by the host. In
        // that case restrictions around blocking and such are exempt.
        if let Some(instance) = slot.instance {
            store.0.enter_guest_sync_call(false, instance)?;
        }

        // This should be safe because `dtor` has been checked to belong to the
        // `store` provided which means it's valid and still alive. Additionally
        // destructors have al been previously type-checked and are guaranteed
        // to take one i32 argument and return no results, so the parameters
        // here should be configured correctly.
        unsafe {
            crate::Func::call_unchecked_raw(store, dtor, NonNull::from(&mut args))?;
        }

        if slot.instance.is_some() {
            store.0.exit_guest_sync_call()?;
        }

        Ok(())
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = match self.idx {
                    ResourceAnyIndex::Table(idx) => cx.host_resource_lift_own(idx)?,
                    ResourceAnyIndex::Borrow(_) => {
                        bail!("cannot lower a `borrow` resource into an `own`")
                    }
                };
                cx.guest_resource_lower_own(t, rep)
            }
            InterfaceType::Borrow(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = match self.idx {
                    ResourceAnyIndex::Table(idx) => cx.host_resource_lift_borrow(idx)?,
                    ResourceAnyIndex::Borrow(rep) => rep,
                };
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
                Ok(ResourceAny::new(idx, ty, true))
            }
            InterfaceType::Borrow(t) => {
                let ty = cx.resource_type(t);
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                let idx = cx.host_resource_lower_borrow(rep)?;
                Ok(ResourceAny::new(idx, ty, false))
            }
            _ => bad_type_info(),
        }
    }
}

unsafe impl ComponentType for ResourceAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;
    const MAY_REQUIRE_REALLOC: bool = false;

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
