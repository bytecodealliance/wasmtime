//! Implementation of [`FutureAny`] and [`StreamAny`].

use crate::AsContextMut;
use crate::component::concurrent::futures_and_streams::{self, TransmitOrigin};
use crate::component::concurrent::{TableId, TransmitHandle};
use crate::component::func::{LiftContext, LowerContext, bad_type_info, desc};
use crate::component::matching::InstanceType;
use crate::component::types::{self, FutureType, StreamType};
use crate::component::{
    ComponentInstanceId, ComponentType, FutureReader, Lift, Lower, StreamReader,
};
use crate::store::StoreOpaque;
use anyhow::{Context, Result, bail};
use std::any::TypeId;
use std::mem::MaybeUninit;
use wasmtime_environ::component::{
    CanonicalAbiInfo, InterfaceType, TypeFutureTableIndex, TypeStreamTableIndex,
};

/// Represents a type-erased component model `future`.
///
/// This type is similar to [`ResourceAny`](crate::component::ResourceAny)
/// where it's a static guarantee that it represents a component model
/// `future`, but it does not contain any information about the underlying type
/// that is associated with this future. This is intended to be used in
/// "dynamically typed" situations where embedders may not know ahead of time
/// the type of a `future` being used by component that is loaded.
///
/// # Closing futures
///
/// A [`FutureAny`] represents a resource that is owned by a [`Store`]. Proper
/// disposal of a future requires invoking the [`FutureAny::close`] method to
/// ensure that this handle does not leak. If [`FutureAny::close`] is not
/// called then memory will not be leaked once the owning [`Store`] is dropped,
/// but the resource handle will be leaked until the [`Store`] is dropped.
///
/// [`Store`]: crate::Store
#[derive(Debug, Clone, PartialEq)]
pub struct FutureAny {
    id: TableId<TransmitHandle>,
    ty: PayloadType<FutureType>,
}

impl FutureAny {
    fn lower_to_index<T>(&self, cx: &mut LowerContext<'_, T>, ty: InterfaceType) -> Result<u32> {
        // Note that unlike `FutureReader<T>` we need to perform an extra
        // typecheck to ensure that the dynamic type of this future matches
        // what the guest we're lowering into expects. This couldn't happen
        // before this point (see the `ComponentType::typecheck` implementation
        // for this type), so do it now.
        let future_ty = match ty {
            InterfaceType::Future(payload) => payload,
            _ => bad_type_info(),
        };
        let payload = cx.types[cx.types[future_ty].ty].payload.as_ref();
        self.ty.typecheck_guest(
            &cx.instance_type(),
            payload,
            FutureType::equivalent_payload_guest,
        )?;

        // Like `FutureReader<T>`, however, lowering "just" gets a u32.
        futures_and_streams::lower_future_to_index(self.id, cx, ty)
    }

    /// Attempts to convert this [`FutureAny`] to a [`FutureReader<T>`]
    /// with a statically known type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `T` does not match the type of
    /// value on this future.
    pub fn try_into_future_reader<T>(self) -> Result<FutureReader<T>>
    where
        T: ComponentType + 'static,
    {
        self.ty
            .typecheck_host::<T>(FutureType::equivalent_payload_host::<T>)?;
        Ok(FutureReader::new_(self.id))
    }

    /// Attempts to convert `reader` to a [`FutureAny`], erasing its statically
    /// known type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `reader` does not belong to
    /// `store`.
    pub fn try_from_future_reader<T>(
        mut store: impl AsContextMut,
        reader: FutureReader<T>,
    ) -> Result<Self>
    where
        T: ComponentType + 'static,
    {
        let store = store.as_context_mut();
        let ty = match store.0.transmit_origin(reader.id())? {
            TransmitOrigin::Host => PayloadType::new_host::<T>(),
            TransmitOrigin::GuestFuture(id, ty) => PayloadType::new_guest_future(store.0, id, ty),
            TransmitOrigin::GuestStream(..) => bail!("not a future"),
        };
        Ok(FutureAny {
            id: reader.id(),
            ty,
        })
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let id = futures_and_streams::lift_index_to_future(cx, ty, index)?;
        let InterfaceType::Future(ty) = ty else {
            unreachable!()
        };
        let ty = cx.types[ty].ty;
        Ok(FutureAny {
            id,
            // Note that this future might actually be a host-originating
            // future which means that this ascription of "the type is the
            // guest" may be slightly in accurate. The guest, however, has the
            // most accurate view of what type this future has so that should
            // be reasonable to ascribe as the type here regardless.
            ty: PayloadType::Guest(FutureType::from(ty, &cx.instance_type())),
        })
    }

