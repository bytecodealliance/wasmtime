use super::table::{TableDebug, TableId};
use super::{Event, GlobalErrorContextRefCount, Waitable, WaitableCommon};
use crate::component::concurrent::{ConcurrentState, WorkItem, tls};
use crate::component::func::{self, LiftContext, LowerContext, Options};
use crate::component::matching::InstanceType;
use crate::component::values::{ErrorContextAny, FutureAny, StreamAny};
use crate::component::{AsAccessor, Instance, Lower, Val, WasmList, WasmStr};
use crate::store::{StoreOpaque, StoreToken};
use crate::vm::component::{ComponentInstance, HandleTable, TransmitLocalState};
use crate::vm::{AlwaysMut, VMStore};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{Context as _, Error, Result, anyhow, bail};
use buffers::{Extender, SliceBuffer, UntypedWriteBuffer};
use core::fmt;
use core::future;
use core::iter;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll, Waker, ready};
use futures::channel::oneshot;
use futures::{FutureExt as _, stream};
use std::boxed::Box;
use std::io::Cursor;
use std::string::{String, ToString};
use std::sync::{Arc, Mutex};
use std::vec::Vec;
use wasmtime_environ::component::{
    CanonicalAbiInfo, ComponentTypes, InterfaceType, OptionsIndex,
    TypeComponentGlobalErrorContextTableIndex, TypeComponentLocalErrorContextTableIndex,
    TypeFutureTableIndex, TypeStreamTableIndex,
};

pub use buffers::{ReadBuffer, VecBuffer, WriteBuffer};

mod buffers;

/// Enum for distinguishing between a stream or future in functions that handle
/// both.
#[derive(Copy, Clone, Debug)]
pub enum TransmitKind {
    Stream,
    Future,
}

/// Represents `{stream,future}.{read,write}` results.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReturnCode {
    Blocked,
    Completed(u32),
    Dropped(u32),
    Cancelled(u32),
}

impl ReturnCode {
    /// Pack `self` into a single 32-bit integer that may be returned to the
    /// guest.
    ///
    /// This corresponds to `pack_copy_result` in the Component Model spec.
    pub fn encode(&self) -> u32 {
        const BLOCKED: u32 = 0xffff_ffff;
        const COMPLETED: u32 = 0x0;
        const DROPPED: u32 = 0x1;
        const CANCELLED: u32 = 0x2;
        match self {
            ReturnCode::Blocked => BLOCKED,
            ReturnCode::Completed(n) => {
                debug_assert!(*n < (1 << 28));
                (n << 4) | COMPLETED
            }
            ReturnCode::Dropped(n) => {
                debug_assert!(*n < (1 << 28));
                (n << 4) | DROPPED
            }
            ReturnCode::Cancelled(n) => {
                debug_assert!(*n < (1 << 28));
                (n << 4) | CANCELLED
            }
        }
    }

    /// Returns `Self::Completed` with the specified count (or zero if
    /// `matches!(kind, TransmitKind::Future)`)
    fn completed(kind: TransmitKind, count: u32) -> Self {
        Self::Completed(if let TransmitKind::Future = kind {
            0
        } else {
            count
        })
    }
}

/// Represents a stream or future type index.
///
/// This is useful as a parameter type for functions which operate on either a
/// future or a stream.
#[derive(Copy, Clone, Debug)]
pub enum TransmitIndex {
    Stream(TypeStreamTableIndex),
    Future(TypeFutureTableIndex),
}

impl TransmitIndex {
    pub fn kind(&self) -> TransmitKind {
        match self {
            TransmitIndex::Stream(_) => TransmitKind::Stream,
            TransmitIndex::Future(_) => TransmitKind::Future,
        }
    }
}

/// Retrieve the payload type of the specified stream or future, or `None` if it
/// has no payload type.
fn payload(ty: TransmitIndex, types: &Arc<ComponentTypes>) -> Option<InterfaceType> {
    match ty {
        TransmitIndex::Future(ty) => types[types[ty].ty].payload,
        TransmitIndex::Stream(ty) => types[types[ty].ty].payload,
    }
}

/// Retrieve the host rep and state for the specified guest-visible waitable
/// handle.
fn get_mut_by_index_from(
    handle_table: &mut HandleTable,
    ty: TransmitIndex,
    index: u32,
) -> Result<(u32, &mut TransmitLocalState)> {
    match ty {
        TransmitIndex::Stream(ty) => handle_table.stream_rep(ty, index),
        TransmitIndex::Future(ty) => handle_table.future_rep(ty, index),
    }
}

fn lower<T: func::Lower + Send + 'static, B: WriteBuffer<T>, U: 'static>(
    mut store: StoreContextMut<U>,
    instance: Instance,
    options: &Options,
    ty: TransmitIndex,
    address: usize,
    count: usize,
    buffer: &mut B,
) -> Result<()> {
    let types = instance.id().get(store.0).component().types().clone();
    let count = buffer.remaining().len().min(count);

    let lower = &mut if T::MAY_REQUIRE_REALLOC {
        LowerContext::new
    } else {
        LowerContext::new_without_realloc
    }(store.as_context_mut(), options, &types, instance);

    if address % usize::try_from(T::ALIGN32)? != 0 {
        bail!("read pointer not aligned");
    }
    lower
        .as_slice_mut()
        .get_mut(address..)
        .and_then(|b| b.get_mut(..T::SIZE32 * count))
        .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?;

    if let Some(ty) = payload(ty, &types) {
        T::linear_store_list_to_memory(lower, ty, address, &buffer.remaining()[..count])?;
    }

    buffer.skip(count);

    Ok(())
}

fn lift<T: func::Lift + Send + 'static, B: ReadBuffer<T>, U>(
    lift: &mut LiftContext<'_>,
    ty: Option<InterfaceType>,
    buffer: &mut B,
    address: usize,
    count: usize,
) -> Result<()> {
    let count = count.min(buffer.remaining_capacity());
    if T::IS_RUST_UNIT_TYPE {
        // SAFETY: `T::IS_RUST_UNIT_TYPE` is only true for `()`, a
        // zero-sized type, so `MaybeUninit::uninit().assume_init()`
        // is a valid way to populate the zero-sized buffer.
        buffer.extend(
            iter::repeat_with(|| unsafe { MaybeUninit::uninit().assume_init() }).take(count),
        )
    } else {
        let ty = ty.unwrap();
        if address % usize::try_from(T::ALIGN32)? != 0 {
            bail!("write pointer not aligned");
        }
        lift.memory()
            .get(address..)
            .and_then(|b| b.get(..T::SIZE32 * count))
            .ok_or_else(|| anyhow::anyhow!("write pointer out of bounds of memory"))?;

        let list = &WasmList::new(address, count, lift, ty)?;
        T::linear_lift_into_from_memory(lift, list, &mut Extender(buffer))?
    }
    Ok(())
}

/// Represents the state associated with an error context
#[derive(Debug, PartialEq, Eq, PartialOrd)]
pub(super) struct ErrorContextState {
    /// Debug message associated with the error context
    pub(crate) debug_msg: String,
}

/// Represents the size and alignment for a "flat" Component Model type,
/// i.e. one containing no pointers or handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FlatAbi {
    pub(super) size: u32,
    pub(super) align: u32,
}

/// Represents the buffer for a host- or guest-initiated stream read.
pub struct Destination<'a, T, B> {
    instance: Instance,
    id: TableId<TransmitState>,
    buffer: &'a mut B,
    host_buffer: Option<&'a mut Cursor<Vec<u8>>>,
    _phantom: PhantomData<fn() -> T>,
}

impl<'a, T, B> Destination<'a, T, B> {
    /// Reborrow `self` so it can be used again later.
    pub fn reborrow(&mut self) -> Destination<'_, T, B> {
        Destination {
            instance: self.instance,
            id: self.id,
            buffer: &mut *self.buffer,
            host_buffer: self.host_buffer.as_deref_mut(),
            _phantom: PhantomData,
        }
    }

    /// Take the buffer out of `self`, leaving a default-initialized one in its
    /// place.
    ///
    /// This can be useful for reusing the previously-stored buffer's capacity
    /// instead of allocating a fresh one.
    pub fn take_buffer(&mut self) -> B
    where
        B: Default,
    {
        mem::take(self.buffer)
    }

    /// Store the specified buffer in `self`.
    ///
    /// Any items contained in the buffer will be delivered to the reader after
    /// the `StreamProducer::poll_produce` call to which this `Destination` was
    /// passed returns (unless overwritten by another call to `set_buffer`).
    ///
    /// If items are stored via this buffer _and_ written via a
    /// `DirectDestination` view of `self`, then the items in the buffer will be
    /// delivered after the ones written using `DirectDestination`.
    pub fn set_buffer(&mut self, buffer: B) {
        *self.buffer = buffer;
    }

    /// Return the remaining number of items the current read has capacity to
    /// accept, if known.
    ///
    /// This will return `Some(_)` if the reader is a guest; it will return
    /// `None` if the reader is the host.
    ///
    /// Note that, if this returns `None(0)`, the producer must still attempt to
    /// produce at least one item if the value of `finish` passed to
    /// `StreamProducer::poll_produce` is false.  In that case, the reader is
    /// effectively asking when the producer will be able to produce items
    /// without blocking (or reach a terminal state such as end-of-stream),
    /// meaning the next non-zero read must complete without blocking.
    pub fn remaining(&self, mut store: impl AsContextMut) -> Option<usize> {
        let transmit = self
            .instance
            .concurrent_state_mut(store.as_context_mut().0)
            .get_mut(self.id)
            .unwrap();

        if let &ReadState::GuestReady { count, .. } = &transmit.read {
            let &WriteState::HostReady { guest_offset, .. } = &transmit.write else {
                unreachable!()
            };

            Some(count - guest_offset)
        } else {
            None
        }
    }
}

impl<'a, B> Destination<'a, u8, B> {
    /// Return a `DirectDestination` view of `self`.
    ///
    /// If the reader is a guest, this will provide direct access to the guest's
    /// read buffer.  If the reader is a host, this will provide access to a
    /// buffer which will be delivered to the host before any items stored using
    /// `Destination::set_buffer`.
    ///
    /// `capacity` will only be used if the reader is a host, in which case it
    /// will update the length of the buffer, possibly zero-initializing the new
    /// elements if the new length is larger than the old length.
    pub fn as_direct<D>(
        mut self,
        store: StoreContextMut<'a, D>,
        capacity: usize,
    ) -> DirectDestination<'a, D> {
        if let Some(buffer) = self.host_buffer.as_deref_mut() {
            buffer.set_position(0);
            if buffer.get_mut().is_empty() {
                buffer.get_mut().resize(capacity, 0);
            }
        }

        DirectDestination {
            instance: self.instance,
            id: self.id,
            host_buffer: self.host_buffer,
            store,
        }
    }
}

/// Represents a read from a `stream<u8>`, providing direct access to the
/// writer's buffer.
pub struct DirectDestination<'a, D: 'static> {
    instance: Instance,
    id: TableId<TransmitState>,
    host_buffer: Option<&'a mut Cursor<Vec<u8>>>,
    store: StoreContextMut<'a, D>,
}

impl<D: 'static> DirectDestination<'_, D> {
    /// Provide direct access to the writer's buffer.
    pub fn remaining(&mut self) -> &mut [u8] {
        if let Some(buffer) = self.host_buffer.as_deref_mut() {
            buffer.get_mut()
        } else {
            let transmit = self
                .instance
                .concurrent_state_mut(self.store.as_context_mut().0)
                .get_mut(self.id)
                .unwrap();

            let &ReadState::GuestReady {
                address,
                count,
                options,
                ..
            } = &transmit.read
            else {
                unreachable!();
            };

            let &WriteState::HostReady { guest_offset, .. } = &transmit.write else {
                unreachable!()
            };

            options
                .memory_mut(self.store.0)
                .get_mut((address + guest_offset)..)
                .and_then(|b| b.get_mut(..(count - guest_offset)))
                .unwrap()
        }
    }

    /// Mark the specified number of bytes as written to the writer's buffer.
    ///
    /// This will panic if the count is larger than the size of the
    /// buffer returned by `Self::remaining`.
    pub fn mark_written(&mut self, count: usize) {
        if let Some(buffer) = self.host_buffer.as_deref_mut() {
            buffer.set_position(
                buffer
                    .position()
                    .checked_add(u64::try_from(count).unwrap())
                    .unwrap(),
            );
        } else {
            let transmit = self
                .instance
                .concurrent_state_mut(self.store.as_context_mut().0)
                .get_mut(self.id)
                .unwrap();

            let ReadState::GuestReady {
                count: read_count, ..
            } = &transmit.read
            else {
                unreachable!();
            };

            let WriteState::HostReady { guest_offset, .. } = &mut transmit.write else {
                unreachable!()
            };

            if *guest_offset + count > *read_count {
                panic!(
                    "write count ({count}) must be less than or equal to read count ({read_count})"
                )
            } else {
                *guest_offset += count;
            }
        }
    }
}

/// Represents the state of a `Stream{Producer,Consumer}`.
#[derive(Copy, Clone, Debug)]
pub enum StreamResult {
    /// The operation completed normally, and the producer or consumer may be
    /// able to produce or consume more items, respectively.
    Completed,
    /// The operation was interrupted (i.e. it wrapped up early after receiving
    /// a `finish` parameter value of true in a call to `poll_produce` or
    /// `poll_consume`), and the producer or consumer may be able to produce or
    /// consume more items, respectively.
    Cancelled,
    /// The operation completed normally, but the producer or consumer will
    /// _not_ able to produce or consume more items, respectively.
    Dropped,
}

