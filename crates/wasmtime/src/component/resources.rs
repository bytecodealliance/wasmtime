use crate::component::func::{bad_type_info, desc, LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::{ComponentType, Lift, Lower};
use crate::store::{StoreId, StoreOpaque};
use crate::{AsContext, AsContextMut, StoreContextMut, Trap};
use anyhow::{bail, Result};
use std::any::TypeId;
use std::fmt;
use std::marker;
use std::mem::MaybeUninit;
use wasmtime_environ::component::{CanonicalAbiInfo, DefinedResourceIndex, InterfaceType};
use wasmtime_runtime::component::{ComponentInstance, InstanceFlags, ResourceTables};
use wasmtime_runtime::{SendSyncPtr, VMFuncRef, ValRaw};

/// TODO
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResourceType {
    kind: ResourceTypeKind,
}

impl ResourceType {
    /// TODO
    pub fn host<T: 'static>() -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Host(TypeId::of::<T>()),
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
                // TODO: comment this
                instance: instance as *const _ as usize,
                id,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ResourceTypeKind {
    Host(TypeId),
    Guest {
        store: StoreId,
        // TODO: comment what this `usize` is
        instance: usize,
        id: DefinedResourceIndex,
    },
}

/// TODO
///
/// document lack of dtor
///
/// document it's both borrow and own
pub struct Resource<T> {
    repr: ResourceRepr,
    _marker: marker::PhantomData<fn() -> T>,
}

enum ResourceRepr {
    Borrow(u32),
    OwnInTable(u32),
}

fn host_resource_tables(store: &mut StoreOpaque) -> ResourceTables<'_> {
    let (calls, host_table) = store.component_calls_and_host_table();
    ResourceTables {
        calls,
        host_table: Some(host_table),
        tables: None,
    }
}

impl<T> Resource<T>
where
    T: 'static,
{
    /// TODO
    pub fn new(mut store: impl AsContextMut, rep: u32) -> Resource<T> {
        let store = store.as_context_mut().0;
        let idx = host_resource_tables(store).resource_lower_own(None, rep);
        Resource {
            repr: ResourceRepr::OwnInTable(idx),
            _marker: marker::PhantomData,
        }
    }

    /// TODO
    pub fn rep(&self, store: impl AsContext) -> Result<u32> {
        match self.repr {
            ResourceRepr::OwnInTable(idx) => store.as_context().0.host_table().rep(idx),
            ResourceRepr::Borrow(rep) => Ok(rep),
        }
    }

    /// TODO
    pub fn owned(&self) -> bool {
        match self.repr {
            ResourceRepr::OwnInTable(_) => true,
            ResourceRepr::Borrow(_) => false,
        }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                let rep = match self.repr {
                    // If this resource lives in a host table then try to take
                    // it out of the table, which may fail, and on success we
                    // can move the rep into the guest table.
                    ResourceRepr::OwnInTable(idx) => cx.host_resource_lift_own(idx)?,

                    // If this is a borrow resource then this is a dynamic
                    // error on behalf of the embedder.
                    ResourceRepr::Borrow(_rep) => {
                        bail!("cannot lower a `borrow` resource into an `own`")
                    }
                };
                Ok(cx.guest_resource_lower_own(t, rep))
            }
            InterfaceType::Borrow(t) => {
                let rep = match self.repr {
                    // Borrowing an owned resource may fail because it could
                    // have been previously moved out. If successful this
                    // operation will record that the resource is borrowed for
                    // the duration of this call.
                    ResourceRepr::OwnInTable(idx) => cx.host_resource_lift_borrow(idx)?,

                    // Reborrowing host resources always succeeds and the
                    // representation can be plucked out easily here.
                    ResourceRepr::Borrow(rep) => rep,
                };
                Ok(cx.guest_resource_lower_borrow(t, rep))
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let repr = match ty {
            // Ownership is being transferred from a guest to the host, so move
            // it from the guest table into a fresh slot in the host table.
            InterfaceType::Own(t) => {
                debug_assert!(cx.resource_type(t) == ResourceType::host::<T>());
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                assert!(dtor.is_some());
                assert!(flags.is_none());
                ResourceRepr::OwnInTable(cx.host_resource_lower_own(rep))
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
                ResourceRepr::Borrow(rep)
            }
            _ => bad_type_info(),
        };
        Ok(Resource {
            repr,
            _marker: marker::PhantomData,
        })
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
        match types.resource_type(resource).kind {
            ResourceTypeKind::Host(id) if TypeId::of::<T>() == id => {}
            _ => bail!("resource type mismatch"),
        }

        Ok(())
    }
}

unsafe impl<T: 'static> Lower for Resource<T> {
    fn lower<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .lower(cx, InterfaceType::U32, dst)
    }

    fn store<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .store(cx, InterfaceType::U32, offset)
    }
}