    /// Close this `FutureAny`.
    ///
    /// This will close this future and cause any write that happens later to
    /// returned `DROPPED`.
    ///
    /// # Panics
    ///
    /// Panics if the `store` does not own this future. Usage of this future
    /// after calling `close` will also cause a panic.
    pub fn close(&mut self, mut store: impl AsContextMut) {
        futures_and_streams::future_close(store.as_context_mut().0, &mut self.id)
    }
}

unsafe impl ComponentType for FutureAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Future(_) => Ok(()),
            other => bail!("expected `future`, found `{}`", desc(other)),
        }
    }
}

unsafe impl Lower for FutureAny {
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

unsafe impl Lift for FutureAny {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let index = u32::linear_lift_from_flat(cx, InterfaceType::U32, src)?;
        Self::lift_from_index(cx, ty, index)
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let index = u32::linear_lift_from_memory(cx, InterfaceType::U32, bytes)?;
        Self::lift_from_index(cx, ty, index)
    }
}

/// Represents a type-erased component model `stream`.
///
/// This type is similar to [`ResourceAny`](crate::component::ResourceAny)
/// where it's a static guarantee that it represents a component model
/// `stream`, but it does not contain any information about the underlying type
/// that is associated with this stream. This is intended to be used in
/// "dynamically typed" situations where embedders may not know ahead of time
/// the type of a `stream` being used by component that is loaded.
///
/// # Closing streams
///
/// A [`StreamAny`] represents a resource that is owned by a [`Store`]. Proper
/// disposal of a stream requires invoking the [`StreamAny::close`] method to
/// ensure that this handle does not leak. If [`StreamAny::close`] is not
/// called then memory will not be leaked once the owning [`Store`] is dropped,
/// but the resource handle will be leaked until the [`Store`] is dropped.
///
/// [`Store`]: crate::Store
#[derive(Debug, Clone, PartialEq)]
pub struct StreamAny {
    id: TableId<TransmitHandle>,
    ty: PayloadType<StreamType>,
}

impl StreamAny {
    fn lower_to_index<T>(&self, cx: &mut LowerContext<'_, T>, ty: InterfaceType) -> Result<u32> {
        // See comments in `FutureAny::lower_to_index` for why this is
        // different from `StreamReader`'s implementation.
        let stream_ty = match ty {
            InterfaceType::Stream(payload) => payload,
            _ => bad_type_info(),
        };
        let payload = cx.types[cx.types[stream_ty].ty].payload.as_ref();
        self.ty.typecheck_guest(
            &cx.instance_type(),
            payload,
            StreamType::equivalent_payload_guest,
        )?;
        futures_and_streams::lower_stream_to_index(self.id, cx, ty)
    }

    /// Attempts to convert this [`StreamAny`] to a [`StreamReader<T>`]
    /// with a statically known type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `T` does not match the type of
    /// value on this stream.
    pub fn try_into_stream_reader<T>(self) -> Result<StreamReader<T>>
    where
        T: ComponentType + 'static,
    {
        self.ty
            .typecheck_host::<T>(StreamType::equivalent_payload_host::<T>)?;
        Ok(StreamReader::new_(self.id))
    }

    /// Attempts to convert `reader` to a [`StreamAny`], erasing its statically
    /// known type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `reader` does not belong to
    /// `store`.
    pub fn try_from_stream_reader<T>(
        mut store: impl AsContextMut,
        reader: StreamReader<T>,
    ) -> Result<Self>
    where
        T: ComponentType + 'static,
    {
        let store = store.as_context_mut();
        let ty = match store.0.transmit_origin(reader.id())? {
            TransmitOrigin::Host => PayloadType::new_host::<T>(),
            TransmitOrigin::GuestStream(id, ty) => PayloadType::new_guest_stream(store.0, id, ty),
            TransmitOrigin::GuestFuture(..) => bail!("not a stream"),
        };
        Ok(StreamAny {
            id: reader.id(),
            ty,
        })
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let id = futures_and_streams::lift_index_to_stream(cx, ty, index)?;
        let InterfaceType::Stream(ty) = ty else {
            unreachable!()
        };
        let ty = cx.types[ty].ty;
        Ok(StreamAny {
            id,
            // Note that this stream might actually be a host-originating, but
            // see the documentation in `FutureAny::lift_from_index` for why
            // this should be ok.
            ty: PayloadType::Guest(StreamType::from(ty, &cx.instance_type())),
        })
    }