/// Represents the host-owned write end of a stream.
pub trait StreamProducer<D>: Send + 'static {
    /// The payload type of this stream.
    type Item;

    /// The `WriteBuffer` type to use when delivering items.
    type Buffer: WriteBuffer<Self::Item> + Default;

    /// Handle a host- or guest-initiated read by delivering zero or more items
    /// to the specified destination.
    ///
    /// This will be called whenever the reader starts a read.
    ///
    /// If the implementation is able to produce one or more items immediately,
    /// it should write them to `destination` and return either
    /// `Poll::Ready(Ok(StreamResult::Completed))` if it expects to produce more
    /// items, or `Poll::Ready(Ok(StreamResult::Dropped))` if it cannot produce
    /// any more items.
    ///
    /// If the implementation is unable to produce any items immediately, but
    /// expects to do so later, and `finish` is _false_, it should store the
    /// waker from `cx` for later and return `Poll::Pending` without writing
    /// anything to `destination`.  Later, it should alert the waker when either
    /// the items arrive, the stream has ended, or an error occurs.
    ///
    /// If the implementation is unable to produce any items immediately, but
    /// expects to do so later, and `finish` is _true_, it should, if possible,
    /// return `Poll::Ready(Ok(StreamResult::Cancelled))` immediately without
    /// writing anything to `destination`.  However, that might not be possible
    /// if an earlier call to `poll_produce` kicked off an asynchronous
    /// operation which needs to be completed (and possibly interrupted)
    /// gracefully, in which case the implementation may return `Poll::Pending`
    /// and later alert the waker as described above.  In other words, when
    /// `finish` is true, the implementation should prioritize returning a
    /// result to the reader (even if no items can be produced) rather than wait
    /// indefinitely for at least one item to arrive.
    ///
    /// In all of the above cases, the implementation may alternatively choose
    /// to return `Err(_)` to indicate an unrecoverable error.  This will cause
    /// the guest (if any) to trap and render the component instance (if any)
    /// unusable.  The implementation should report errors that _are_
    /// recoverable by other means (e.g. by writing to a `future`) and return
    /// `Poll::Ready(Ok(StreamResult::Dropped))`.
    ///
    /// Note that the implementation should never return `Poll::Pending` after
    /// writing one or more items to `destination`; if it does, the caller will
    /// trap as if `Err(_)` was returned.  Conversely, it should only return
    /// `Poll::Ready(Ok(StreamResult::Cancelled))` without writing any items to
    /// `destination` if called with `finish` set to true.  If it does so when
    /// `finish` is false, the caller will trap.  Additionally, it should only
    /// return `Poll::Ready(Ok(StreamResult::Completed))` after writing at least
    /// one item to `destination` if it has capacity to accept that item;
    /// otherwise, the caller will trap.
    ///
    /// If more items are written to `destination` than the reader has immediate
    /// capacity to accept, they will be retained in memory by the caller and
    /// used to satisfy future reads, in which case `poll_produce` will only be
    /// called again once all those items have been delivered.
    ///
    /// If this function is called with zero capacity
    /// (i.e. `Destination::remaining` returns `Some(0)`), the implementation
    /// should either:
    ///
    /// - Return `Poll::Ready(Ok(StreamResult::Completed))` without writing
    /// anything if it expects to be able to produce items immediately
    /// (i.e. without first returning `Poll::Pending`) the next time
    /// `poll_produce` is called with non-zero capacity _or_ if that cannot be
    /// reliably determined.
    ///
    /// - Return `Poll::Pending` if the next call to `poll_produce` with
    /// non-zero capacity is likely to also return `Poll::Pending`.
    ///
    /// - Return `Poll::Ready(Ok(StreamResult::Completed))` after calling
    /// `Destination::set_buffer` with one more more items.  Note, however, that
    /// this creates the hazard that the items will never be received by the
    /// guest if it decides not to do another non-zero-length read before
    /// closing the stream.  Moreover, if `Self::Item` is e.g. a `Resource<_>`,
    /// they may end up leaking in that scenario.
    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>>;
}

impl<T, D> StreamProducer<D> for iter::Empty<T>
where
    T: Send + Sync + 'static,
{
    type Item = T;
    type Buffer = Option<Self::Item>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        _: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

impl<T, D> StreamProducer<D> for stream::Empty<T>
where
    T: Send + Sync + 'static,
{
    type Item = T;
    type Buffer = Option<Self::Item>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        _: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

impl<T, D> StreamProducer<D> for Vec<T>
where
    T: Unpin + Send + Sync + 'static,
{
    type Item = T;
    type Buffer = VecBuffer<T>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        dst.set_buffer(mem::take(self.get_mut()).into());
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

impl<T, D> StreamProducer<D> for Box<[T]>
where
    T: Unpin + Send + Sync + 'static,
{
    type Item = T;
    type Buffer = VecBuffer<T>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        dst.set_buffer(mem::take(self.get_mut()).into_vec().into());
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

#[cfg(feature = "component-model-async-bytes")]
impl<D> StreamProducer<D> for bytes::Bytes {
    type Item = u8;
    type Buffer = Cursor<Self>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        let cap = dst.remaining(&mut store);
        let Some(cap) = cap.and_then(core::num::NonZeroUsize::new) else {
            // on 0-length or host reads, buffer the bytes
            dst.set_buffer(Cursor::new(mem::take(self.get_mut())));
            return Poll::Ready(Ok(StreamResult::Dropped));
        };
        let cap = cap.into();
        // data does not fit in destination, fill it and buffer the rest
        dst.set_buffer(Cursor::new(self.split_off(cap)));
        let mut dst = dst.as_direct(store, cap);
        dst.remaining().copy_from_slice(&self);
        dst.mark_written(cap);
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

#[cfg(feature = "component-model-async-bytes")]
impl<D> StreamProducer<D> for bytes::BytesMut {
    type Item = u8;
    type Buffer = Cursor<Self>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        let cap = dst.remaining(&mut store);
        let Some(cap) = cap.and_then(core::num::NonZeroUsize::new) else {
            // on 0-length or host reads, buffer the bytes
            dst.set_buffer(Cursor::new(mem::take(self.get_mut())));
            return Poll::Ready(Ok(StreamResult::Dropped));
        };
        let cap = cap.into();
        // data does not fit in destination, fill it and buffer the rest
        dst.set_buffer(Cursor::new(self.split_off(cap)));
        let mut dst = dst.as_direct(store, cap);
        dst.remaining().copy_from_slice(&self);
        dst.mark_written(cap);
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

/// Represents the buffer for a host- or guest-initiated stream write.
pub struct Source<'a, T> {
    instance: Instance,
    id: TableId<TransmitState>,
    host_buffer: Option<&'a mut dyn WriteBuffer<T>>,
}

impl<'a, T> Source<'a, T> {
    /// Reborrow `self` so it can be used again later.
    pub fn reborrow(&mut self) -> Source<'_, T> {
        Source {
            instance: self.instance,
            id: self.id,
            host_buffer: self.host_buffer.as_deref_mut(),
        }
    }

    /// Accept zero or more items from the writer.
    pub fn read<B, S: AsContextMut>(&mut self, mut store: S, buffer: &mut B) -> Result<()>
    where
        T: func::Lift + 'static,
        B: ReadBuffer<T>,
    {
        if let Some(input) = &mut self.host_buffer {
            let count = input.remaining().len().min(buffer.remaining_capacity());
            buffer.move_from(*input, count);
        } else {
            let store = store.as_context_mut();
            let transmit = self
                .instance
                .concurrent_state_mut(store.0)
                .get_mut(self.id)?;

            let &ReadState::HostReady { guest_offset, .. } = &transmit.read else {
                unreachable!();
            };

            let &WriteState::GuestReady {
                ty,
                address,
                count,
                options,
                ..
            } = &transmit.write
            else {
                unreachable!()
            };

            let cx = &mut LiftContext::new(store.0.store_opaque_mut(), &options, self.instance);
            let ty = payload(ty, cx.types);
            let old_remaining = buffer.remaining_capacity();
            lift::<T, B, S::Data>(
                cx,
                ty,
                buffer,
                address + (T::SIZE32 * guest_offset),
                count - guest_offset,
            )?;

            let transmit = self
                .instance
                .concurrent_state_mut(store.0)
                .get_mut(self.id)?;

            let ReadState::HostReady { guest_offset, .. } = &mut transmit.read else {
                unreachable!();
            };

            *guest_offset += old_remaining - buffer.remaining_capacity();
        }

        Ok(())
    }

    /// Return the number of items remaining to be read from the current write
    /// operation.
    pub fn remaining(&self, mut store: impl AsContextMut) -> usize
    where
        T: 'static,
    {
        let transmit = self
            .instance
            .concurrent_state_mut(store.as_context_mut().0)
            .get_mut(self.id)
            .unwrap();

        if let &WriteState::GuestReady { count, .. } = &transmit.write {
            let &ReadState::HostReady { guest_offset, .. } = &transmit.read else {
                unreachable!()
            };

            count - guest_offset
        } else if let Some(host_buffer) = &self.host_buffer {
            host_buffer.remaining().len()
        } else {
            unreachable!()
        }
    }
}

impl<'a> Source<'a, u8> {
    /// Return a `DirectSource` view of `self`.
    pub fn as_direct<D>(self, store: StoreContextMut<'a, D>) -> DirectSource<'a, D> {
        DirectSource {
            instance: self.instance,
            id: self.id,
            host_buffer: self.host_buffer,
            store,
        }
    }
}

/// Represents a write to a `stream<u8>`, providing direct access to the
/// writer's buffer.
pub struct DirectSource<'a, D: 'static> {
    instance: Instance,
    id: TableId<TransmitState>,
    host_buffer: Option<&'a mut dyn WriteBuffer<u8>>,
    store: StoreContextMut<'a, D>,
}

impl<D: 'static> DirectSource<'_, D> {
    /// Provide direct access to the writer's buffer.
    pub fn remaining(&mut self) -> &[u8] {
        if let Some(buffer) = self.host_buffer.as_deref_mut() {
            buffer.remaining()
        } else {
            let transmit = self
                .instance
                .concurrent_state_mut(self.store.as_context_mut().0)
                .get_mut(self.id)
                .unwrap();

            let &WriteState::GuestReady {
                address,
                count,
                options,
                ..
            } = &transmit.write
            else {
                unreachable!()
            };

            let &ReadState::HostReady { guest_offset, .. } = &transmit.read else {
                unreachable!()
            };

            options
                .memory(self.store.0)
                .get((address + guest_offset)..)
                .and_then(|b| b.get(..(count - guest_offset)))
                .unwrap()
        }
    }

    /// Mark the specified number of bytes as read from the writer's buffer.
    ///
    /// This will panic if the count is larger than the size of the buffer
    /// returned by `Self::remaining`.
    pub fn mark_read(&mut self, count: usize) {
        if let Some(buffer) = self.host_buffer.as_deref_mut() {
            buffer.skip(count);
        } else {
            let transmit = self
                .instance
                .concurrent_state_mut(self.store.as_context_mut().0)
                .get_mut(self.id)
                .unwrap();

            let WriteState::GuestReady {
                count: write_count, ..
            } = &transmit.write
            else {
                unreachable!()
            };

            let ReadState::HostReady { guest_offset, .. } = &mut transmit.read else {
                unreachable!()
            };

            if *guest_offset + count > *write_count {
                panic!(
                    "read count ({count}) must be less than or equal to write count ({write_count})"
                )
            } else {
                *guest_offset += count;
            }
        }
    }
}

/// Represents the host-owned read end of a stream.
pub trait StreamConsumer<D>: Send + 'static {
    /// The payload type of this stream.
    type Item;

    /// Handle a host- or guest-initiated write by accepting zero or more items
    /// from the specified source.
    ///
    /// This will be called whenever the writer starts a write.
    ///
    /// If the implementation is able to consume one or more items immediately,
    /// it should take them from `source` and return either
    /// `Poll::Ready(Ok(StreamResult::Completed))` if it expects to be able to consume
    /// more items, or `Poll::Ready(Ok(StreamResult::Dropped))` if it cannot
    /// accept any more items.  Alternatively, it may return `Poll::Pending` to
    /// indicate that the caller should delay sending a `COMPLETED` event to the
    /// writer until a later call to this function returns `Poll::Ready(_)`.
    /// For more about that, see the `Backpressure` section below.
    ///
    /// If the implementation cannot consume any items immediately and `finish`
    /// is _false_, it should store the waker from `cx` for later and return
    /// `Poll::Pending` without writing anything to `destination`.  Later, it
    /// should alert the waker when either (1) the items arrive, (2) the stream
    /// has ended, or (3) an error occurs.
    ///
    /// If the implementation cannot consume any items immediately and `finish`
    /// is _true_, it should, if possible, return
    /// `Poll::Ready(Ok(StreamResult::Cancelled))` immediately without taking
    /// anything from `source`.  However, that might not be possible if an
    /// earlier call to `poll_consume` kicked off an asynchronous operation
    /// which needs to be completed (and possibly interrupted) gracefully, in
    /// which case the implementation may return `Poll::Pending` and later alert
    /// the waker as described above.  In other words, when `finish` is true,
    /// the implementation should prioritize returning a result to the reader
    /// (even if no items can be consumed) rather than wait indefinitely for at
    /// capacity to free up.
    ///
    /// In all of the above cases, the implementation may alternatively choose
    /// to return `Err(_)` to indicate an unrecoverable error.  This will cause
    /// the guest (if any) to trap and render the component instance (if any)
    /// unusable.  The implementation should report errors that _are_
    /// recoverable by other means (e.g. by writing to a `future`) and return
    /// `Poll::Ready(Ok(StreamResult::Dropped))`.
    ///
    /// Note that the implementation should only return
    /// `Poll::Ready(Ok(StreamResult::Cancelled))` without having taken any
    /// items from `source` if called with `finish` set to true.  If it does so
    /// when `finish` is false, the caller will trap.  Additionally, it should
    /// only return `Poll::Ready(Ok(StreamResult::Completed))` after taking at
    /// least one item from `source` if there is an item available; otherwise,
    /// the caller will trap.  If `poll_consume` is called with no items in
    /// `source`, it should only return `Poll::Ready(_)` once it is able to
    /// accept at least one item during the next call to `poll_consume`.
    ///
    /// Note that any items which the implementation of this trait takes from
    /// `source` become the responsibility of that implementation.  For that
    /// reason, an implementation which forwards items to an upstream sink
    /// should reserve capacity in that sink before taking items out of
    /// `source`, if possible.  Alternatively, it might buffer items which can't
    /// be forwarded immediately and send them once capacity is freed up.
    ///
    /// ## Backpressure
    ///
    /// As mentioned above, an implementation might choose to return
    /// `Poll::Pending` after taking items from `source`, which tells the caller
    /// to delay sending a `COMPLETED` event to the writer.  This can be used as
    /// a form of backpressure when the items are forwarded to an upstream sink
    /// asynchronously.  Note, however, that it's not possible to "put back"
    /// items into `source` once they've been taken out, so if the upstream sink
    /// is unable to accept all the items, that cannot be communicated to the
    /// writer at this level of abstraction.  Just as with application-specific,
    /// recoverable errors, information about which items could be forwarded and
    /// which could not must be communicated out-of-band, e.g. by writing to an
    /// application-specific `future`.
    ///
    /// Similarly, if the writer cancels the write after items have been taken
    /// from `source` but before the items have all been forwarded to an
    /// upstream sink, `poll_consume` will be called with `finish` set to true,
    /// and the implementation may either:
    ///
    /// - Interrupt the forwarding process gracefully.  This may be preferable
    /// if there is an out-of-band channel for communicating to the writer how
    /// many items were forwarded before being interrupted.
    ///
    /// - Allow the forwarding to complete without interrupting it.  This is
    /// usually preferable if there's no out-of-band channel for reporting back
    /// to the writer how many items were forwarded.
    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<StreamResult>>;
}

/// Represents a host-owned write end of a future.
pub trait FutureProducer<D>: Send + 'static {
    /// The payload type of this future.
    type Item;

    /// Handle a host- or guest-initiated read by producing a value.
    ///
    /// This is equivalent to `StreamProducer::poll_produce`, but with a
    /// simplified interface for futures.
    ///
    /// If `finish` is true, the implementation may return
    /// `Poll::Ready(Ok(None))` to indicate the operation was canceled before it
    /// could produce a value.  Otherwise, it must either return
    /// `Poll::Ready(Ok(Some(_)))`, `Poll::Ready(Err(_))`, or `Poll::Pending`.
    fn poll_produce(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        finish: bool,
    ) -> Poll<Result<Option<Self::Item>>>;
}

impl<T, E, D, Fut> FutureProducer<D> for Fut
where
    E: Into<Error>,
    Fut: Future<Output = Result<T, E>> + ?Sized + Send + 'static,
{
    type Item = T;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        finish: bool,
    ) -> Poll<Result<Option<T>>> {
        match self.poll(cx) {
            Poll::Ready(Ok(v)) => Poll::Ready(Ok(Some(v))),
            Poll::Ready(Err(err)) => Poll::Ready(Err(err.into())),
            Poll::Pending if finish => Poll::Ready(Ok(None)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Represents a host-owned read end of a future.
pub trait FutureConsumer<D>: Send + 'static {
    /// The payload type of this future.
    type Item;

    /// Handle a host- or guest-initiated write by consuming a value.
    ///
    /// This is equivalent to `StreamProducer::poll_produce`, but with a
    /// simplified interface for futures.
    ///
    /// If `finish` is true, the implementation may return `Poll::Ready(Ok(()))`
    /// without taking the item from `source`, which indicates the operation was
    /// canceled before it could consume the value.  Otherwise, it must either
    /// take the item from `source` and return `Poll::Ready(Ok(()))`, or else
    /// return `Poll::Ready(Err(_))` or `Poll::Pending` (with or without taking
    /// the item).
    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<()>>;
}

/// Represents the readable end of a Component Model `future`.
///
/// Note that `FutureReader` instances must be disposed of using either `pipe`
/// or `close`; otherwise the in-store representation will leak and the writer
/// end will hang indefinitely.  Consider using [`GuardedFutureReader`] to
/// ensure that disposal happens automatically.
pub struct FutureReader<T> {
    instance: Instance,
    id: TableId<TransmitHandle>,
    _phantom: PhantomData<T>,
}

impl<T> FutureReader<T> {
    /// Create a new future with the specified producer.
    pub fn new<S: AsContextMut>(
        instance: Instance,
        mut store: S,
        producer: impl FutureProducer<S::Data, Item = T>,
    ) -> Self
    where
        T: func::Lower + func::Lift + Send + Sync + 'static,
    {
        struct Producer<P>(P);

        impl<D, T: func::Lower + 'static, P: FutureProducer<D, Item = T>> StreamProducer<D>
            for Producer<P>
        {
            type Item = P::Item;
            type Buffer = Option<P::Item>;

            fn poll_produce<'a>(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                store: StoreContextMut<D>,
                mut destination: Destination<'a, Self::Item, Self::Buffer>,
                finish: bool,
            ) -> Poll<Result<StreamResult>> {
                // SAFETY: This is a standard pin-projection, and we never move
                // out of `self`.
                let producer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };

                Poll::Ready(Ok(
                    if let Some(value) = ready!(producer.poll_produce(cx, store, finish))? {
                        destination.set_buffer(Some(value));

                        // Here we return `StreamResult::Completed` even though
                        // we've produced the last item we'll ever produce.
                        // That's because the ABI expects
                        // `ReturnCode::Completed(1)` rather than
                        // `ReturnCode::Dropped(1)`.  In any case, we won't be
                        // called again since the future will have resolved.
                        StreamResult::Completed
                    } else {
                        StreamResult::Cancelled
                    },
                ))
            }
        }

        Self::new_(
            instance.new_transmit(
                store.as_context_mut(),
                TransmitKind::Future,
                Producer(producer),
            ),
            instance,
        )
    }

    fn new_(id: TableId<TransmitHandle>, instance: Instance) -> Self {
        Self {
            instance,
            id,
            _phantom: PhantomData,
        }
    }

    /// Set the consumer that accepts the result of this future.
    pub fn pipe<S: AsContextMut>(
        self,
        store: S,
        consumer: impl FutureConsumer<S::Data, Item = T> + Unpin,
    ) where
        T: func::Lift + 'static,
    {
        struct Consumer<C>(C);

        impl<D: 'static, T: func::Lift + 'static, C: FutureConsumer<D, Item = T>> StreamConsumer<D>
            for Consumer<C>
        {
            type Item = T;

            fn poll_consume(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                mut store: StoreContextMut<D>,
                mut source: Source<Self::Item>,
                finish: bool,
            ) -> Poll<Result<StreamResult>> {
                // SAFETY: This is a standard pin-projection, and we never move
                // out of `self`.
                let consumer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };

                ready!(consumer.poll_consume(
                    cx,
                    store.as_context_mut(),
                    source.reborrow(),
                    finish
                ))?;

                Poll::Ready(Ok(if source.remaining(store) == 0 {
                    // Here we return `StreamResult::Completed` even though
                    // we've consumed the last item we'll ever consume.  That's
                    // because the ABI expects `ReturnCode::Completed(1)` rather
                    // than `ReturnCode::Dropped(1)`.  In any case, we won't be
                    // called again since the future will have resolved.
                    StreamResult::Completed
                } else {
                    StreamResult::Cancelled
                }))
            }
        }

        self.instance
            .set_consumer(store, self.id, TransmitKind::Future, Consumer(consumer));
    }

    /// Convert this `FutureReader` into a [`Val`].
    // See TODO comment for `FutureAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        Val::Future(FutureAny(self.id.rep()))
    }

    /// Attempt to convert the specified [`Val`] to a `FutureReader`.
    pub fn from_val(
        mut store: impl AsContextMut<Data: Send>,
        instance: Instance,
        value: &Val,
    ) -> Result<Self> {
        let Val::Future(FutureAny(rep)) = value else {
            bail!("expected `future`; got `{}`", value.desc());
        };
        let store = store.as_context_mut();
        let id = TableId::<TransmitHandle>::new(*rep);
        instance.concurrent_state_mut(store.0).get_mut(id)?; // Just make sure it's present
        Ok(Self::new_(id, instance))
    }

    /// Transfer ownership of the read end of a future from a guest to the host.
    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Future(src) => {
                let handle_table = cx
                    .instance_mut()
                    .table_for_transmit(TransmitIndex::Future(src));
                let (rep, is_done) = handle_table.future_remove_readable(src, index)?;
                if is_done {
                    bail!("cannot lift future after being notified that the writable end dropped");
                }
                let id = TableId::<TransmitHandle>::new(rep);
                let concurrent_state = cx.instance_mut().concurrent_state_mut();
                let future = concurrent_state.get_mut(id)?;
                future.common.handle = None;
                let state = future.state;

                if concurrent_state.get_mut(state)?.done {
                    bail!("cannot lift future after previous read succeeded");
                }

                Ok(Self::new_(id, cx.instance_handle()))
            }
            _ => func::bad_type_info(),
        }
    }

    /// Close this `FutureReader`, writing the default value.
    ///
    /// # Panics
    ///
    /// Panics if the store that the [`Accessor`] is derived from does not own
    /// this future. Usage of this future after calling `close` will also cause
    /// a panic.
    pub fn close(&mut self, mut store: impl AsContextMut) {
        // `self` should never be used again, but leave an invalid handle there just in case.
        let id = mem::replace(&mut self.id, TableId::new(u32::MAX));
        self.instance
            .host_drop_reader(store.as_context_mut().0, id, TransmitKind::Future)
            .unwrap();
    }

    /// Convenience method around [`Self::close`].
    pub fn close_with(&mut self, accessor: impl AsAccessor) {
        accessor.as_accessor().with(|access| self.close(access))
    }

    /// Returns a [`GuardedFutureReader`] which will auto-close this future on
    /// drop and clean it up from the store.
    ///
    /// Note that the `accessor` provided must own this future and is
    /// additionally transferred to the `GuardedFutureReader` return value.
    pub fn guard<A>(self, accessor: A) -> GuardedFutureReader<T, A>
    where
        A: AsAccessor,
    {
        GuardedFutureReader::new(accessor, self)
    }
}

