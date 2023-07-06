use crate::component::func::{bad_type_info, desc, LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::{ComponentType, Lift, Lower};
use crate::store::StoreId;
use crate::{AsContextMut, StoreContextMut};
use anyhow::{bail, Result};
use std::any::TypeId;
use std::marker;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use wasmtime_environ::component::{CanonicalAbiInfo, DefinedResourceIndex, InterfaceType};
use wasmtime_runtime::component::ComponentInstance;
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
pub struct Resource<T> {
    rep: ResourceRep,
    _marker: marker::PhantomData<fn() -> T>,
}

impl<T> Resource<T> {
    /// TODO
    pub fn new(rep: u32) -> Resource<T> {
        Resource {
            rep: ResourceRep::new(rep),
            _marker: marker::PhantomData,
        }
    }

    /// TODO - document panic
    pub fn rep(&self) -> u32 {
        match self.rep.get() {
            Some(val) => val,
            None => todo!(),
        }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        let resource = match ty {
            InterfaceType::Own(t) => t,
            _ => bad_type_info(),
        };
        let rep = self.rep.take()?;
        Ok(cx.resource_lower_own(resource, rep))
    }

    fn lift_from_index(cx: &LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let resource = match ty {
            InterfaceType::Own(t) => t,
            _ => bad_type_info(),
        };
        let (rep, _dtor) = cx.resource_lift_own(resource, index)?;
        // TODO: should debug assert types match here
        Ok(Resource::new(rep))
    }
}

unsafe impl<T: 'static> ComponentType for Resource<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        let resource = match ty {
            InterfaceType::Own(t) => *t,
            other => bail!("expected `own` found `{}`", desc(other)),
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
    fn lift(cx: &LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Resource::lift_from_index(cx, ty, index)
    }

    fn load(cx: &LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Resource::lift_from_index(cx, ty, index)
    }
}

/// TODO
#[derive(Debug)]
pub struct ResourceAny {
    store: StoreId,
    rep: ResourceRep,
    ty: ResourceType,
    dtor: Option<SendSyncPtr<VMFuncRef>>,
}

impl ResourceAny {
    /// TODO
    pub fn ty(&self) -> ResourceType {
        self.ty
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
        assert_eq!(store.0.id(), self.store);
        let rep = self.rep.take()?;
        let dtor = match self.dtor {
            Some(dtor) => dtor.as_non_null(),
            None => return Ok(()),
        };
        let mut args = [ValRaw::u32(rep)];
        // TODO: unsafe call
        unsafe { crate::Func::call_unchecked_raw(store, dtor, args.as_mut_ptr(), args.len()) }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        let resource = match ty {
            InterfaceType::Own(t) => t,
            _ => bad_type_info(),
        };
        if cx.resource_type(resource) != self.ty {
            bail!("mismatched resource types")
        }
        let rep = self.rep.take()?;
        Ok(cx.resource_lower_own(resource, rep))
    }

    fn lift_from_index(cx: &LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let resource = match ty {
            InterfaceType::Own(t) => t,
            _ => bad_type_info(),
        };
        let (rep, dtor) = cx.resource_lift_own(resource, index)?;
        let ty = cx.resource_type(resource);
        Ok(ResourceAny {
            store: cx.store.id(),
            rep: ResourceRep::new(rep),
            ty,
            dtor: dtor.map(SendSyncPtr::new),
        })
    }

    /// TODO
    pub(crate) fn partial_eq_for_val(&self, other: &ResourceAny) -> bool {
        self.store == other.store && self.ty == other.ty && self.rep.get() == other.rep.get()
    }

    /// TODO
    pub(crate) fn clone_for_val(&self) -> ResourceAny {
        ResourceAny {
            store: self.store,
            ty: self.ty,
            rep: ResourceRep::empty(),
            dtor: None,
        }
    }
}

unsafe impl ComponentType for ResourceAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Own(_) => Ok(()),
            other => bail!("expected `own` found `{}`", desc(other)),
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
    fn lift(cx: &LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }

    fn load(cx: &LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }
}

/// TODO
#[derive(Debug)]
struct ResourceRep(AtomicU64);

impl ResourceRep {
    fn new(rep: u32) -> ResourceRep {
        ResourceRep(AtomicU64::new((u64::from(rep) << 1) | 1))
    }

    fn empty() -> ResourceRep {
        ResourceRep(AtomicU64::new(0))
    }

    fn take(&self) -> Result<u32> {
        match self.0.swap(0, Relaxed) {
            0 => bail!("resource already consumed"),
            n => Ok((n >> 1) as u32),
        }
    }

    fn get(&self) -> Option<u32> {
        match self.0.load(Relaxed) {
            0 => None,
            n => Some((n >> 1) as u32),
        }
    }
}