    /// Close this `StreamAny`.
    ///
    /// This will close this stream and cause any write that happens later to
    /// returned `DROPPED`.
    ///
    /// # Panics
    ///
    /// Panics if the `store` does not own this stream. Usage of this stream
    /// after calling `close` will also cause a panic.
    pub fn close(&mut self, mut store: impl AsContextMut) {
        futures_and_streams::future_close(store.as_context_mut().0, &mut self.id)
    }
}

unsafe impl ComponentType for StreamAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Stream(_) => Ok(()),
            other => bail!("expected `stream`, found `{}`", desc(other)),
        }
    }
}

unsafe impl Lower for StreamAny {
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

unsafe impl Lift for StreamAny {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        let index = u32::linear_lift_from_flat(cx, InterfaceType::U32, src)?;
        Self::lift_from_index(cx, ty, index)
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        let index = u32::linear_lift_from_memory(cx, InterfaceType::U32, bytes)?;
        Self::lift_from_index(cx, ty, index)
    }
}

#[derive(Debug, Clone)]
enum PayloadType<T> {
    Guest(T),
    Host {
        id: TypeId,
        typecheck: fn(Option<&InterfaceType>, &InstanceType<'_>) -> Result<()>,
    },
}

impl<T: PartialEq> PartialEq for PayloadType<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PayloadType::Guest(a), PayloadType::Guest(b)) => a == b,
            (PayloadType::Guest(_), _) => false,
            (PayloadType::Host { id: a_id, .. }, PayloadType::Host { id: b_id, .. }) => {
                a_id == b_id
            }
            (PayloadType::Host { .. }, _) => false,
        }
    }
}

impl PayloadType<FutureType> {
    fn new_guest_future(
        store: &StoreOpaque,
        id: ComponentInstanceId,
        ty: TypeFutureTableIndex,
    ) -> Self {
        let types = InstanceType::new(&store.component_instance(id));
        let ty = types.types[ty].ty;
        PayloadType::Guest(FutureType::from(ty, &types))
    }
}

impl PayloadType<StreamType> {
    fn new_guest_stream(
        store: &StoreOpaque,
        id: ComponentInstanceId,
        ty: TypeStreamTableIndex,
    ) -> Self {
        let types = InstanceType::new(&store.component_instance(id));
        let ty = types.types[ty].ty;
        PayloadType::Guest(StreamType::from(ty, &types))
    }
}

impl<T> PayloadType<T> {
    fn new_host<P>() -> Self
    where
        P: ComponentType + 'static,
    {
        PayloadType::Host {
            typecheck: types::typecheck_payload::<P>,
            id: TypeId::of::<P>(),
        }
    }

    fn typecheck_guest(
        &self,
        types: &InstanceType<'_>,
        payload: Option<&InterfaceType>,
        equivalent: fn(&T, &InstanceType<'_>, Option<&InterfaceType>) -> bool,
    ) -> Result<()> {
        match self {
            Self::Guest(ty) => {
                if equivalent(ty, types, payload) {
                    Ok(())
                } else {
                    bail!("future payload types differ")
                }
            }
            Self::Host { typecheck, .. } => {
                typecheck(payload, types).context("future payload types differ")
            }
        }
    }

    fn typecheck_host<P>(&self, equivalent: fn(&T) -> Result<()>) -> Result<()>
    where
        P: ComponentType + 'static,
    {
        match self {
            Self::Guest(ty) => equivalent(ty),
            Self::Host { id, .. } => {
                if *id == TypeId::of::<P>() {
                    Ok(())
                } else {
                    bail!("future payload types differ")
                }
            }
        }
    }
}