impl<T> fmt::Debug for FutureReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureReader")
            .field("id", &self.id)
            .field("instance", &self.instance)
            .finish()
    }
}

/// Transfer ownership of the read end of a future from the host to a guest.
pub(crate) fn lower_future_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    match ty {
        InterfaceType::Future(dst) => {
            let concurrent_state = cx.instance_mut().concurrent_state_mut();
            let id = TableId::<TransmitHandle>::new(rep);
            let state = concurrent_state.get_mut(id)?.state;
            let rep = concurrent_state.get_mut(state)?.read_handle.rep();

            let handle = cx
                .instance_mut()
                .table_for_transmit(TransmitIndex::Future(dst))
                .future_insert_read(dst, rep)?;

            cx.instance_mut()
                .concurrent_state_mut()
                .get_mut(id)?
                .common
                .handle = Some(handle);

            Ok(handle)
        }
        _ => func::bad_type_info(),
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower future handles as `u32`s.
unsafe impl<T: Send + Sync> func::ComponentType for FutureReader<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Future(_) => Ok(()),
            other => bail!("expected `future`, found `{}`", func::desc(other)),
        }
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lower for FutureReader<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        lower_future_to_index(self.id.rep(), cx, ty)?.linear_lower_to_flat(
            cx,
            InterfaceType::U32,
            dst,
        )
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        lower_future_to_index(self.id.rep(), cx, ty)?.linear_lower_to_memory(
            cx,
            InterfaceType::U32,
            offset,
        )
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lift for FutureReader<T> {
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

/// A [`FutureReader`] paired with an [`Accessor`].
///
/// This is an RAII wrapper around [`FutureReader`] that ensures it is closed
/// when dropped. This can be created through [`GuardedFutureReader::new`] or
/// [`FutureReader::guard`].
pub struct GuardedFutureReader<T, A>
where
    A: AsAccessor,
{
    // This field is `None` to implement the conversion from this guard back to
    // `FutureReader`. When `None` is seen in the destructor it will cause the
    // destructor to do nothing.
    reader: Option<FutureReader<T>>,
    accessor: A,
}

impl<T, A> GuardedFutureReader<T, A>
where
    A: AsAccessor,
{
    /// Create a new `GuardedFutureReader` with the specified `accessor` and `reader`.
    pub fn new(accessor: A, reader: FutureReader<T>) -> Self {
        Self {
            reader: Some(reader),
            accessor,
        }
    }

    /// Extracts the underlying [`FutureReader`] from this guard, returning it
    /// back.
    pub fn into_future(self) -> FutureReader<T> {
        self.into()
    }
}

impl<T, A> From<GuardedFutureReader<T, A>> for FutureReader<T>
where
    A: AsAccessor,
{
    fn from(mut guard: GuardedFutureReader<T, A>) -> Self {
        guard.reader.take().unwrap()
    }
}

impl<T, A> Drop for GuardedFutureReader<T, A>
where
    A: AsAccessor,
{
    fn drop(&mut self) {
        if let Some(reader) = &mut self.reader {
            reader.close_with(&self.accessor)
        }
    }
}

/// Represents the readable end of a Component Model `stream`.
///
/// Note that `StreamReader` instances must be disposed of using `close`;
/// otherwise the in-store representation will leak and the writer end will hang
/// indefinitely.  Consider using [`GuardedStreamReader`] to ensure that
/// disposal happens automatically.
pub struct StreamReader<T> {
    instance: Instance,
    id: TableId<TransmitHandle>,
    _phantom: PhantomData<T>,
}

impl<T> StreamReader<T> {
    /// Create a new stream with the specified producer.
    pub fn new<S: AsContextMut>(
        instance: Instance,
        store: S,
        producer: impl StreamProducer<S::Data, Item = T>,
    ) -> Self
    where
        T: func::Lower + func::Lift + Send + Sync + 'static,
    {
        Self::new_(
            instance.new_transmit(store, TransmitKind::Stream, producer),
            instance,
        )
    }

    fn new_(id: TableId<TransmitHandle>, instance: Instance) -> Self {
        Self {
            instance,
            id,
            _phantom: PhantomData,
        }
    }

    /// Set the consumer that accepts the items delivered to this stream.
    pub fn pipe<S: AsContextMut>(self, store: S, consumer: impl StreamConsumer<S::Data, Item = T>)
    where
        T: 'static,
    {
        self.instance
            .set_consumer(store, self.id, TransmitKind::Stream, consumer);
    }

    /// Convert this `StreamReader` into a [`Val`].
    // See TODO comment for `StreamAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        Val::Stream(StreamAny(self.id.rep()))
    }

    /// Attempt to convert the specified [`Val`] to a `StreamReader`.
    pub fn from_val(
        mut store: impl AsContextMut<Data: Send>,
        instance: Instance,
        value: &Val,
    ) -> Result<Self> {
        let Val::Stream(StreamAny(rep)) = value else {
            bail!("expected `stream`; got `{}`", value.desc());
        };
        let store = store.as_context_mut();
        let id = TableId::<TransmitHandle>::new(*rep);
        instance.concurrent_state_mut(store.0).get_mut(id)?; // Just make sure it's present
        Ok(Self::new_(id, instance))
    }

    /// Transfer ownership of the read end of a stream from a guest to the host.
    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Stream(src) => {
                let handle_table = cx
                    .instance_mut()
                    .table_for_transmit(TransmitIndex::Stream(src));
                let (rep, is_done) = handle_table.stream_remove_readable(src, index)?;
                if is_done {
                    bail!("cannot lift stream after being notified that the writable end dropped");
                }
                let id = TableId::<TransmitHandle>::new(rep);
                cx.instance_mut()
                    .concurrent_state_mut()
                    .get_mut(id)?
                    .common
                    .handle = None;
                Ok(Self::new_(id, cx.instance_handle()))
            }
            _ => func::bad_type_info(),
        }
    }

    /// Close this `StreamReader`, writing the default value.
    ///
    /// # Panics
    ///
    /// Panics if the store that the [`Accessor`] is derived from does not own
    /// this future. Usage of this future after calling `close` will also cause
    /// a panic.
    pub fn close(&mut self, mut store: impl AsContextMut) {
        // `self` should never be used again, but leave an invalid handle there just in case.
        let id = mem::replace(&mut self.id, TableId::new(u32::MAX));
        self.instance
            .host_drop_reader(store.as_context_mut().0, id, TransmitKind::Stream)
            .unwrap()
    }

    /// Convenience method around [`Self::close`].
    pub fn close_with(&mut self, accessor: impl AsAccessor) {
        accessor.as_accessor().with(|access| self.close(access))
    }

    /// Returns a [`GuardedStreamReader`] which will auto-close this stream on
    /// drop and clean it up from the store.
    ///
    /// Note that the `accessor` provided must own this future and is
    /// additionally transferred to the `GuardedStreamReader` return value.
    pub fn guard<A>(self, accessor: A) -> GuardedStreamReader<T, A>
    where
        A: AsAccessor,
    {
        GuardedStreamReader::new(accessor, self)
    }
}

impl<T> fmt::Debug for StreamReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamReader")
            .field("id", &self.id)
            .field("instance", &self.instance)
            .finish()
    }
}

