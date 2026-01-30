use crate::AsContextMut;
use crate::component::func::{LiftContext, LowerContext, bad_type_info, desc};
use crate::component::matching::InstanceType;
use crate::component::resources::{HostResourceIndex, HostResourceTables};
use crate::component::{ComponentType, Lift, Lower, ResourceAny, ResourceType, Val};
use crate::prelude::*;
use core::fmt;
use core::marker;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

/// Internal type in Wasmtime used to represent host-defined resources that are
/// not tracked within the store.
pub struct HostResource<T: HostResourceType<D>, D> {
    /// The host-defined 32-bit representation of this resource.
    rep: u32,

    /// Metadata, if necessary, tracking the type of this resource.
    ty: D,

    /// Internal state about borrows and such.
    state: AtomicResourceState,

    _marker: marker::PhantomData<fn() -> T>,
}

// FIXME(rust-lang/rust#110338) the `D` type parameter here should be an
// associated type. In a first attempt at doing that the `wasmtime-wasi-io`
// crate failed to compile with obscure errors. At least this is an internal
// trait for now...
pub trait HostResourceType<D> {
    /// Tests whether `ty` matches this resource type.
    fn typecheck(ty: ResourceType) -> Option<D>;
    /// Converts `Data` to a `ResourceType`.
    fn resource_type(data: D) -> ResourceType;
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

impl<T, D> HostResource<T, D>
where
    T: HostResourceType<D>,
    D: PartialEq + Send + Sync + Copy + 'static,
{
    pub fn new_own(rep: u32, ty: D) -> Self {
        HostResource {
            state: AtomicResourceState::NOT_IN_TABLE,
            rep,
            ty,
            _marker: marker::PhantomData,
        }
    }

    pub fn new_borrow(rep: u32, ty: D) -> Self {
        HostResource {
            state: AtomicResourceState::BORROW,
            rep,
            ty,
            _marker: marker::PhantomData,
        }
    }

    pub fn rep(&self) -> u32 {
        self.rep
    }

    pub fn ty(&self) -> D {
        self.ty
    }

    pub fn owned(&self) -> bool {
        match self.state.get() {
            ResourceState::Borrow => false,
            ResourceState::Taken | ResourceState::NotInTable | ResourceState::Index(_) => true,
        }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                match T::typecheck(cx.resource_type(t)) {
                    Some(t) if t == self.ty => {}
                    _ => bail!("resource type mismatch"),
                }
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
                match T::typecheck(cx.resource_type(t)) {
                    Some(t) if t == self.ty => {}
                    _ => bail!("resource type mismatch"),
                }
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
        let (state, rep, ty) = match ty {
            // Ownership is being transferred from a guest to the host, so move
            // it from the guest table into a new `Resource`. Note that this
            // isn't immediately inserted into the host table and that's left
            // for the future if it's necessary to take a borrow from this owned
            // resource.
            InterfaceType::Own(t) => {
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                assert!(dtor.is_some());
                assert!(flags.is_none());
                (AtomicResourceState::NOT_IN_TABLE, rep, t)
            }

            // The borrow here is lifted from the guest, but note the lack of
            // `host_resource_lower_borrow` as it's intentional. Lowering
            // a borrow has a special case in the canonical ABI where if the
            // receiving module is the owner of the resource then it directly
            // receives the `rep` and no other dynamic tracking is employed.
            // This effectively mirrors that even though the canonical ABI
            // isn't really all that applicable in host context here.
            InterfaceType::Borrow(t) => {
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                (AtomicResourceState::BORROW, rep, t)
            }
            _ => bad_type_info(),
        };
        let ty = T::typecheck(cx.resource_type(ty)).unwrap();
        Ok(HostResource {
            state,
            rep,
            ty,
            _marker: marker::PhantomData,
        })
    }

    pub(crate) fn try_as_resource_any(&self, mut store: impl AsContextMut) -> Result<ResourceAny> {
        let store = store.as_context_mut();

        let mut tables = HostResourceTables::new_host(store.0);
        let (idx, owned) = match self.state.get() {
            ResourceState::Borrow => (tables.host_resource_lower_borrow(self.rep)?, false),
            ResourceState::NotInTable => {
                let idx = tables.host_resource_lower_own(self.rep, None, None)?;
                (idx, true)
            }
            ResourceState::Taken => bail!("host resource already consumed"),
            ResourceState::Index(idx) => (idx, true),
        };
        Ok(ResourceAny::new(idx, T::resource_type(self.ty), owned))
    }

    pub fn try_into_resource_any(self, store: impl AsContextMut) -> Result<ResourceAny> {
        self.try_as_resource_any(store)
    }
}

unsafe impl<T, D> ComponentType for HostResource<T, D>
where
    T: HostResourceType<D>,
    D: PartialEq + Send + Sync + Copy + 'static,
{
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        let resource = match ty {
            InterfaceType::Own(t) | InterfaceType::Borrow(t) => *t,
            other => bail!("expected `own` or `borrow`, found `{}`", desc(other)),
        };
        if T::typecheck(types.resource_type(resource)).is_none() {
            bail!("resource type mismatch");
        }

        Ok(())
    }

    fn as_val(&self, store: impl AsContextMut) -> Result<Val> {
        Ok(Val::Resource(self.try_as_resource_any(store)?))
    }
}

unsafe impl<T, D> Lower for HostResource<T, D>
where
    T: HostResourceType<D>,
    D: PartialEq + Send + Sync + Copy + 'static,
{
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

unsafe impl<T, D> Lift for HostResource<T, D>
where
    T: HostResourceType<D>,
    D: PartialEq + Send + Sync + Copy + 'static,
{
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let index = u32::linear_lift_from_flat(cx, InterfaceType::U32, src)?;
        HostResource::lift_from_index(cx, ty, index)
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let index = u32::linear_lift_from_memory(cx, InterfaceType::U32, bytes)?;
        HostResource::lift_from_index(cx, ty, index)
    }
}

impl<T, D> fmt::Debug for HostResource<T, D>
where
    T: HostResourceType<D>,
    D: PartialEq + Send + Sync + Copy + 'static,
{
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
