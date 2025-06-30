use super::ConcurrentState;
use crate::component::func::{self, LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::{Instance, Val};
use crate::store::StoreOpaque;
use crate::vm::{VMFuncRef, VMMemoryDefinition, VMStore};
use anyhow::Result;
use std::{marker::PhantomData, mem::MaybeUninit};
use wasmtime_environ::component::{
    CanonicalAbiInfo, InterfaceType, TypeComponentLocalErrorContextTableIndex,
    TypeFutureTableIndex, TypeStreamTableIndex,
};

/// Represents the readable end of a Component Model `future`.
///
/// In order to actually read from or drop this `future`, first convert it to a
/// [`FutureReader`] using the `into_reader` method.
///
/// Note that if a value of this type is dropped without either being converted
/// to a `FutureReader` or passed to the guest, any writes on the write end may
/// block forever.
pub struct HostFuture<T> {
    _phantom: PhantomData<T>,
}

impl<T> HostFuture<T> {
    /// Convert this `HostFuture` into a [`Val`].
    // See TODO comment for `FutureAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        todo!()
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower future handles as `u32`s.
unsafe impl<T: Send + Sync> func::ComponentType for HostFuture<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        _ = ty;
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lower for HostFuture<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        _ = (cx, ty, dst);
        todo!()
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        _ = (cx, ty, offset);
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lift for HostFuture<T> {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        _ = (cx, ty, src);
        todo!()
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        _ = (cx, ty, bytes);
        todo!()
    }
}

/// Transfer ownership of the read end of a future from the host to a guest.
pub(crate) fn lower_future_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    _ = (rep, cx, ty);
    todo!()
}

/// Represents the readable end of a Component Model `future`.
pub struct FutureReader<T> {
    _phantom: PhantomData<T>,
}

/// Represents the readable end of a Component Model `stream`.
///
/// In order to actually read from or drop this `stream`, first convert it to a
/// [`FutureReader`] using the `into_reader` method.
///
/// Note that if a value of this type is dropped without either being converted
/// to a `StreamReader` or passed to the guest, any writes on the write end may
/// block forever.
pub struct HostStream<T> {
    _phantom: PhantomData<T>,
}

impl<T> HostStream<T> {
    /// Convert this `HostStream` into a [`Val`].
    // See TODO comment for `StreamAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        todo!()
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower stream handles as `u32`s.
unsafe impl<T: Send + Sync> func::ComponentType for HostStream<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        _ = ty;
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lower for HostStream<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        _ = (cx, ty, dst);
        todo!()
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        _ = (cx, ty, offset);
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lift for HostStream<T> {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        _ = (cx, ty, src);
        todo!()
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        _ = (cx, ty, bytes);
        todo!()
    }
}

/// Transfer ownership of the read end of a stream from the host to a guest.
pub(crate) fn lower_stream_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    _ = (rep, cx, ty);
    todo!()
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T> {
    _phantom: PhantomData<T>,
}

/// Represents the writable end of a Component Model `future`.
pub struct FutureWriter<T> {
    _phantom: PhantomData<T>,
}

/// Represents the writable end of a Component Model `stream`.
pub struct StreamWriter<T> {
    _phantom: PhantomData<T>,
}

/// Represents a Component Model `error-context`.
pub struct ErrorContext {}