/// Transfer ownership of the read end of a stream from the host to a guest.
pub(crate) fn lower_stream_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    match ty {
        InterfaceType::Stream(dst) => {
            let concurrent_state = cx.instance_mut().concurrent_state_mut();
            let id = TableId::<TransmitHandle>::new(rep);
            let state = concurrent_state.get_mut(id)?.state;
            let rep = concurrent_state.get_mut(state)?.read_handle.rep();

            let handle = cx
                .instance_mut()
                .table_for_transmit(TransmitIndex::Stream(dst))
                .stream_insert_read(dst, rep)?;

            cx.instance_mut()
                .concurrent_state_mut()
                .get_mut(id)?
                .common
                .handle = Some(handle);

            Ok(handle)
        }
        _ => func::bad_type_info(),
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower stream handles as `u32`s.
unsafe impl<T: Send + Sync> func::ComponentType for StreamReader<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Stream(_) => Ok(()),
            other => bail!("expected `stream`, found `{}`", func::desc(other)),
        }
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lower for StreamReader<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        lower_stream_to_index(self.id.rep(), cx, ty)?.linear_lower_to_flat(
            cx,
            InterfaceType::U32,
            dst,
        )
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        lower_stream_to_index(self.id.rep(), cx, ty)?.linear_lower_to_memory(
            cx,
            InterfaceType::U32,
            offset,
        )
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T: Send + Sync> func::Lift for StreamReader<T> {
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

/// A [`StreamReader`] paired with an [`Accessor`].
///
/// This is an RAII wrapper around [`StreamReader`] that ensures it is closed
/// when dropped. This can be created through [`GuardedStreamReader::new`] or
/// [`StreamReader::guard`].
pub struct GuardedStreamReader<T, A>
where
    A: AsAccessor,
{
    // This field is `None` to implement the conversion from this guard back to
    // `StreamReader`. When `None` is seen in the destructor it will cause the
    // destructor to do nothing.
    reader: Option<StreamReader<T>>,
    accessor: A,
}

impl<T, A> GuardedStreamReader<T, A>
where
    A: AsAccessor,
{
    /// Create a new `GuardedStreamReader` with the specified `accessor` and
    /// `reader`.
    pub fn new(accessor: A, reader: StreamReader<T>) -> Self {
        Self {
            reader: Some(reader),
            accessor,
        }
    }

    /// Extracts the underlying [`StreamReader`] from this guard, returning it
    /// back.
    pub fn into_stream(self) -> StreamReader<T> {
        self.into()
    }
}

impl<T, A> From<GuardedStreamReader<T, A>> for StreamReader<T>
where
    A: AsAccessor,
{
    fn from(mut guard: GuardedStreamReader<T, A>) -> Self {
        guard.reader.take().unwrap()
    }
}

impl<T, A> Drop for GuardedStreamReader<T, A>
where
    A: AsAccessor,
{
    fn drop(&mut self) {
        if let Some(reader) = &mut self.reader {
            reader.close_with(&self.accessor)
        }
    }
}

/// Represents a Component Model `error-context`.
pub struct ErrorContext {
    rep: u32,
}

impl ErrorContext {
    pub(crate) fn new(rep: u32) -> Self {
        Self { rep }
    }

    /// Convert this `ErrorContext` into a [`Val`].
    pub fn into_val(self) -> Val {
        Val::ErrorContext(ErrorContextAny(self.rep))
    }

    /// Attempt to convert the specified [`Val`] to a `ErrorContext`.
    pub fn from_val(_: impl AsContextMut, value: &Val) -> Result<Self> {
        let Val::ErrorContext(ErrorContextAny(rep)) = value else {
            bail!("expected `error-context`; got `{}`", value.desc());
        };
        Ok(Self::new(*rep))
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::ErrorContext(src) => {
                let rep = cx
                    .instance_mut()
                    .table_for_error_context(src)
                    .error_context_rep(index)?;

                Ok(Self { rep })
            }
            _ => func::bad_type_info(),
        }
    }
}

pub(crate) fn lower_error_context_to_index<U>(
    rep: u32,
    cx: &mut LowerContext<'_, U>,
    ty: InterfaceType,
) -> Result<u32> {
    match ty {
        InterfaceType::ErrorContext(dst) => {
            let tbl = cx.instance_mut().table_for_error_context(dst);
            tbl.error_context_insert(rep)
        }
        _ => func::bad_type_info(),
    }
}
// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower future handles as `u32`s.
unsafe impl func::ComponentType for ErrorContext {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::ErrorContext(_) => Ok(()),
            other => bail!("expected `error`, found `{}`", func::desc(other)),
        }
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
        lower_error_context_to_index(self.rep, cx, ty)?.linear_lower_to_flat(
            cx,
            InterfaceType::U32,
            dst,
        )
    }

    fn linear_lower_to_memory<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        lower_error_context_to_index(self.rep, cx, ty)?.linear_lower_to_memory(
            cx,
            InterfaceType::U32,
            offset,
        )
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl func::Lift for ErrorContext {
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

/// Represents the read or write end of a stream or future.
pub(super) struct TransmitHandle {
    pub(super) common: WaitableCommon,
    /// See `TransmitState`
    state: TableId<TransmitState>,
}

impl TransmitHandle {
    fn new(state: TableId<TransmitState>) -> Self {
        Self {
            common: WaitableCommon::default(),
            state,
        }
    }
}

impl TableDebug for TransmitHandle {
    fn type_name() -> &'static str {
        "TransmitHandle"
    }
}

/// Represents the state of a stream or future.
struct TransmitState {
    /// The write end of the stream or future.
    write_handle: TableId<TransmitHandle>,
    /// The read end of the stream or future.
    read_handle: TableId<TransmitHandle>,
    /// See `WriteState`
    write: WriteState,
    /// See `ReadState`
    read: ReadState,
    /// Whether futher values may be transmitted via this stream or future.
    done: bool,
}

impl Default for TransmitState {
    fn default() -> Self {
        Self {
            write_handle: TableId::new(u32::MAX),
            read_handle: TableId::new(u32::MAX),
            read: ReadState::Open,
            write: WriteState::Open,
            done: false,
        }
    }
}

impl TableDebug for TransmitState {
    fn type_name() -> &'static str {
        "TransmitState"
    }
}

type PollStream = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<StreamResult>> + Send + 'static>> + Send + Sync,
>;

/// Represents the state of the write end of a stream or future.
enum WriteState {
    /// The write end is open, but no write is pending.
    Open,
    /// The write end is owned by a guest task and a write is pending.
    GuestReady {
        ty: TransmitIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        handle: u32,
    },
    /// The write end is owned by the host, which is ready to produce items.
    HostReady {
        produce: PollStream,
        guest_offset: usize,
        cancel: bool,
        cancel_waker: Option<Waker>,
    },
    /// The write end has been dropped.
    Dropped,
}

impl fmt::Debug for WriteState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Open => f.debug_tuple("Open").finish(),
            Self::GuestReady { .. } => f.debug_tuple("GuestReady").finish(),
            Self::HostReady { .. } => f.debug_tuple("HostReady").finish(),
            Self::Dropped => f.debug_tuple("Dropped").finish(),
        }
    }
}

/// Represents the state of the read end of a stream or future.
enum ReadState {
    /// The read end is open, but no read is pending.
    Open,
    /// The read end is owned by a guest task and a read is pending.
    GuestReady {
        ty: TransmitIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        handle: u32,
    },
    /// The read end is owned by a host task, and it is ready to consume items.
    HostReady {
        consume: PollStream,
        guest_offset: usize,
        cancel: bool,
        cancel_waker: Option<Waker>,
    },
    /// Both the read and write ends are owned by the host.
    HostToHost {
        accept: Box<
            dyn for<'a> Fn(
                    &'a mut UntypedWriteBuffer<'a>,
                )
                    -> Pin<Box<dyn Future<Output = Result<StreamResult>> + Send + 'a>>
                + Send
                + Sync,
        >,
        buffer: Vec<u8>,
        limit: usize,
    },
    /// The read end has been dropped.
    Dropped,
}

impl fmt::Debug for ReadState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Open => f.debug_tuple("Open").finish(),
            Self::GuestReady { .. } => f.debug_tuple("GuestReady").finish(),
            Self::HostReady { .. } => f.debug_tuple("HostReady").finish(),
            Self::HostToHost { .. } => f.debug_tuple("HostToHost").finish(),
            Self::Dropped => f.debug_tuple("Dropped").finish(),
        }
    }
}

fn return_code(kind: TransmitKind, state: StreamResult, guest_offset: usize) -> ReturnCode {
    let count = guest_offset.try_into().unwrap();
    match state {
        StreamResult::Dropped => ReturnCode::Dropped(count),
        StreamResult::Completed => ReturnCode::completed(kind, count),
        StreamResult::Cancelled => ReturnCode::Cancelled(count),
    }
}

impl Instance {
    fn new_transmit<S: AsContextMut, P: StreamProducer<S::Data>>(
        self,
        mut store: S,
        kind: TransmitKind,
        producer: P,
    ) -> TableId<TransmitHandle>
    where
        P::Item: func::Lower,
    {
        let mut store = store.as_context_mut();
        let token = StoreToken::new(store.as_context_mut());
        let state = self.concurrent_state_mut(store.0);
        let (_, read) = state.new_transmit().unwrap();
        let producer = Arc::new(Mutex::new(Some((Box::pin(producer), P::Buffer::default()))));
        let id = state.get_mut(read).unwrap().state;
        let produce = Box::new(move || {
            let producer = producer.clone();
            async move {
                let (mut mine, mut buffer) = producer.lock().unwrap().take().unwrap();

                let (result, cancelled) = if buffer.remaining().is_empty() {
                    future::poll_fn(|cx| {
                        tls::get(|store| {
                            let transmit = self.concurrent_state_mut(store).get_mut(id).unwrap();

                            let &WriteState::HostReady { cancel, .. } = &transmit.write else {
                                unreachable!();
                            };

                            let mut host_buffer =
                                if let ReadState::HostToHost { buffer, .. } = &mut transmit.read {
                                    Some(Cursor::new(mem::take(buffer)))
                                } else {
                                    None
                                };

                            let poll = mine.as_mut().poll_produce(
                                cx,
                                token.as_context_mut(store),
                                Destination {
                                    instance: self,
                                    id,
                                    buffer: &mut buffer,
                                    host_buffer: host_buffer.as_mut(),
                                    _phantom: PhantomData,
                                },
                                cancel,
                            );

                            let transmit = self.concurrent_state_mut(store).get_mut(id).unwrap();

                            let host_offset = if let (
                                Some(host_buffer),
                                ReadState::HostToHost { buffer, limit, .. },
                            ) = (host_buffer, &mut transmit.read)
                            {
                                *limit = usize::try_from(host_buffer.position()).unwrap();
                                *buffer = host_buffer.into_inner();
                                *limit
                            } else {
                                0
                            };

                            {
                                let WriteState::HostReady {
                                    guest_offset,
                                    cancel,
                                    cancel_waker,
                                    ..
                                } = &mut transmit.write
                                else {
                                    unreachable!();
                                };

                                if poll.is_pending() {
                                    if !buffer.remaining().is_empty()
                                        || *guest_offset > 0
                                        || host_offset > 0
                                    {
                                        return Poll::Ready(Err(anyhow!(
                                            "StreamProducer::poll_produce returned Poll::Pending \
                                             after producing at least one item"
                                        )));
                                    }
                                    *cancel_waker = Some(cx.waker().clone());
                                } else {
                                    *cancel_waker = None;
                                    *cancel = false;
                                }
                            }

                            poll.map(|v| v.map(|result| (result, cancel)))
                        })
                    })
                    .await?
                } else {
                    (StreamResult::Completed, false)
                };

                let (guest_offset, host_offset, count) = tls::get(|store| {
                    let transmit = self.concurrent_state_mut(store).get_mut(id).unwrap();
                    let (count, host_offset) = match &transmit.read {
                        &ReadState::GuestReady { count, .. } => (count, 0),
                        &ReadState::HostToHost { limit, .. } => (1, limit),
                        _ => unreachable!(),
                    };
                    let guest_offset = match &transmit.write {
                        &WriteState::HostReady { guest_offset, .. } => guest_offset,
                        _ => unreachable!(),
                    };
                    (guest_offset, host_offset, count)
                });

                match result {
                    StreamResult::Completed => {
                        if count > 1
                            && buffer.remaining().is_empty()
                            && guest_offset == 0
                            && host_offset == 0
                        {
                            bail!(
                                "StreamProducer::poll_produce returned StreamResult::Completed \
                                 without producing any items"
                            );
                        }
                    }
                    StreamResult::Cancelled => {
                        if !cancelled {
                            bail!(
                                "StreamProducer::poll_produce returned StreamResult::Cancelled \
                                 without being given a `finish` parameter value of true"
                            );
                        }
                    }
                    StreamResult::Dropped => {}
                }

                let write_buffer = !buffer.remaining().is_empty() || host_offset > 0;

                *producer.lock().unwrap() = Some((mine, buffer));

                if write_buffer {
                    self.write(token, id, producer, kind).await?;
                }

                Ok(result)
            }
            .boxed()
        });
        state.get_mut(id).unwrap().write = WriteState::HostReady {
            produce,
            guest_offset: 0,
            cancel: false,
            cancel_waker: None,
        };
        read
    }

    fn set_consumer<S: AsContextMut, C: StreamConsumer<S::Data>>(
        self,
        mut store: S,
        id: TableId<TransmitHandle>,
        kind: TransmitKind,
        consumer: C,
    ) {
        let mut store = store.as_context_mut();
        let token = StoreToken::new(store.as_context_mut());
        let state = self.concurrent_state_mut(store.0);
        let id = state.get_mut(id).unwrap().state;
        let transmit = state.get_mut(id).unwrap();
        let consumer = Arc::new(Mutex::new(Some(Box::pin(consumer))));
        let consume_with_buffer = {
            let consumer = consumer.clone();
            async move |mut host_buffer: Option<&mut dyn WriteBuffer<C::Item>>| {
                let mut mine = consumer.lock().unwrap().take().unwrap();

                let host_buffer_remaining_before =
                    host_buffer.as_deref_mut().map(|v| v.remaining().len());

                let (result, cancelled) = future::poll_fn(|cx| {
                    tls::get(|store| {
                        let cancel =
                            match &self.concurrent_state_mut(store).get_mut(id).unwrap().read {
                                &ReadState::HostReady { cancel, .. } => cancel,
                                ReadState::Open => false,
                                _ => unreachable!(),
                            };

                        let poll = mine.as_mut().poll_consume(
                            cx,
                            token.as_context_mut(store),
                            Source {
                                instance: self,
                                id,
                                host_buffer: host_buffer.as_deref_mut(),
                            },
                            cancel,
                        );

                        if let ReadState::HostReady {
                            cancel_waker,
                            cancel,
                            ..
                        } = &mut self.concurrent_state_mut(store).get_mut(id).unwrap().read
                        {
                            if poll.is_pending() {
                                *cancel_waker = Some(cx.waker().clone());
                            } else {
                                *cancel_waker = None;
                                *cancel = false;
                            }
                        }

                        poll.map(|v| v.map(|result| (result, cancel)))
                    })
                })
                .await?;

                let (guest_offset, count) = tls::get(|store| {
                    let transmit = self.concurrent_state_mut(store).get_mut(id).unwrap();
                    (
                        match &transmit.read {
                            &ReadState::HostReady { guest_offset, .. } => guest_offset,
                            ReadState::Open => 0,
                            _ => unreachable!(),
                        },
                        match &transmit.write {
                            &WriteState::GuestReady { count, .. } => count,
                            WriteState::HostReady { .. } => host_buffer_remaining_before.unwrap(),
                            _ => unreachable!(),
                        },
                    )
                });

                match result {
                    StreamResult::Completed => {
                        if count > 0
                            && guest_offset == 0
                            && host_buffer_remaining_before
                                .zip(host_buffer.map(|v| v.remaining().len()))
                                .map(|(before, after)| before == after)
                                .unwrap_or(false)
                        {
                            bail!(
                                "StreamConsumer::poll_consume returned StreamResult::Completed \
                                 without consuming any items"
                            );
                        }

                        if let TransmitKind::Future = kind {
                            tls::get(|store| {
                                self.concurrent_state_mut(store).get_mut(id).unwrap().done = true;
                            });
                        }
                    }
                    StreamResult::Cancelled => {
                        if !cancelled {
                            bail!(
                                "StreamConsumer::poll_consume returned StreamResult::Cancelled \
                                 without being given a `finish` parameter value of true"
                            );
                        }
                    }
                    StreamResult::Dropped => {}
                }

                *consumer.lock().unwrap() = Some(mine);

                Ok(result)
            }
        };
        let consume = {
            let consume = consume_with_buffer.clone();
            Box::new(move || {
                let consume = consume.clone();
                async move { consume(None).await }.boxed()
            })
        };

        match &transmit.write {
            WriteState::Open => {
                transmit.read = ReadState::HostReady {
                    consume,
                    guest_offset: 0,
                    cancel: false,
                    cancel_waker: None,
                };
            }
            WriteState::GuestReady { .. } => {
                let future = consume();
                transmit.read = ReadState::HostReady {
                    consume,
                    guest_offset: 0,
                    cancel: false,
                    cancel_waker: None,
                };
                self.pipe_from_guest(store.0, kind, id, future);
            }
            WriteState::HostReady { .. } => {
                let WriteState::HostReady { produce, .. } = mem::replace(
                    &mut transmit.write,
                    WriteState::HostReady {
                        produce: Box::new(|| unreachable!()),
                        guest_offset: 0,
                        cancel: false,
                        cancel_waker: None,
                    },
                ) else {
                    unreachable!();
                };

                transmit.read = ReadState::HostToHost {
                    accept: Box::new(move |input| {
                        let consume = consume_with_buffer.clone();
                        async move { consume(Some(input.get_mut::<C::Item>())).await }.boxed()
                    }),
                    buffer: Vec::new(),
                    limit: 0,
                };

                let future = async move {
                    loop {
                        if tls::get(|store| {
                            anyhow::Ok(matches!(
                                self.concurrent_state_mut(store).get_mut(id)?.read,
                                ReadState::Dropped
                            ))
                        })? {
                            break Ok(());
                        }

                        match produce().await? {
                            StreamResult::Completed | StreamResult::Cancelled => {}
                            StreamResult::Dropped => break Ok(()),
                        }

                        if let TransmitKind::Future = kind {
                            break Ok(());
                        }
                    }
                }
                .map(move |result| {
                    tls::get(|store| self.concurrent_state_mut(store).delete_transmit(id))?;
                    result
                });

                state.push_future(Box::pin(future));
            }
            WriteState::Dropped => {
                let reader = transmit.read_handle;
                self.host_drop_reader(store.0, reader, kind).unwrap();
            }
        }
    }