unsafe impl<T: 'static> Lift for Resource<T> {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Resource::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Resource::lift_from_index(cx, ty, index)
    }
}

/// TODO
///
/// document it's both borrow and own
///
/// document dtor importance
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ResourceAny {
    idx: u32,
    ty: ResourceType,
    own_state: Option<OwnState>,
}

#[derive(Copy, Clone)]
struct OwnState {
    store: StoreId,
    flags: Option<InstanceFlags>,
    dtor: Option<SendSyncPtr<VMFuncRef>>,
}

impl ResourceAny {
    /// TODO
    pub fn ty(&self) -> ResourceType {
        self.ty
    }

    /// TODO
    pub fn owned(&self) -> bool {
        self.own_state.is_some()
    }

    /// TODO
    pub fn resource_drop(self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `resource_drop_async` when async support is enabled on the config"
        );
        self.resource_drop_impl(&mut store.as_context_mut())
    }

    /// TODO
    pub async fn resource_drop_async<T>(self, mut store: impl AsContextMut<Data = T>) -> Result<()>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `resource_drop_async` without enabling async support in the config"
        );
        store
            .on_fiber(|store| self.resource_drop_impl(store))
            .await?
    }

    fn resource_drop_impl<T>(self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        // Attempt to remove `self.idx` from the host table in `store`.
        //
        // This could fail if the index is invalid or if this is removing an
        // `Own` entry which is currently being borrowed.
        let rep = host_resource_tables(store.0).resource_drop(None, self.idx)?;

        let (rep, state) = match (rep, &self.own_state) {
            (Some(rep), Some(state)) => (rep, state),

            // A `borrow` was removed from the table and no further
            // destruction, e.g. the destructor, is required so we're done.
            (None, None) => return Ok(()),

            _ => unreachable!(),
        };

        // Double-check that accessing the raw pointers on `state` are safe due
        // to the presence of `store` above.
        assert_eq!(
            store.0.id(),
            state.store,
            "wrong store used to destroy resource"
        );

        // Implement the reentrance check required by the canonical ABI. Note
        // that this happens whether or not a destructor is present.
        //
        // Note that this should be safe because the raw pointer access in
        // `flags` is valid due to `store` being the owner of the flags and
        // flags are never destroyed within the store.
        if let Some(flags) = state.flags {
            unsafe {
                if !flags.may_enter() {
                    bail!(Trap::CannotEnterComponent);
                }
            }
        }

        let dtor = match state.dtor {
            Some(dtor) => dtor.as_non_null(),
            None => return Ok(()),
        };
        let mut args = [ValRaw::u32(rep)];

        // This should be safe because `dtor` has been checked to belong to the
        // `store` provided which means it's valid and still alive. Additionally
        // destructors have al been previously type-checked and are guaranteed
        // to take one i32 argument and return no results, so the parameters
        // here should be configured correctly.
        unsafe { crate::Func::call_unchecked_raw(store, dtor, args.as_mut_ptr(), args.len()) }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types")
                }
                let rep = cx.host_resource_lift_own(self.idx)?;
                Ok(cx.guest_resource_lower_own(t, rep))
            }
            InterfaceType::Borrow(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types")
                }
                let rep = cx.host_resource_lift_borrow(self.idx)?;
                Ok(cx.guest_resource_lower_borrow(t, rep))
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Own(t) => {
                let ty = cx.resource_type(t);
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                let idx = cx.host_resource_lower_own(rep);
                Ok(ResourceAny {
                    idx,
                    ty,
                    own_state: Some(OwnState {
                        dtor: dtor.map(SendSyncPtr::new),
                        flags,
                        store: cx.store_id(),
                    }),
                })
            }
            InterfaceType::Borrow(t) => {
                let ty = cx.resource_type(t);
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                let idx = cx.host_resource_lower_borrow(rep);
                Ok(ResourceAny {
                    idx,
                    ty,
                    own_state: None,
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
    fn lower<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .lower(cx, InterfaceType::U32, dst)
    }

    fn store<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .store(cx, InterfaceType::U32, offset)
    }
}

unsafe impl Lift for ResourceAny {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }
}

impl fmt::Debug for OwnState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnState")
            .field("store", &self.store)
            .finish()
    }
}

// This is a loose definition for `Val` primarily so it doesn't need to be
// strictly 100% correct, and equality of resources is a bit iffy anyway, so
// ignore equality here and only factor in the indices and other metadata in
// `ResourceAny`.
impl PartialEq for OwnState {
    fn eq(&self, _other: &OwnState) -> bool {
        true
    }
}

impl Eq for OwnState {}