impl ErrorContext {
    /// Convert this `ErrorContext` into a [`Val`].
    pub fn into_val(self) -> Val {
        todo!()
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower future handles as `u32`s.
unsafe impl func::ComponentType for ErrorContext {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        _ = ty;
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl func::Lower for ErrorContext {
    fn linear_lower_to_flat<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        _ = (cx, ty, dst);
        todo!()
    }

    fn linear_lower_to_memory<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        _ = (cx, ty, offset);
        todo!()
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl func::Lift for ErrorContext {
    fn linear_lift_from_flat(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &Self::Lower,
    ) -> Result<Self> {
        _ = (cx, ty, src);
        todo!()
    }

    fn linear_lift_from_memory(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        bytes: &[u8],
    ) -> Result<Self> {
        _ = (cx, ty, bytes);
        todo!()
    }
}

/// Transfer ownership of an error-context from the host to a guest.
pub(crate) fn lower_error_context_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    _ = (rep, cx, ty);
    todo!()
}

pub(crate) struct ResourcePair {
    pub(crate) write: u32,
    pub(crate) read: u32,
}

impl ConcurrentState {
    pub(crate) fn future_new(&mut self, ty: TypeFutureTableIndex) -> Result<ResourcePair> {
        _ = ty;
        todo!()
    }

    /// Implements the `future.cancel-write` intrinsic.
    pub(crate) fn future_cancel_write(
        &mut self,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        _ = (ty, async_, writer);
        todo!()
    }

    /// Implements the `future.cancel-read` intrinsic.
    pub(crate) fn future_cancel_read(
        &mut self,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        _ = (ty, async_, reader);
        todo!()
    }

    /// Implements the `future.drop-writable` intrinsic.
    pub(crate) fn future_drop_writable(
        &mut self,
        ty: TypeFutureTableIndex,
        writer: u32,
    ) -> Result<()> {
        _ = (ty, writer);
        todo!()
    }

    /// Implements the `stream.new` intrinsic.
    pub(crate) fn stream_new(&mut self, ty: TypeStreamTableIndex) -> Result<ResourcePair> {
        _ = ty;
        todo!()
    }

    /// Implements the `stream.cancel-write` intrinsic.
    pub(crate) fn stream_cancel_write(
        &mut self,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        _ = (ty, async_, writer);
        todo!()
    }

    /// Implements the `stream.cancel-read` intrinsic.
    pub(crate) fn stream_cancel_read(
        &mut self,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        _ = (ty, async_, reader);
        todo!()
    }

    /// Implements the `stream.drop-writable` intrinsic.
    pub(crate) fn stream_drop_writable(
        &mut self,
        ty: TypeStreamTableIndex,
        writer: u32,
    ) -> Result<()> {
        _ = (ty, writer);
        todo!()
    }

    pub(crate) fn future_transfer(
        &mut self,
        src_idx: u32,
        src: TypeFutureTableIndex,
        dst: TypeFutureTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    pub(crate) fn stream_transfer(
        &mut self,
        src_idx: u32,
        src: TypeStreamTableIndex,
        dst: TypeStreamTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    pub(crate) fn error_context_transfer(
        &mut self,
        src_idx: u32,
        src: TypeComponentLocalErrorContextTableIndex,
        dst: TypeComponentLocalErrorContextTableIndex,
    ) -> Result<u32> {
        _ = (src_idx, src, dst);
        todo!()
    }

    pub(crate) fn error_context_drop(
        &mut self,
        ty: TypeComponentLocalErrorContextTableIndex,
        error_context: u32,
    ) -> Result<()> {
        _ = (ty, error_context);
        todo!()
    }
}

impl Instance {
    /// Implements the `future.drop-readable` intrinsic.
    pub(crate) fn future_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TypeFutureTableIndex,
        reader: u32,
    ) -> Result<()> {
        _ = (store, ty, reader);
        todo!()
    }

    /// Implements the `stream.drop-readable` intrinsic.
    pub(crate) fn stream_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TypeStreamTableIndex,
        reader: u32,
    ) -> Result<()> {
        _ = (store, ty, reader);
        todo!()
    }

    /// Create a new error context for the given component.
    ///
    /// SAFETY: `memory` and `realloc` must be valid pointers to their
    /// respective guest entities.
    pub(crate) unsafe fn error_context_new(
        self,
        store: &mut StoreOpaque,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        debug_msg_address: u32,
        debug_msg_len: u32,
    ) -> Result<u32> {
        _ = (
            store,
            memory,
            realloc,
            string_encoding,
            ty,
            debug_msg_address,
            debug_msg_len,
        );
        todo!()
    }
}
