//! This module defines the `Resource<T>` type in the public API of Wasmtime.
//!
//! The purpose of this type is to represent a typed resource on the host where
//! the runtime representation is just a 32-bit integer plus some minor state
//! tracking. Notably the `T` enables statically differentiating resources from
//! one another and enables up-front type-checking where the lift/lower
//! operations need not do any type-checking at all.
//!
//! The actual `T` type parameter is just a guide, no `T` value is ever needed.

use crate::AsContextMut;
use crate::component::func::{LiftContext, LowerContext, bad_type_info, desc};
use crate::component::matching::InstanceType;
use crate::component::resources::{HostResourceIndex, HostResourceTables};
use crate::component::{ComponentType, Lift, Lower, ResourceAny, ResourceType};
use crate::prelude::*;
use core::fmt;
use core::marker;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

/// A host-defined resource in the component model.
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
pub struct Resource<T> {
    /// The host-defined 32-bit representation of this resource.
    rep: u32,

    /// Dear rust please consider `T` used even though it's not actually used.
    _marker: marker::PhantomData<fn() -> T>,

    state: AtomicResourceState,
}

/// Internal dynamic state tracking for this resource. This can be one of
/// four different states:
///
/// * `BORROW` / `u64::MAX` - this indicates that this is a borrowed
///   resource. The `rep` doesn't live in the host table and this `Resource`
///   instance is transiently available. It's the host's responsibility to
///   discard this resource when the borrow duration has finished.
///
/// * `NOT_IN_TABLE` / `u64::MAX - 1` - this indicates that this is an owned
///   resource not present in any store's table. This resource is not lent
///   out. It can be passed as an `(own $t)` directly into a guest's table
///   or it can be passed as a borrow to a guest which will insert it into
///   a host store's table for dynamic borrow tracking.
///
/// * `TAKEN` / `u64::MAX - 2` - while the `rep` is available the resource
///   has been dynamically moved into a guest and cannot be moved in again.
///   This is used for example to prevent the same resource from being
///   passed twice to a guest.
///
/// * All other values - any other value indicates that the value is an
///   index into a store's table of host resources. It's guaranteed that the
///   table entry represents a host resource and the resource may have
///   borrow tracking associated with it. The low 32-bits of the value are
///   the table index and the upper 32-bits are the generation.
///
/// Note that this is two `AtomicU32` fields but it's not intended to actually
/// be used in conjunction with threads as generally a `Store<T>` lives on one
/// thread at a time. The pair of `AtomicU32` here is used to ensure that this
/// type is `Send + Sync` when captured as a reference to make async
/// programming more ergonomic.
///
/// Also note that two `AtomicU32` here are used instead of `AtomicU64` to be
/// more portable to platforms without 64-bit atomics.
struct AtomicResourceState {
    index: AtomicU32,
    generation: AtomicU32,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum ResourceState {
    Borrow,
    NotInTable,
    Taken,
    Index(HostResourceIndex),
}

impl AtomicResourceState {
    const BORROW: Self = AtomicResourceState::new(ResourceState::Borrow);
    const NOT_IN_TABLE: Self = AtomicResourceState::new(ResourceState::NotInTable);

    const fn new(state: ResourceState) -> AtomicResourceState {
        let (index, generation) = state.encode();
        Self {
            index: AtomicU32::new(index),
            generation: AtomicU32::new(generation),
        }
    }

    fn get(&self) -> ResourceState {
        ResourceState::decode(self.index.load(Relaxed), self.generation.load(Relaxed))
    }

    fn swap(&self, state: ResourceState) -> ResourceState {
        let (index, generation) = state.encode();
        let index_prev = self.index.load(Relaxed);
        self.index.store(index, Relaxed);
        let generation_prev = self.generation.load(Relaxed);
        self.generation.store(generation, Relaxed);
        ResourceState::decode(index_prev, generation_prev)
    }
}

impl ResourceState {
    // See comments on `state` above for info about these values.
    const BORROW: u32 = u32::MAX;
    const NOT_IN_TABLE: u32 = u32::MAX - 1;
    const TAKEN: u32 = u32::MAX - 2;