    async fn write<D: 'static, P: Send + 'static, T: func::Lower + 'static, B: WriteBuffer<T>>(
        self,
        token: StoreToken<D>,
        id: TableId<TransmitState>,
        pair: Arc<Mutex<Option<(P, B)>>>,
        kind: TransmitKind,
    ) -> Result<()> {
        let (read, guest_offset) = tls::get(|store| {
            let transmit = self.concurrent_state_mut(store).get_mut(id)?;

            let guest_offset = if let &WriteState::HostReady { guest_offset, .. } = &transmit.write
            {
                Some(guest_offset)
            } else {
                None
            };

            anyhow::Ok((
                mem::replace(&mut transmit.read, ReadState::Open),
                guest_offset,
            ))
        })?;

        match read {
            ReadState::GuestReady {
                ty,
                flat_abi,
                options,
                address,
                count,
                handle,
            } => {
                let guest_offset = guest_offset.unwrap();

                if let TransmitKind::Future = kind {
                    tls::get(|store| {
                        self.concurrent_state_mut(store).get_mut(id)?.done = true;
                        anyhow::Ok(())
                    })?;
                }

                let old_remaining = pair.lock().unwrap().as_mut().unwrap().1.remaining().len();
                let accept = {
                    let pair = pair.clone();
                    move |mut store: StoreContextMut<D>| {
                        lower::<T, B, D>(
                            store.as_context_mut(),
                            self,
                            &options,
                            ty,
                            address + (T::SIZE32 * guest_offset),
                            count - guest_offset,
                            &mut pair.lock().unwrap().as_mut().unwrap().1,
                        )?;
                        anyhow::Ok(())
                    }
                };

                if guest_offset < count {
                    if T::MAY_REQUIRE_REALLOC {
                        // For payloads which may require a realloc call, use a
                        // oneshot::channel and background task.  This is
                        // necessary because calling the guest while there are
                        // host embedder frames on the stack is unsound.
                        let (tx, rx) = oneshot::channel();
                        tls::get(move |store| {
                            self.concurrent_state_mut(store).push_high_priority(
                                WorkItem::WorkerFunction(AlwaysMut::new(Box::new(
                                    move |store, _| {
                                        _ = tx.send(accept(token.as_context_mut(store))?);
                                        Ok(())
                                    },
                                ))),
                            )
                        });
                        rx.await?
                    } else {
                        // Optimize flat payloads (i.e. those which do not
                        // require calling the guest's realloc function) by
                        // lowering directly instead of using a oneshot::channel
                        // and background task.
                        tls::get(|store| accept(token.as_context_mut(store)))?
                    };
                }

                tls::get(|store| {
                    let count =
                        old_remaining - pair.lock().unwrap().as_mut().unwrap().1.remaining().len();

                    let transmit = self.concurrent_state_mut(store).get_mut(id)?;

                    let WriteState::HostReady { guest_offset, .. } = &mut transmit.write else {
                        unreachable!();
                    };

                    *guest_offset += count;

                    transmit.read = ReadState::GuestReady {
                        ty,
                        flat_abi,
                        options,
                        address,
                        count,
                        handle,
                    };

                    anyhow::Ok(())
                })?;

                Ok(())
            }

            ReadState::HostToHost {
                accept,
                mut buffer,
                limit,
            } => {
                let mut state = StreamResult::Completed;
                let mut position = 0;

                while !matches!(state, StreamResult::Dropped) && position < limit {
                    let mut slice_buffer = SliceBuffer::new(buffer, position, limit);
                    state = accept(&mut UntypedWriteBuffer::new(&mut slice_buffer)).await?;
                    (buffer, position, _) = slice_buffer.into_parts();
                }

                {
                    let (mine, mut buffer) = pair.lock().unwrap().take().unwrap();

                    while !(matches!(state, StreamResult::Dropped) || buffer.remaining().is_empty())
                    {
                        state = accept(&mut UntypedWriteBuffer::new(&mut buffer)).await?;
                    }

                    *pair.lock().unwrap() = Some((mine, buffer));
                }

                tls::get(|store| {
                    self.concurrent_state_mut(store).get_mut(id)?.read = match state {
                        StreamResult::Dropped => ReadState::Dropped,
                        StreamResult::Completed | StreamResult::Cancelled => {
                            ReadState::HostToHost {
                                accept,
                                buffer,
                                limit: 0,
                            }
                        }
                    };

                    anyhow::Ok(())
                })?;
                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn pipe_from_guest(
        self,
        store: &mut dyn VMStore,
        kind: TransmitKind,
        id: TableId<TransmitState>,
        future: Pin<Box<dyn Future<Output = Result<StreamResult>> + Send + 'static>>,
    ) {
        let future = async move {
            let stream_state = future.await?;
            tls::get(|store| {
                let state = self.concurrent_state_mut(store);
                let transmit = state.get_mut(id)?;
                let ReadState::HostReady {
                    consume,
                    guest_offset,
                    ..
                } = mem::replace(&mut transmit.read, ReadState::Open)
                else {
                    unreachable!();
                };
                let code = return_code(kind, stream_state, guest_offset);
                transmit.read = match stream_state {
                    StreamResult::Dropped => ReadState::Dropped,
                    StreamResult::Completed | StreamResult::Cancelled => ReadState::HostReady {
                        consume,
                        guest_offset: 0,
                        cancel: false,
                        cancel_waker: None,
                    },
                };
                let WriteState::GuestReady { ty, handle, .. } =
                    mem::replace(&mut transmit.write, WriteState::Open)
                else {
                    unreachable!();
                };
                state.send_write_result(ty, id, handle, code)?;
                Ok(())
            })
        };

        self.concurrent_state_mut(store).push_future(future.boxed());
    }

    fn pipe_to_guest(
        self,
        store: &mut dyn VMStore,
        kind: TransmitKind,
        id: TableId<TransmitState>,
        future: Pin<Box<dyn Future<Output = Result<StreamResult>> + Send + 'static>>,
    ) {
        let future = async move {
            let stream_state = future.await?;
            tls::get(|store| {
                let state = self.concurrent_state_mut(store);
                let transmit = state.get_mut(id)?;
                let WriteState::HostReady {
                    produce,
                    guest_offset,
                    ..
                } = mem::replace(&mut transmit.write, WriteState::Open)
                else {
                    unreachable!();
                };
                let code = return_code(kind, stream_state, guest_offset);
                transmit.write = match stream_state {
                    StreamResult::Dropped => WriteState::Dropped,
                    StreamResult::Completed | StreamResult::Cancelled => WriteState::HostReady {
                        produce,
                        guest_offset: 0,
                        cancel: false,
                        cancel_waker: None,
                    },
                };
                let ReadState::GuestReady { ty, handle, .. } =
                    mem::replace(&mut transmit.read, ReadState::Open)
                else {
                    unreachable!();
                };
                state.send_read_result(ty, id, handle, code)?;
                Ok(())
            })
        };

        self.concurrent_state_mut(store).push_future(future.boxed());
    }

    /// Drop the read end of a stream or future read from the host.
    fn host_drop_reader(
        self,
        store: &mut dyn VMStore,
        id: TableId<TransmitHandle>,
        kind: TransmitKind,
    ) -> Result<()> {
        let transmit_id = self.concurrent_state_mut(store).get_mut(id)?.state;
        let state = self.concurrent_state_mut(store);
        let transmit = state
            .get_mut(transmit_id)
            .with_context(|| format!("error closing reader {transmit_id:?}"))?;
        log::trace!(
            "host_drop_reader state {transmit_id:?}; read state {:?} write state {:?}",
            transmit.read,
            transmit.write
        );

        transmit.read = ReadState::Dropped;

        // If the write end is already dropped, it should stay dropped,
        // otherwise, it should be opened.
        let new_state = if let WriteState::Dropped = &transmit.write {
            WriteState::Dropped
        } else {
            WriteState::Open
        };

        let write_handle = transmit.write_handle;

        match mem::replace(&mut transmit.write, new_state) {
            // If a guest is waiting to write, notify it that the read end has
            // been dropped.
            WriteState::GuestReady { ty, handle, .. } => {
                state.update_event(
                    write_handle.rep(),
                    match ty {
                        TransmitIndex::Future(ty) => Event::FutureWrite {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                        TransmitIndex::Stream(ty) => Event::StreamWrite {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                    },
                )?;
            }

            WriteState::HostReady { .. } => {}

            WriteState::Open => {
                state.update_event(
                    write_handle.rep(),
                    match kind {
                        TransmitKind::Future => Event::FutureWrite {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                        TransmitKind::Stream => Event::StreamWrite {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                    },
                )?;
            }

            WriteState::Dropped => {
                log::trace!("host_drop_reader delete {transmit_id:?}");
                state.delete_transmit(transmit_id)?;
            }
        }
        Ok(())
    }

    /// Drop the write end of a stream or future read from the host.
    fn host_drop_writer<U>(
        self,
        store: StoreContextMut<U>,
        id: TableId<TransmitHandle>,
        on_drop_open: Option<fn() -> Result<()>>,
    ) -> Result<()> {
        let transmit_id = self.concurrent_state_mut(store.0).get_mut(id)?.state;
        let transmit = self
            .concurrent_state_mut(store.0)
            .get_mut(transmit_id)
            .with_context(|| format!("error closing writer {transmit_id:?}"))?;
        log::trace!(
            "host_drop_writer state {transmit_id:?}; write state {:?} read state {:?}",
            transmit.read,
            transmit.write
        );

        // Existing queued transmits must be updated with information for the impending writer closure
        match &mut transmit.write {
            WriteState::GuestReady { .. } => {
                unreachable!("can't call `host_drop_writer` on a guest-owned writer");
            }
            WriteState::HostReady { .. } => {}
            v @ WriteState::Open => {
                if let (Some(on_drop_open), false) = (
                    on_drop_open,
                    transmit.done || matches!(transmit.read, ReadState::Dropped),
                ) {
                    on_drop_open()?;
                } else {
                    *v = WriteState::Dropped;
                }
            }
            WriteState::Dropped => unreachable!("write state is already dropped"),
        }

        let transmit = self.concurrent_state_mut(store.0).get_mut(transmit_id)?;

        // If the existing read state is dropped, then there's nothing to read
        // and we can keep it that way.
        //
        // If the read state was any other state, then we must set the new state to open
        // to indicate that there *is* data to be read
        let new_state = if let ReadState::Dropped = &transmit.read {
            ReadState::Dropped
        } else {
            ReadState::Open
        };

        let read_handle = transmit.read_handle;

        // Swap in the new read state
        match mem::replace(&mut transmit.read, new_state) {
            // If the guest was ready to read, then we cannot drop the reader (or writer);
            // we must deliver the event, and update the state associated with the handle to
            // represent that a read must be performed
            ReadState::GuestReady { ty, handle, .. } => {
                // Ensure the final read of the guest is queued, with appropriate closure indicator
                self.concurrent_state_mut(store.0).update_event(
                    read_handle.rep(),
                    match ty {
                        TransmitIndex::Future(ty) => Event::FutureRead {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                        TransmitIndex::Stream(ty) => Event::StreamRead {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                    },
                )?;
            }

            ReadState::HostReady { .. } | ReadState::HostToHost { .. } => {}

            // If the read state is open, then there are no registered readers of the stream/future
            ReadState::Open => {
                self.concurrent_state_mut(store.0).update_event(
                    read_handle.rep(),
                    match on_drop_open {
                        Some(_) => Event::FutureRead {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                        None => Event::StreamRead {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                    },
                )?;
            }

            // If the read state was already dropped, then we can remove the transmit state completely
            // (both writer and reader have been dropped)
            ReadState::Dropped => {
                log::trace!("host_drop_writer delete {transmit_id:?}");
                self.concurrent_state_mut(store.0)
                    .delete_transmit(transmit_id)?;
            }
        }
        Ok(())
    }

    /// Drop the writable end of the specified stream or future from the guest.
    pub(super) fn guest_drop_writable<T>(
        self,
        store: StoreContextMut<T>,
        ty: TransmitIndex,
        writer: u32,
    ) -> Result<()> {
        let table = self.id().get_mut(store.0).table_for_transmit(ty);
        let transmit_rep = match ty {
            TransmitIndex::Future(ty) => table.future_remove_writable(ty, writer)?,
            TransmitIndex::Stream(ty) => table.stream_remove_writable(ty, writer)?,
        };

        let id = TableId::<TransmitHandle>::new(transmit_rep);
        log::trace!("guest_drop_writable: drop writer {id:?}");
        match ty {
            TransmitIndex::Stream(_) => self.host_drop_writer(store, id, None),
            TransmitIndex::Future(_) => self.host_drop_writer(
                store,
                id,
                Some(|| {
                    Err(anyhow!(
                        "cannot drop future write end without first writing a value"
                    ))
                }),
            ),
        }
    }

    /// Copy `count` items from `read_address` to `write_address` for the
    /// specified stream or future.
    fn copy<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        flat_abi: Option<FlatAbi>,
        write_ty: TransmitIndex,
        write_options: &Options,
        write_address: usize,
        read_ty: TransmitIndex,
        read_options: &Options,
        read_address: usize,
        count: usize,
        rep: u32,
    ) -> Result<()> {
        let types = self.id().get(store.0).component().types().clone();
        match (write_ty, read_ty) {
            (TransmitIndex::Future(write_ty), TransmitIndex::Future(read_ty)) => {
                assert_eq!(count, 1);

                let val = types[types[write_ty].ty]
                    .payload
                    .map(|ty| {
                        let abi = types.canonical_abi(&ty);
                        // FIXME: needs to read an i64 for memory64
                        if write_address % usize::try_from(abi.align32)? != 0 {
                            bail!("write pointer not aligned");
                        }

                        let lift =
                            &mut LiftContext::new(store.0.store_opaque_mut(), write_options, self);
                        let bytes = lift
                            .memory()
                            .get(write_address..)
                            .and_then(|b| b.get(..usize::try_from(abi.size32).unwrap()))
                            .ok_or_else(|| {
                                anyhow::anyhow!("write pointer out of bounds of memory")
                            })?;

                        Val::load(lift, ty, bytes)
                    })
                    .transpose()?;

                if let Some(val) = val {
                    let lower =
                        &mut LowerContext::new(store.as_context_mut(), read_options, &types, self);
                    let ty = types[types[read_ty].ty].payload.unwrap();
                    let ptr = func::validate_inbounds_dynamic(
                        types.canonical_abi(&ty),
                        lower.as_slice_mut(),
                        &ValRaw::u32(read_address.try_into().unwrap()),
                    )?;
                    val.store(lower, ty, ptr)?;
                }
            }
            (TransmitIndex::Stream(write_ty), TransmitIndex::Stream(read_ty)) => {
                if let Some(flat_abi) = flat_abi {
                    // Fast path memcpy for "flat" (i.e. no pointers or handles) payloads:
                    let length_in_bytes = usize::try_from(flat_abi.size).unwrap() * count;
                    if length_in_bytes > 0 {
                        if write_address % usize::try_from(flat_abi.align)? != 0 {
                            bail!("write pointer not aligned");
                        }
                        if read_address % usize::try_from(flat_abi.align)? != 0 {
                            bail!("read pointer not aligned");
                        }

                        let store_opaque = store.0.store_opaque_mut();

                        {
                            let src = write_options
                                .memory(store_opaque)
                                .get(write_address..)
                                .and_then(|b| b.get(..length_in_bytes))
                                .ok_or_else(|| {
                                    anyhow::anyhow!("write pointer out of bounds of memory")
                                })?
                                .as_ptr();
                            let dst = read_options
                                .memory_mut(store_opaque)
                                .get_mut(read_address..)
                                .and_then(|b| b.get_mut(..length_in_bytes))
                                .ok_or_else(|| {
                                    anyhow::anyhow!("read pointer out of bounds of memory")
                                })?
                                .as_mut_ptr();
                            // SAFETY: Both `src` and `dst` have been validated
                            // above.
                            unsafe { src.copy_to(dst, length_in_bytes) };
                        }
                    }
                } else {
                    let store_opaque = store.0.store_opaque_mut();
                    let lift = &mut LiftContext::new(store_opaque, write_options, self);
                    let ty = types[types[write_ty].ty].payload.unwrap();
                    let abi = lift.types.canonical_abi(&ty);
                    let size = usize::try_from(abi.size32).unwrap();
                    if write_address % usize::try_from(abi.align32)? != 0 {
                        bail!("write pointer not aligned");
                    }
                    let bytes = lift
                        .memory()
                        .get(write_address..)
                        .and_then(|b| b.get(..size * count))
                        .ok_or_else(|| anyhow::anyhow!("write pointer out of bounds of memory"))?;

                    let values = (0..count)
                        .map(|index| Val::load(lift, ty, &bytes[(index * size)..][..size]))
                        .collect::<Result<Vec<_>>>()?;

                    let id = TableId::<TransmitHandle>::new(rep);
                    log::trace!("copy values {values:?} for {id:?}");

                    let lower =
                        &mut LowerContext::new(store.as_context_mut(), read_options, &types, self);
                    let ty = types[types[read_ty].ty].payload.unwrap();
                    let abi = lower.types.canonical_abi(&ty);
                    if read_address % usize::try_from(abi.align32)? != 0 {
                        bail!("read pointer not aligned");
                    }
                    let size = usize::try_from(abi.size32).unwrap();
                    lower
                        .as_slice_mut()
                        .get_mut(read_address..)
                        .and_then(|b| b.get_mut(..size * count))
                        .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?;
                    let mut ptr = read_address;
                    for value in values {
                        value.store(lower, ty, ptr)?;
                        ptr += size
                    }
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn check_bounds(
        self,
        store: &StoreOpaque,
        options: &Options,
        ty: TransmitIndex,
        address: usize,
        count: usize,
    ) -> Result<()> {
        let types = self.id().get(store).component().types().clone();
        let size = usize::try_from(
            match ty {
                TransmitIndex::Future(ty) => types[types[ty].ty]
                    .payload
                    .map(|ty| types.canonical_abi(&ty).size32),
                TransmitIndex::Stream(ty) => types[types[ty].ty]
                    .payload
                    .map(|ty| types.canonical_abi(&ty).size32),
            }
            .unwrap_or(0),
        )
        .unwrap();

        if count > 0 && size > 0 {
            options
                .memory(store)
                .get(address..)
                .and_then(|b| b.get(..(size * count)))
                .map(drop)
                .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))
        } else {
            Ok(())
        }
    }

    /// Write to the specified stream or future from the guest.
    pub(super) fn guest_write<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        ty: TransmitIndex,
        options: OptionsIndex,
        flat_abi: Option<FlatAbi>,
        handle: u32,
        address: u32,
        count: u32,
    ) -> Result<ReturnCode> {
        let address = usize::try_from(address).unwrap();
        let count = usize::try_from(count).unwrap();
        let options = Options::new_index(store.0, self, options);
        self.check_bounds(store.0, &options, ty, address, count)?;
        let (rep, state) = self.id().get_mut(store.0).get_mut_by_index(ty, handle)?;
        let TransmitLocalState::Write { done } = *state else {
            bail!(
                "invalid handle {handle}; expected `Write`; got {:?}",
                *state
            );
        };

        if done {
            bail!("cannot write to stream after being notified that the readable end dropped");
        }

        *state = TransmitLocalState::Busy;
        let transmit_handle = TableId::<TransmitHandle>::new(rep);
        let concurrent_state = self.concurrent_state_mut(store.0);
        let transmit_id = concurrent_state.get_mut(transmit_handle)?.state;
        let transmit = concurrent_state.get_mut(transmit_id)?;
        log::trace!(
            "guest_write {count} to {transmit_handle:?} (handle {handle}; state {transmit_id:?}); {:?}",
            transmit.read
        );

        if transmit.done {
            bail!("cannot write to future after previous write succeeded or readable end dropped");
        }

        let new_state = if let ReadState::Dropped = &transmit.read {
            ReadState::Dropped
        } else {
            ReadState::Open
        };

        let set_guest_ready = |me: &mut ConcurrentState| {
            let transmit = me.get_mut(transmit_id)?;
            assert!(matches!(&transmit.write, WriteState::Open));
            transmit.write = WriteState::GuestReady {
                ty,
                flat_abi,
                options,
                address,
                count,
                handle,
            };
            Ok::<_, crate::Error>(())
        };

        let mut result = match mem::replace(&mut transmit.read, new_state) {
            ReadState::GuestReady {
                ty: read_ty,
                flat_abi: read_flat_abi,
                options: read_options,
                address: read_address,
                count: read_count,
                handle: read_handle,
            } => {
                assert_eq!(flat_abi, read_flat_abi);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                // Note that zero-length reads and writes are handling specially
                // by the spec to allow each end to signal readiness to the
                // other.  Quoting the spec:
                //
                // ```
                // The meaning of a read or write when the length is 0 is that
                // the caller is querying the "readiness" of the other
                // side. When a 0-length read/write rendezvous with a
                // non-0-length read/write, only the 0-length read/write
                // completes; the non-0-length read/write is kept pending (and
                // ready for a subsequent rendezvous).
                //
                // In the corner case where a 0-length read and write
                // rendezvous, only the writer is notified of readiness. To
                // avoid livelock, the Canonical ABI requires that a writer must
                // (eventually) follow a completed 0-length write with a
                // non-0-length write that is allowed to block (allowing the
                // reader end to run and rendezvous with its own non-0-length
                // read).
                // ```

                let write_complete = count == 0 || read_count > 0;
                let read_complete = count > 0;
                let read_buffer_remaining = count < read_count;

                let read_handle_rep = transmit.read_handle.rep();

                let count = count.min(read_count);

                self.copy(
                    store.as_context_mut(),
                    flat_abi,
                    ty,
                    &options,
                    address,
                    read_ty,
                    &read_options,
                    read_address,
                    count,
                    rep,
                )?;

                let instance = self.id().get_mut(store.0);
                let types = instance.component().types();
                let item_size = payload(ty, types)
                    .map(|ty| usize::try_from(types.canonical_abi(&ty).size32).unwrap())
                    .unwrap_or(0);
                let concurrent_state = instance.concurrent_state_mut();
                if read_complete {
                    let count = u32::try_from(count).unwrap();
                    let total = if let Some(Event::StreamRead {
                        code: ReturnCode::Completed(old_total),
                        ..
                    }) = concurrent_state.take_event(read_handle_rep)?
                    {
                        count + old_total
                    } else {
                        count
                    };

                    let code = ReturnCode::completed(ty.kind(), total);

                    concurrent_state.send_read_result(read_ty, transmit_id, read_handle, code)?;
                }

                if read_buffer_remaining {
                    let transmit = concurrent_state.get_mut(transmit_id)?;
                    transmit.read = ReadState::GuestReady {
                        ty: read_ty,
                        flat_abi: read_flat_abi,
                        options: read_options,
                        address: read_address + (count * item_size),
                        count: read_count - count,
                        handle: read_handle,
                    };
                }

                if write_complete {
                    ReturnCode::completed(ty.kind(), count.try_into().unwrap())
                } else {
                    set_guest_ready(concurrent_state)?;
                    ReturnCode::Blocked
                }
            }

            ReadState::HostReady {
                consume,
                guest_offset,
                cancel,
                cancel_waker,
            } => {
                assert!(cancel_waker.is_none());
                assert!(!cancel);
                assert_eq!(0, guest_offset);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                set_guest_ready(concurrent_state)?;
                self.consume(store.0, ty.kind(), transmit_id, consume, 0, false)?
            }

            ReadState::HostToHost { .. } => unreachable!(),

            ReadState::Open => {
                set_guest_ready(concurrent_state)?;
                ReturnCode::Blocked
            }

            ReadState::Dropped => {
                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                ReturnCode::Dropped(0)
            }
        };

        if result == ReturnCode::Blocked && !options.async_() {
            result = self.wait_for_write(store.0, transmit_handle)?;
        }

        if result != ReturnCode::Blocked {
            *self.id().get_mut(store.0).get_mut_by_index(ty, handle)?.1 =
                TransmitLocalState::Write {
                    done: matches!(
                        (result, ty),
                        (ReturnCode::Dropped(_), TransmitIndex::Stream(_))
                    ),
                };
        }

        log::trace!(
            "guest_write result for {transmit_handle:?} (handle {handle}; state {transmit_id:?}): {result:?}",
        );

        Ok(result)
    }

    /// Handle a host- or guest-initiated write by delivering the item(s) to the
    /// `StreamConsumer` for the specified stream or future.
    fn consume(
        self,
        store: &mut dyn VMStore,
        kind: TransmitKind,
        transmit_id: TableId<TransmitState>,
        consume: PollStream,
        guest_offset: usize,
        cancel: bool,
    ) -> Result<ReturnCode> {
        let mut future = consume();
        self.concurrent_state_mut(store).get_mut(transmit_id)?.read = ReadState::HostReady {
            consume,
            guest_offset,
            cancel,
            cancel_waker: None,
        };
        let poll = self.set_tls(store, || {
            future
                .as_mut()
                .poll(&mut Context::from_waker(&Waker::noop()))
        });

        Ok(match poll {
            Poll::Ready(state) => {
                let transmit = self.concurrent_state_mut(store).get_mut(transmit_id)?;
                let ReadState::HostReady { guest_offset, .. } = &mut transmit.read else {
                    unreachable!();
                };
                let code = return_code(kind, state?, mem::replace(guest_offset, 0));
                transmit.write = WriteState::Open;
                code
            }
            Poll::Pending => {
                self.pipe_from_guest(store, kind, transmit_id, future);
                ReturnCode::Blocked
            }
        })
    }

    /// Read from the specified stream or future from the guest.
    pub(super) fn guest_read<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        ty: TransmitIndex,
        options: OptionsIndex,
        flat_abi: Option<FlatAbi>,
        handle: u32,
        address: u32,
        count: u32,
    ) -> Result<ReturnCode> {
        let address = usize::try_from(address).unwrap();
        let count = usize::try_from(count).unwrap();
        let options = Options::new_index(store.0, self, options);
        self.check_bounds(store.0, &options, ty, address, count)?;
        let (rep, state) = self.id().get_mut(store.0).get_mut_by_index(ty, handle)?;
        let TransmitLocalState::Read { done } = *state else {
            bail!("invalid handle {handle}; expected `Read`; got {:?}", *state);
        };

        if done {
            bail!("cannot read from stream after being notified that the writable end dropped");
        }

        *state = TransmitLocalState::Busy;
        let transmit_handle = TableId::<TransmitHandle>::new(rep);
        let concurrent_state = self.concurrent_state_mut(store.0);
        let transmit_id = concurrent_state.get_mut(transmit_handle)?.state;
        let transmit = concurrent_state.get_mut(transmit_id)?;
        log::trace!(
            "guest_read {count} from {transmit_handle:?} (handle {handle}; state {transmit_id:?}); {:?}",
            transmit.write
        );

        if transmit.done {
            bail!("cannot read from future after previous read succeeded");
        }

        let new_state = if let WriteState::Dropped = &transmit.write {
            WriteState::Dropped
        } else {
            WriteState::Open
        };

        let set_guest_ready = |me: &mut ConcurrentState| {
            let transmit = me.get_mut(transmit_id)?;
            assert!(matches!(&transmit.read, ReadState::Open));
            transmit.read = ReadState::GuestReady {
                ty,
                flat_abi,
                options,
                address,
                count,
                handle,
            };
            Ok::<_, crate::Error>(())
        };

        let mut result = match mem::replace(&mut transmit.write, new_state) {
            WriteState::GuestReady {
                ty: write_ty,
                flat_abi: write_flat_abi,
                options: write_options,
                address: write_address,
                count: write_count,
                handle: write_handle,
            } => {
                assert_eq!(flat_abi, write_flat_abi);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let write_handle_rep = transmit.write_handle.rep();

                // See the comment in `guest_write` for the
                // `ReadState::GuestReady` case concerning zero-length reads and
                // writes.

                let write_complete = write_count == 0 || count > 0;
                let read_complete = write_count > 0;
                let write_buffer_remaining = count < write_count;

                let count = count.min(write_count);

                self.copy(
                    store.as_context_mut(),
                    flat_abi,
                    write_ty,
                    &write_options,
                    write_address,
                    ty,
                    &options,
                    address,
                    count,
                    rep,
                )?;

                let instance = self.id().get_mut(store.0);
                let types = instance.component().types();
                let item_size = payload(ty, types)
                    .map(|ty| usize::try_from(types.canonical_abi(&ty).size32).unwrap())
                    .unwrap_or(0);
                let concurrent_state = instance.concurrent_state_mut();

                if write_complete {
                    let count = u32::try_from(count).unwrap();
                    let total = if let Some(Event::StreamWrite {
                        code: ReturnCode::Completed(old_total),
                        ..
                    }) = concurrent_state.take_event(write_handle_rep)?
                    {
                        count + old_total
                    } else {
                        count
                    };

                    let code = ReturnCode::completed(ty.kind(), total);

                    concurrent_state.send_write_result(
                        write_ty,
                        transmit_id,
                        write_handle,
                        code,
                    )?;
                }

                if write_buffer_remaining {
                    let transmit = concurrent_state.get_mut(transmit_id)?;
                    transmit.write = WriteState::GuestReady {
                        ty: write_ty,
                        flat_abi: write_flat_abi,
                        options: write_options,
                        address: write_address + (count * item_size),
                        count: write_count - count,
                        handle: write_handle,
                    };
                }

                if read_complete {
                    ReturnCode::completed(ty.kind(), count.try_into().unwrap())
                } else {
                    set_guest_ready(concurrent_state)?;
                    ReturnCode::Blocked
                }
            }

            WriteState::HostReady {
                produce,
                guest_offset,
                cancel,
                cancel_waker,
            } => {
                assert!(cancel_waker.is_none());
                assert!(!cancel);
                assert_eq!(0, guest_offset);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                set_guest_ready(concurrent_state)?;

                self.produce(store.0, ty.kind(), transmit_id, produce, 0, false)?
            }

            WriteState::Open => {
                set_guest_ready(concurrent_state)?;
                ReturnCode::Blocked
            }

            WriteState::Dropped => ReturnCode::Dropped(0),
        };

        if result == ReturnCode::Blocked && !options.async_() {
            result = self.wait_for_read(store.0, transmit_handle)?;
        }

        if result != ReturnCode::Blocked {
            *self.id().get_mut(store.0).get_mut_by_index(ty, handle)?.1 =
                TransmitLocalState::Read {
                    done: matches!(
                        (result, ty),
                        (ReturnCode::Dropped(_), TransmitIndex::Stream(_))
                    ),
                };
        }

        log::trace!(
            "guest_read result for {transmit_handle:?} (handle {handle}; state {transmit_id:?}): {result:?}",
        );

        Ok(result)
    }

    /// Handle a host- or guest-initiated read by polling the `StreamProducer`
    /// for the specified stream or future for items.
    fn produce(
        self,
        store: &mut dyn VMStore,
        kind: TransmitKind,
        transmit_id: TableId<TransmitState>,
        produce: PollStream,
        guest_offset: usize,
        cancel: bool,
    ) -> Result<ReturnCode> {
        let mut future = produce();
        self.concurrent_state_mut(store).get_mut(transmit_id)?.write = WriteState::HostReady {
            produce,
            guest_offset,
            cancel,
            cancel_waker: None,
        };
        let poll = self.set_tls(store, || {
            future
                .as_mut()
                .poll(&mut Context::from_waker(&Waker::noop()))
        });

        Ok(match poll {
            Poll::Ready(state) => {
                let transmit = self.concurrent_state_mut(store).get_mut(transmit_id)?;
                let WriteState::HostReady { guest_offset, .. } = &mut transmit.write else {
                    unreachable!();
                };
                let code = return_code(kind, state?, mem::replace(guest_offset, 0));
                transmit.read = ReadState::Open;
                code
            }
            Poll::Pending => {
                self.pipe_to_guest(store, kind, transmit_id, future);
                ReturnCode::Blocked
            }
        })
    }

    fn wait_for_write(
        self,
        store: &mut dyn VMStore,
        handle: TableId<TransmitHandle>,
    ) -> Result<ReturnCode> {
        let waitable = Waitable::Transmit(handle);
        self.wait_for_event(store, waitable)?;
        let event = waitable.take_event(self.concurrent_state_mut(store))?;
        if let Some(event @ (Event::StreamWrite { code, .. } | Event::FutureWrite { code, .. })) =
            event
        {
            waitable.on_delivery(self.id().get_mut(store), event);
            Ok(code)
        } else {
            unreachable!()
        }
    }

    /// Cancel a pending stream or future write.
    fn cancel_write(
        self,
        store: &mut dyn VMStore,
        transmit_id: TableId<TransmitState>,
        async_: bool,
    ) -> Result<ReturnCode> {
        let state = self.concurrent_state_mut(store);
        let transmit = state.get_mut(transmit_id)?;
        log::trace!(
            "host_cancel_write state {transmit_id:?}; write state {:?} read state {:?}",
            transmit.read,
            transmit.write
        );

        let code = if let Some(event) =
            Waitable::Transmit(transmit.write_handle).take_event(state)?
        {
            let (Event::FutureWrite { code, .. } | Event::StreamWrite { code, .. }) = event else {
                unreachable!();
            };
            match (code, event) {
                (ReturnCode::Completed(count), Event::StreamWrite { .. }) => {
                    ReturnCode::Cancelled(count)
                }
                (ReturnCode::Dropped(_) | ReturnCode::Completed(_), _) => code,
                _ => unreachable!(),
            }
        } else if let ReadState::HostReady {
            cancel,
            cancel_waker,
            ..
        } = &mut state.get_mut(transmit_id)?.read
        {
            *cancel = true;
            if let Some(waker) = cancel_waker.take() {
                waker.wake();
            }

            if async_ {
                ReturnCode::Blocked
            } else {
                let handle = self
                    .concurrent_state_mut(store)
                    .get_mut(transmit_id)?
                    .write_handle;
                self.wait_for_write(store, handle)?
            }
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.concurrent_state_mut(store).get_mut(transmit_id)?;

        match &transmit.write {
            WriteState::GuestReady { .. } => {
                transmit.write = WriteState::Open;
            }
            WriteState::HostReady { .. } => todo!("support host write cancellation"),
            WriteState::Open | WriteState::Dropped => {}
        }

        log::trace!("cancelled write {transmit_id:?}: {code:?}");

        Ok(code)
    }

    fn wait_for_read(
        self,
        store: &mut dyn VMStore,
        handle: TableId<TransmitHandle>,
    ) -> Result<ReturnCode> {
        let waitable = Waitable::Transmit(handle);
        self.wait_for_event(store, waitable)?;
        let event = waitable.take_event(self.concurrent_state_mut(store))?;
        if let Some(event @ (Event::StreamRead { code, .. } | Event::FutureRead { code, .. })) =
            event
        {
            waitable.on_delivery(self.id().get_mut(store), event);
            Ok(code)
        } else {
            unreachable!()
        }
    }

    /// Cancel a pending stream or future read.
    fn cancel_read(
        self,
        store: &mut dyn VMStore,
        transmit_id: TableId<TransmitState>,
        async_: bool,
    ) -> Result<ReturnCode> {
        let state = self.concurrent_state_mut(store);
        let transmit = state.get_mut(transmit_id)?;
        log::trace!(
            "host_cancel_read state {transmit_id:?}; read state {:?} write state {:?}",
            transmit.read,
            transmit.write
        );

        let code = if let Some(event) =
            Waitable::Transmit(transmit.read_handle).take_event(state)?
        {
            let (Event::FutureRead { code, .. } | Event::StreamRead { code, .. }) = event else {
                unreachable!();
            };
            match (code, event) {
                (ReturnCode::Completed(count), Event::StreamRead { .. }) => {
                    ReturnCode::Cancelled(count)
                }
                (ReturnCode::Dropped(_) | ReturnCode::Completed(_), _) => code,
                _ => unreachable!(),
            }
        } else if let WriteState::HostReady {
            cancel,
            cancel_waker,
            ..
        } = &mut state.get_mut(transmit_id)?.write
        {
            *cancel = true;
            if let Some(waker) = cancel_waker.take() {
                waker.wake();
            }

            if async_ {
                ReturnCode::Blocked
            } else {
                let handle = self
                    .concurrent_state_mut(store)
                    .get_mut(transmit_id)?
                    .read_handle;
                self.wait_for_read(store, handle)?
            }
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.concurrent_state_mut(store).get_mut(transmit_id)?;

        match &transmit.read {
            ReadState::GuestReady { .. } => {
                transmit.read = ReadState::Open;
            }
            ReadState::HostReady { .. } | ReadState::HostToHost { .. } => {
                todo!("support host read cancellation")
            }
            ReadState::Open | ReadState::Dropped => {}
        }

        log::trace!("cancelled read {transmit_id:?}: {code:?}");

        Ok(code)
    }

    /// Cancel a pending write for the specified stream or future from the guest.
    fn guest_cancel_write(
        self,
        store: &mut dyn VMStore,
        ty: TransmitIndex,
        async_: bool,
        writer: u32,
    ) -> Result<ReturnCode> {
        let (rep, state) =
            get_mut_by_index_from(self.id().get_mut(store).table_for_transmit(ty), ty, writer)?;
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel write {id:?} (handle {writer})");
        match state {
            TransmitLocalState::Write { .. } => {
                bail!("stream or future write cancelled when no write is pending")
            }
            TransmitLocalState::Read { .. } => {
                bail!("passed read end to `{{stream|future}}.cancel-write`")
            }
            TransmitLocalState::Busy => {}
        }
        let transmit_id = self.concurrent_state_mut(store).get_mut(id)?.state;
        let code = self.cancel_write(store, transmit_id, async_)?;
        if !matches!(code, ReturnCode::Blocked) {
            let state =
                get_mut_by_index_from(self.id().get_mut(store).table_for_transmit(ty), ty, writer)?
                    .1;
            if let TransmitLocalState::Busy = state {
                *state = TransmitLocalState::Write { done: false };
            }
        }
        Ok(code)
    }

    /// Cancel a pending read for the specified stream or future from the guest.
    fn guest_cancel_read(
        self,
        store: &mut dyn VMStore,
        ty: TransmitIndex,
        async_: bool,
        reader: u32,
    ) -> Result<ReturnCode> {
        let (rep, state) =
            get_mut_by_index_from(self.id().get_mut(store).table_for_transmit(ty), ty, reader)?;
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel read {id:?} (handle {reader})");
        match state {
            TransmitLocalState::Read { .. } => {
                bail!("stream or future read cancelled when no read is pending")
            }
            TransmitLocalState::Write { .. } => {
                bail!("passed write end to `{{stream|future}}.cancel-read`")
            }
            TransmitLocalState::Busy => {}
        }
        let transmit_id = self.concurrent_state_mut(store).get_mut(id)?.state;
        let code = self.cancel_read(store, transmit_id, async_)?;
        if !matches!(code, ReturnCode::Blocked) {
            let state =
                get_mut_by_index_from(self.id().get_mut(store).table_for_transmit(ty), ty, reader)?
                    .1;
            if let TransmitLocalState::Busy = state {
                *state = TransmitLocalState::Read { done: false };
            }
        }
        Ok(code)
    }

    /// Drop the readable end of the specified stream or future from the guest.
    fn guest_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TransmitIndex,
        reader: u32,
    ) -> Result<()> {
        let table = self.id().get_mut(store).table_for_transmit(ty);
        let (rep, _is_done) = match ty {
            TransmitIndex::Stream(ty) => table.stream_remove_readable(ty, reader)?,
            TransmitIndex::Future(ty) => table.future_remove_readable(ty, reader)?,
        };
        let kind = match ty {
            TransmitIndex::Stream(_) => TransmitKind::Stream,
            TransmitIndex::Future(_) => TransmitKind::Future,
        };
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest_drop_readable: drop reader {id:?}");
        self.host_drop_reader(store, id, kind)
    }

    /// Create a new error context for the given component.
    pub(crate) fn error_context_new(
        self,
        store: &mut StoreOpaque,
        ty: TypeComponentLocalErrorContextTableIndex,
        options: OptionsIndex,
        debug_msg_address: u32,
        debug_msg_len: u32,
    ) -> Result<u32> {
        let options = Options::new_index(store, self, options);
        let lift_ctx = &mut LiftContext::new(store, &options, self);
        //  Read string from guest memory
        let address = usize::try_from(debug_msg_address)?;
        let len = usize::try_from(debug_msg_len)?;
        lift_ctx
            .memory()
            .get(address..)
            .and_then(|b| b.get(..len))
            .ok_or_else(|| anyhow::anyhow!("invalid debug message pointer: out of bounds"))?;
        let message = WasmStr::new(address, len, lift_ctx)?;

        // Create a new ErrorContext that is tracked along with other concurrent state
        let err_ctx = ErrorContextState {
            debug_msg: message
                .to_str_from_memory(options.memory(store))?
                .to_string(),
        };
        let state = self.concurrent_state_mut(store);
        let table_id = state.push(err_ctx)?;
        let global_ref_count_idx =
            TypeComponentGlobalErrorContextTableIndex::from_u32(table_id.rep());

        // Add to the global error context ref counts
        let _ = state
            .global_error_context_ref_counts
            .insert(global_ref_count_idx, GlobalErrorContextRefCount(1));

        // Error context are tracked both locally (to a single component instance) and globally
        // the counts for both must stay in sync.
        //
        // Here we reflect the newly created global concurrent error context state into the
        // component instance's locally tracked count, along with the appropriate key into the global
        // ref tracking data structures to enable later lookup
        let local_idx = self
            .id()
            .get_mut(store)
            .table_for_error_context(ty)
            .error_context_insert(table_id.rep())?;

        Ok(local_idx)
    }

    /// Retrieve the debug message from the specified error context.
    pub(super) fn error_context_debug_message<T>(
        self,
        store: StoreContextMut<T>,
        ty: TypeComponentLocalErrorContextTableIndex,
        options: OptionsIndex,
        err_ctx_handle: u32,
        debug_msg_address: u32,
    ) -> Result<()> {
        // Retrieve the error context and internal debug message
        let handle_table_id_rep = self
            .id()
            .get_mut(store.0)
            .table_for_error_context(ty)
            .error_context_rep(err_ctx_handle)?;

        let state = self.concurrent_state_mut(store.0);
        // Get the state associated with the error context
        let ErrorContextState { debug_msg } =
            state.get_mut(TableId::<ErrorContextState>::new(handle_table_id_rep))?;
        let debug_msg = debug_msg.clone();

        let options = Options::new_index(store.0, self, options);
        let types = self.id().get(store.0).component().types().clone();
        let lower_cx = &mut LowerContext::new(store, &options, &types, self);
        let debug_msg_address = usize::try_from(debug_msg_address)?;
        // Lower the string into the component's memory
        let offset = lower_cx
            .as_slice_mut()
            .get(debug_msg_address..)
            .and_then(|b| b.get(..debug_msg.bytes().len()))
            .map(|_| debug_msg_address)
            .ok_or_else(|| anyhow::anyhow!("invalid debug message pointer: out of bounds"))?;
        debug_msg
            .as_str()
            .linear_lower_to_memory(lower_cx, InterfaceType::String, offset)?;

        Ok(())
    }

    /// Implements the `future.cancel-read` intrinsic.
    pub(crate) fn future_cancel_read(
        self,
        store: &mut dyn VMStore,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(store, TransmitIndex::Future(ty), async_, reader)
            .map(|v| v.encode())
    }

    /// Implements the `future.cancel-write` intrinsic.
    pub(crate) fn future_cancel_write(
        self,
        store: &mut dyn VMStore,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(store, TransmitIndex::Future(ty), async_, writer)
            .map(|v| v.encode())
    }

    /// Implements the `stream.cancel-read` intrinsic.
    pub(crate) fn stream_cancel_read(
        self,
        store: &mut dyn VMStore,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(store, TransmitIndex::Stream(ty), async_, reader)
            .map(|v| v.encode())
    }

    /// Implements the `stream.cancel-write` intrinsic.
    pub(crate) fn stream_cancel_write(
        self,
        store: &mut dyn VMStore,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(store, TransmitIndex::Stream(ty), async_, writer)
            .map(|v| v.encode())
    }

    /// Implements the `future.drop-readable` intrinsic.
    pub(crate) fn future_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TypeFutureTableIndex,
        reader: u32,
    ) -> Result<()> {
        self.guest_drop_readable(store, TransmitIndex::Future(ty), reader)
    }

    /// Implements the `stream.drop-readable` intrinsic.
    pub(crate) fn stream_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TypeStreamTableIndex,
        reader: u32,
    ) -> Result<()> {
        self.guest_drop_readable(store, TransmitIndex::Stream(ty), reader)
    }
}

impl ComponentInstance {
    fn table_for_transmit(self: Pin<&mut Self>, ty: TransmitIndex) -> &mut HandleTable {
        let (tables, types) = self.guest_tables();
        let runtime_instance = match ty {
            TransmitIndex::Stream(ty) => types[ty].instance,
            TransmitIndex::Future(ty) => types[ty].instance,
        };
        &mut tables[runtime_instance]
    }

    fn table_for_error_context(
        self: Pin<&mut Self>,
        ty: TypeComponentLocalErrorContextTableIndex,
    ) -> &mut HandleTable {
        let (tables, types) = self.guest_tables();
        let runtime_instance = types[ty].instance;
        &mut tables[runtime_instance]
    }

    fn get_mut_by_index(
        self: Pin<&mut Self>,
        ty: TransmitIndex,
        index: u32,
    ) -> Result<(u32, &mut TransmitLocalState)> {
        get_mut_by_index_from(self.table_for_transmit(ty), ty, index)
    }

    /// Allocate a new future or stream and grant ownership of both the read and
    /// write ends to the (sub-)component instance to which the specified
    /// `TransmitIndex` belongs.
    fn guest_new(mut self: Pin<&mut Self>, ty: TransmitIndex) -> Result<ResourcePair> {
        let (write, read) = self.as_mut().concurrent_state_mut().new_transmit()?;

        let table = self.as_mut().table_for_transmit(ty);
        let (read_handle, write_handle) = match ty {
            TransmitIndex::Future(ty) => (
                table.future_insert_read(ty, read.rep())?,
                table.future_insert_write(ty, write.rep())?,
            ),
            TransmitIndex::Stream(ty) => (
                table.stream_insert_read(ty, read.rep())?,
                table.stream_insert_write(ty, write.rep())?,
            ),
        };

        let state = self.as_mut().concurrent_state_mut();
        state.get_mut(read)?.common.handle = Some(read_handle);
        state.get_mut(write)?.common.handle = Some(write_handle);

        Ok(ResourcePair {
            write: write_handle,
            read: read_handle,
        })
    }

    /// Drop the specified error context.
    pub(crate) fn error_context_drop(
        mut self: Pin<&mut Self>,
        ty: TypeComponentLocalErrorContextTableIndex,
        error_context: u32,
    ) -> Result<()> {
        let local_handle_table = self.as_mut().table_for_error_context(ty);

        let rep = local_handle_table.error_context_drop(error_context)?;

        let global_ref_count_idx = TypeComponentGlobalErrorContextTableIndex::from_u32(rep);

        let state = self.concurrent_state_mut();
        let GlobalErrorContextRefCount(global_ref_count) = state
            .global_error_context_ref_counts
            .get_mut(&global_ref_count_idx)
            .expect("retrieve concurrent state for error context during drop");

        // Reduce the component-global ref count, removing tracking if necessary
        assert!(*global_ref_count >= 1);
        *global_ref_count -= 1;
        if *global_ref_count == 0 {
            state
                .global_error_context_ref_counts
                .remove(&global_ref_count_idx);

            state
                .delete(TableId::<ErrorContextState>::new(rep))
                .context("deleting component-global error context data")?;
        }

        Ok(())
    }

    /// Transfer ownership of the specified stream or future read end from one
    /// guest to another.
    fn guest_transfer(
        mut self: Pin<&mut Self>,
        src_idx: u32,
        src: TransmitIndex,
        dst: TransmitIndex,
    ) -> Result<u32> {
        let src_table = self.as_mut().table_for_transmit(src);
        let (rep, is_done) = match src {
            TransmitIndex::Future(idx) => src_table.future_remove_readable(idx, src_idx)?,
            TransmitIndex::Stream(idx) => src_table.stream_remove_readable(idx, src_idx)?,
        };
        if is_done {
            bail!("cannot lift after being notified that the writable end dropped");
        }
        let dst_table = self.as_mut().table_for_transmit(dst);
        let handle = match dst {
            TransmitIndex::Future(idx) => dst_table.future_insert_read(idx, rep),
            TransmitIndex::Stream(idx) => dst_table.stream_insert_read(idx, rep),
        }?;
        self.concurrent_state_mut()
            .get_mut(TableId::<TransmitHandle>::new(rep))?
            .common
            .handle = Some(handle);
        Ok(handle)
    }

    /// Implements the `future.new` intrinsic.
    pub(crate) fn future_new(
        self: Pin<&mut Self>,
        ty: TypeFutureTableIndex,
    ) -> Result<ResourcePair> {
        self.guest_new(TransmitIndex::Future(ty))
    }

    /// Implements the `stream.new` intrinsic.
    pub(crate) fn stream_new(
        self: Pin<&mut Self>,
        ty: TypeStreamTableIndex,
    ) -> Result<ResourcePair> {
        self.guest_new(TransmitIndex::Stream(ty))
    }

    /// Transfer ownership of the specified future read end from one guest to
    /// another.
    pub(crate) fn future_transfer(
        self: Pin<&mut Self>,
        src_idx: u32,
        src: TypeFutureTableIndex,
        dst: TypeFutureTableIndex,
    ) -> Result<u32> {
        self.guest_transfer(
            src_idx,
            TransmitIndex::Future(src),
            TransmitIndex::Future(dst),
        )
    }

    /// Transfer ownership of the specified stream read end from one guest to
    /// another.
    pub(crate) fn stream_transfer(
        self: Pin<&mut Self>,
        src_idx: u32,
        src: TypeStreamTableIndex,
        dst: TypeStreamTableIndex,
    ) -> Result<u32> {
        self.guest_transfer(
            src_idx,
            TransmitIndex::Stream(src),
            TransmitIndex::Stream(dst),
        )
    }

    /// Copy the specified error context from one component to another.
    pub(crate) fn error_context_transfer(
        mut self: Pin<&mut Self>,
        src_idx: u32,
        src: TypeComponentLocalErrorContextTableIndex,
        dst: TypeComponentLocalErrorContextTableIndex,
    ) -> Result<u32> {
        let rep = self
            .as_mut()
            .table_for_error_context(src)
            .error_context_rep(src_idx)?;
        let dst_idx = self
            .as_mut()
            .table_for_error_context(dst)
            .error_context_insert(rep)?;

        // Update the global (cross-subcomponent) count for error contexts
        // as the new component has essentially created a new reference that will
        // be dropped/handled independently
        let global_ref_count = self
            .concurrent_state_mut()
            .global_error_context_ref_counts
            .get_mut(&TypeComponentGlobalErrorContextTableIndex::from_u32(rep))
            .context("global ref count present for existing (sub)component error context")?;
        global_ref_count.0 += 1;

        Ok(dst_idx)
    }
}

impl ConcurrentState {
    fn send_write_result(
        &mut self,
        ty: TransmitIndex,
        id: TableId<TransmitState>,
        handle: u32,
        code: ReturnCode,
    ) -> Result<()> {
        let write_handle = self.get_mut(id)?.write_handle.rep();
        self.set_event(
            write_handle,
            match ty {
                TransmitIndex::Future(ty) => Event::FutureWrite {
                    code,
                    pending: Some((ty, handle)),
                },
                TransmitIndex::Stream(ty) => Event::StreamWrite {
                    code,
                    pending: Some((ty, handle)),
                },
            },
        )
    }

    fn send_read_result(
        &mut self,
        ty: TransmitIndex,
        id: TableId<TransmitState>,
        handle: u32,
        code: ReturnCode,
    ) -> Result<()> {
        let read_handle = self.get_mut(id)?.read_handle.rep();
        self.set_event(
            read_handle,
            match ty {
                TransmitIndex::Future(ty) => Event::FutureRead {
                    code,
                    pending: Some((ty, handle)),
                },
                TransmitIndex::Stream(ty) => Event::StreamRead {
                    code,
                    pending: Some((ty, handle)),
                },
            },
        )
    }

    fn take_event(&mut self, waitable: u32) -> Result<Option<Event>> {
        Waitable::Transmit(TableId::<TransmitHandle>::new(waitable)).take_event(self)
    }

    fn set_event(&mut self, waitable: u32, event: Event) -> Result<()> {
        Waitable::Transmit(TableId::<TransmitHandle>::new(waitable)).set_event(self, Some(event))
    }

    /// Set or update the event for the specified waitable.
    ///
    /// If there is already an event set for this waitable, we assert that it is
    /// of the same variant as the new one and reuse the `ReturnCode` count and
    /// the `pending` field if applicable.
    // TODO: This is a bit awkward due to how
    // `Event::{Stream,Future}{Write,Read}` and
    // `ReturnCode::{Completed,Dropped,Cancelled}` are currently represented.
    // Consider updating those representations in a way that allows this
    // function to be simplified.
    fn update_event(&mut self, waitable: u32, event: Event) -> Result<()> {
        let waitable = Waitable::Transmit(TableId::<TransmitHandle>::new(waitable));

        fn update_code(old: ReturnCode, new: ReturnCode) -> ReturnCode {
            let (ReturnCode::Completed(count)
            | ReturnCode::Dropped(count)
            | ReturnCode::Cancelled(count)) = old
            else {
                unreachable!()
            };

            match new {
                ReturnCode::Dropped(0) => ReturnCode::Dropped(count),
                ReturnCode::Cancelled(0) => ReturnCode::Cancelled(count),
                _ => unreachable!(),
            }
        }

        let event = match (waitable.take_event(self)?, event) {
            (None, _) => event,
            (Some(old @ Event::FutureWrite { .. }), Event::FutureWrite { .. }) => old,
            (Some(old @ Event::FutureRead { .. }), Event::FutureRead { .. }) => old,
            (
                Some(Event::StreamWrite {
                    code: old_code,
                    pending: old_pending,
                }),
                Event::StreamWrite { code, pending },
            ) => Event::StreamWrite {
                code: update_code(old_code, code),
                pending: old_pending.or(pending),
            },
            (
                Some(Event::StreamRead {
                    code: old_code,
                    pending: old_pending,
                }),
                Event::StreamRead { code, pending },
            ) => Event::StreamRead {
                code: update_code(old_code, code),
                pending: old_pending.or(pending),
            },
            _ => unreachable!(),
        };

        waitable.set_event(self, Some(event))
    }

    /// Allocate a new future or stream, including the `TransmitState` and the
    /// `TransmitHandle`s corresponding to the read and write ends.
    fn new_transmit(&mut self) -> Result<(TableId<TransmitHandle>, TableId<TransmitHandle>)> {
        let state_id = self.push(TransmitState::default())?;

        let write = self.push(TransmitHandle::new(state_id))?;
        let read = self.push(TransmitHandle::new(state_id))?;

        let state = self.get_mut(state_id)?;
        state.write_handle = write;
        state.read_handle = read;

        log::trace!("new transmit: state {state_id:?}; write {write:?}; read {read:?}",);

        Ok((write, read))
    }

    /// Delete the specified future or stream, including the read and write ends.
    fn delete_transmit(&mut self, state_id: TableId<TransmitState>) -> Result<()> {
        let state = self.delete(state_id)?;
        self.delete(state.write_handle)?;
        self.delete(state.read_handle)?;

        log::trace!(
            "delete transmit: state {state_id:?}; write {:?}; read {:?}",
            state.write_handle,
            state.read_handle,
        );

        Ok(())
    }
}

pub(crate) struct ResourcePair {
    pub(crate) write: u32,
    pub(crate) read: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, Store};
    use core::future::pending;
    use core::pin::pin;
    use std::sync::LazyLock;

    static ENGINE: LazyLock<Engine> = LazyLock::new(Engine::default);

    fn poll_future_producer<T>(rx: Pin<&mut T>, finish: bool) -> Poll<Result<Option<T::Item>>>
    where
        T: FutureProducer<()>,
    {
        rx.poll_produce(
            &mut Context::from_waker(Waker::noop()),
            Store::new(&ENGINE, ()).as_context_mut(),
            finish,
        )
    }

    #[test]
    fn future_producer() {
        let mut fut = pin!(async { anyhow::Ok(()) });
        assert!(matches!(
            poll_future_producer(fut.as_mut(), false),
            Poll::Ready(Ok(Some(()))),
        ));

        let mut fut = pin!(async { anyhow::Ok(()) });
        assert!(matches!(
            poll_future_producer(fut.as_mut(), true),
            Poll::Ready(Ok(Some(()))),
        ));

        let mut fut = pin!(pending::<Result<()>>());
        assert!(matches!(
            poll_future_producer(fut.as_mut(), false),
            Poll::Pending,
        ));
        assert!(matches!(
            poll_future_producer(fut.as_mut(), true),
            Poll::Ready(Ok(None)),
        ));

        let (tx, rx) = oneshot::channel();
        let mut rx = pin!(rx);
        assert!(matches!(
            poll_future_producer(rx.as_mut(), false),
            Poll::Pending,
        ));
        assert!(matches!(
            poll_future_producer(rx.as_mut(), true),
            Poll::Ready(Ok(None)),
        ));
        tx.send(()).unwrap();
        assert!(matches!(
            poll_future_producer(rx.as_mut(), true),
            Poll::Ready(Ok(Some(()))),
        ));

        let (tx, rx) = oneshot::channel();
        let mut rx = pin!(rx);
        tx.send(()).unwrap();
        assert!(matches!(
            poll_future_producer(rx.as_mut(), false),
            Poll::Ready(Ok(Some(()))),
        ));

        let (tx, rx) = oneshot::channel::<()>();
        let mut rx = pin!(rx);
        drop(tx);
        assert!(matches!(
            poll_future_producer(rx.as_mut(), false),
            Poll::Ready(Err(..)),
        ));

        let (tx, rx) = oneshot::channel::<()>();
        let mut rx = pin!(rx);
        drop(tx);
        assert!(matches!(
            poll_future_producer(rx.as_mut(), true),
            Poll::Ready(Err(..)),
        ));
    }
}