    fn decode(idx: u32, generation: u32) -> ResourceState {
        match generation {
            Self::BORROW => Self::Borrow,
            Self::NOT_IN_TABLE => Self::NotInTable,
            Self::TAKEN => Self::Taken,
            _ => Self::Index(HostResourceIndex::new(idx, generation)),
        }
    }

    const fn encode(&self) -> (u32, u32) {
        match self {
            Self::Borrow => (0, Self::BORROW),
            Self::NotInTable => (0, Self::NOT_IN_TABLE),
            Self::Taken => (0, Self::TAKEN),
            Self::Index(index) => (index.index(), index.generation()),
        }
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
        Resource {
            state: AtomicResourceState::NOT_IN_TABLE,
            rep,
            _marker: marker::PhantomData,
        }
    }

    /// Creates a new borrowed resource which isn't actually rooted in any
    /// ownership.
    ///
    /// This can be used to pass to a guest as a borrowed resource and the
    /// embedder will know that the `rep` won't be in use by the guest
    /// afterwards. Exactly how the lifetime of `rep` works is up to the
    /// embedder.
    pub fn new_borrow(rep: u32) -> Resource<T> {
        Resource {
            state: AtomicResourceState::BORROW,
            rep,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the underlying 32-bit representation used to originally create
    /// this resource.
    pub fn rep(&self) -> u32 {
        self.rep
    }

    /// Returns whether this is an owned resource or not.
    ///
    /// Owned resources can be safely destroyed by the embedder at any time, and
    /// borrowed resources have an owner somewhere else on the stack so can only
    /// be accessed, not destroyed.
    pub fn owned(&self) -> bool {
        match self.state.get() {
            ResourceState::Borrow => false,
            ResourceState::Taken | ResourceState::NotInTable | ResourceState::Index(_) => true,
        }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                let rep = match self.state.get() {
                    // If this is a borrow resource then this is a dynamic
                    // error on behalf of the embedder.
                    ResourceState::Borrow => {
                        bail!("cannot lower a `borrow` resource into an `own`")
                    }

                    // If this resource does not yet live in a table then we're
                    // dynamically transferring ownership to wasm. Record that
                    // it's no longer present and then pass through the
                    // representation.
                    ResourceState::NotInTable => {
                        let prev = self.state.swap(ResourceState::Taken);
                        assert_eq!(prev, ResourceState::NotInTable);
                        self.rep
                    }

                    // This resource has already been moved into wasm so this is
                    // a dynamic error on behalf of the embedder.
                    ResourceState::Taken => bail!("host resource already consumed"),

                    // If this resource lives in a host table then try to take
                    // it out of the table, which may fail, and on success we
                    // can move the rep into the guest table.
                    ResourceState::Index(idx) => cx.host_resource_lift_own(idx)?,
                };
                cx.guest_resource_lower_own(t, rep)
            }
            InterfaceType::Borrow(t) => {
                let rep = match self.state.get() {
                    // If this is already a borrowed resource, nothing else to
                    // do and the rep is passed through.
                    ResourceState::Borrow => self.rep,

                    // If this resource is already gone, that's a dynamic error
                    // for the embedder.
                    ResourceState::Taken => bail!("host resource already consumed"),

                    // If this resource is not currently in a table then it
                    // needs to move into a table to participate in state
                    // related to borrow tracking. Execute the
                    // `host_resource_lower_own` operation here and update our
                    // state.
                    //
                    // Afterwards this is the same as the `idx` case below.
                    //
                    // Note that flags/dtor are passed as `None` here since
                    // `Resource<T>` doesn't offer destruction support.
                    ResourceState::NotInTable => {
                        let idx = cx.host_resource_lower_own(self.rep, None, None)?;
                        let prev = self.state.swap(ResourceState::Index(idx));
                        assert_eq!(prev, ResourceState::NotInTable);
                        cx.host_resource_lift_borrow(idx)?
                    }

                    // If this resource lives in a table then it needs to come
                    // out of the table with borrow-tracking employed.
                    ResourceState::Index(idx) => cx.host_resource_lift_borrow(idx)?,
                };
                cx.guest_resource_lower_borrow(t, rep)
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let (state, rep) = match ty {
            // Ownership is being transferred from a guest to the host, so move
            // it from the guest table into a new `Resource`. Note that this
            // isn't immediately inserted into the host table and that's left
            // for the future if it's necessary to take a borrow from this owned
            // resource.
            InterfaceType::Own(t) => {
                debug_assert!(cx.resource_type(t) == ResourceType::host::<T>());
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                assert!(dtor.is_some());
                assert!(flags.is_none());
                (AtomicResourceState::NOT_IN_TABLE, rep)
            }

            // The borrow here is lifted from the guest, but note the lack of
            // `host_resource_lower_borrow` as it's intentional. Lowering
            // a borrow has a special case in the canonical ABI where if the
            // receiving module is the owner of the resource then it directly
            // receives the `rep` and no other dynamic tracking is employed.
            // This effectively mirrors that even though the canonical ABI
            // isn't really all that applicable in host context here.
            InterfaceType::Borrow(t) => {
                debug_assert!(cx.resource_type(t) == ResourceType::host::<T>());
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                (AtomicResourceState::BORROW, rep)
            }
            _ => bad_type_info(),
        };
        Ok(Resource {
            state,
            rep,
            _marker: marker::PhantomData,
        })
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
        resource.try_into_resource(store)
    }

    /// See [`ResourceAny::try_from_resource`]
    pub fn try_into_resource_any(self, mut store: impl AsContextMut) -> Result<ResourceAny> {
        let Resource { rep, state, .. } = self;
        let store = store.as_context_mut();

        let mut tables = HostResourceTables::new_host(store.0);
        let (idx, owned) = match state.get() {
            ResourceState::Borrow => (tables.host_resource_lower_borrow(rep)?, false),
            ResourceState::NotInTable => {
                let idx = tables.host_resource_lower_own(rep, None, None)?;
                (idx, true)
            }
            ResourceState::Taken => bail!("host resource already consumed"),
            ResourceState::Index(idx) => (idx, true),
        };
        Ok(ResourceAny::new(idx, ResourceType::host::<T>(), owned))
    }
}

unsafe impl<T: 'static> ComponentType for Resource<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        let resource = match ty {
            InterfaceType::Own(t) | InterfaceType::Borrow(t) => *t,
            other => bail!("expected `own` or `borrow`, found `{}`", desc(other)),
        };
        if !types.resource_type(resource).is_host::<T>() {
            bail!("resource type mismatch");
        }

        Ok(())
    }
}

unsafe impl<T: 'static> Lower for Resource<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .linear_lower_to_flat(cx, InterfaceType::U32, dst)
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .linear_lower_to_memory(cx, InterfaceType::U32, offset)
    }
}

unsafe impl<T: 'static> Lift for Resource<T> {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let index = u32::linear_lift_from_flat(cx, InterfaceType::U32, src)?;
        Resource::lift_from_index(cx, ty, index)
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let index = u32::linear_lift_from_memory(cx, InterfaceType::U32, bytes)?;
        Resource::lift_from_index(cx, ty, index)
    }
}

impl<T> fmt::Debug for Resource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.state.get() {
            ResourceState::Borrow => "borrow",
            ResourceState::NotInTable => "own (not in table)",
            ResourceState::Taken => "taken",
            ResourceState::Index(_) => "own",
        };
        f.debug_struct("Resource")
            .field("rep", &self.rep)
            .field("state", &state)
            .finish()
    }
}
