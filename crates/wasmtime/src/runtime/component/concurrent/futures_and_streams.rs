use super::table::{TableDebug, TableId};
use super::{Event, GlobalErrorContextRefCount, Waitable, WaitableCommon};
use crate::component::concurrent::{Accessor, ConcurrentState, JoinHandle, WorkItem, tls};
use crate::component::func::{self, LiftContext, LowerContext, Options};
use crate::component::matching::InstanceType;
use crate::component::values::{ErrorContextAny, FutureAny, StreamAny};
use crate::component::{AsAccessor, Instance, Lower, Val, WasmList, WasmStr};
use crate::store::{StoreOpaque, StoreToken};
use crate::vm::VMStore;
use crate::vm::component::{ComponentInstance, HandleTable, TransmitLocalState};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{Context as _, Result, anyhow, bail};
use buffers::Extender;
use buffers::UntypedWriteBuffer;
use futures::FutureExt;
use futures::channel::oneshot;
use std::boxed::Box;
use std::fmt;
use std::iter;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::pin::Pin;
use std::string::{String, ToString};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
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
pub struct Destination<T> {
    instance: Instance,
    kind: TransmitKind,
    id: TableId<TransmitState>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Destination<T> {
    /// Deliver zero or more items to the reader.
    pub async fn write<B, A: AsAccessor>(&mut self, accessor: A, mut buffer: B) -> Result<B>
    where
        T: func::Lower + 'static,
        B: WriteBuffer<T>,
    {
        let accessor = accessor.as_accessor();
        let (read, guest_offset) = accessor.with(|mut access| {
            let transmit = self
                .instance
                .concurrent_state_mut(access.as_context_mut().0)
                .get_mut(self.id)?;

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

                if let TransmitKind::Future = self.kind {
                    accessor.with(|mut access| {
                        self.instance
                            .concurrent_state_mut(access.as_context_mut().0)
                            .get_mut(self.id)?
                            .done = true;
                        anyhow::Ok(())
                    })?;
                }

                let old_remaining = buffer.remaining().len();
                let instance = self.instance;
                let accept = move |mut store: StoreContextMut<A::Data>| {
                    lower::<T, B, A::Data>(
                        store.as_context_mut(),
                        instance,
                        &options,
                        ty,
                        address + (T::SIZE32 * guest_offset),
                        count - guest_offset,
                        &mut buffer,
                    )?;
                    anyhow::Ok(buffer)
                };

                let buffer = if T::MAY_REQUIRE_REALLOC {
                    // For payloads which may require a realloc call, use a
                    // oneshot::channel and background task.  This is necessary
                    // because calling the guest while there are host embedder
                    // frames on the stack is unsound.
                    let (tx, rx) = oneshot::channel();
                    accessor.with(move |mut access| {
                        let mut store = access.as_context_mut();
                        let token = StoreToken::new(store.as_context_mut());
                        instance.concurrent_state_mut(store.0).push_high_priority(
                            WorkItem::WorkerFunction(Mutex::new(Box::new(move |store, _| {
                                _ = tx.send(accept(token.as_context_mut(store))?);
                                Ok(())
                            }))),
                        )
                    });
                    rx.await?
                } else {
                    // Optimize flat payloads (i.e. those which do not require
                    // calling the guest's realloc function) by lowering
                    // directly instead of using a oneshot::channel and
                    // background task.
                    accessor.with(|mut access| accept(access.as_context_mut()))?
                };

                accessor.with(|mut access| {
                    let count = old_remaining - buffer.remaining().len();

                    let transmit = self
                        .instance
                        .concurrent_state_mut(access.as_context_mut().0)
                        .get_mut(self.id)?;

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

                Ok(buffer)
            }

            ReadState::HostToHost { accept } => {
                let state = accept(&mut UntypedWriteBuffer::new(&mut buffer)).await?;
                accessor.with(|mut access| {
                    self.instance
                        .concurrent_state_mut(access.as_context_mut().0)
                        .get_mut(self.id)?
                        .read = match state {
                        StreamState::Closed => ReadState::Dropped,
                        StreamState::Open => ReadState::HostToHost { accept },
                    };

                    anyhow::Ok(())
                })?;
                Ok(buffer)
            }

            _ => unreachable!(),
        }
    }
}

impl Destination<u8> {
    /// Return a `GuestDestination` view of `self` if the guest is reading.
    pub fn as_guest_destination<'a, D>(
        &'a mut self,
        store: StoreContextMut<'a, D>,
    ) -> Option<GuestDestination<'a, D>> {
        if let ReadState::GuestReady { .. } = self
            .instance
            .concurrent_state_mut(store.0)
            .get_mut(self.id)
            .unwrap()
            .read
        {
            Some(GuestDestination {
                instance: self.instance,
                id: self.id,
                store,
            })
        } else {
            None
        }
    }
}

/// Represents a guest read from a `stream<u8>`, providing direct access to the
/// guest's buffer.
pub struct GuestDestination<'a, D: 'static> {
    instance: Instance,
    id: TableId<TransmitState>,
    store: StoreContextMut<'a, D>,
}

impl<D: 'static> GuestDestination<'_, D> {
    /// Provide direct access to the guest's buffer.
    pub fn remaining(&mut self) -> &mut [u8] {
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
            unreachable!()
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

    /// Mark the specified number of bytes as written to the guest's buffer.
    ///
    /// This will panic if the count is larger than the size of the
    /// buffer returned by `Self::remaining`.
    pub fn mark_written(&mut self, count: usize) {
        let transmit = self
            .instance
            .concurrent_state_mut(self.store.as_context_mut().0)
            .get_mut(self.id)
            .unwrap();

        let ReadState::GuestReady {
            count: read_count, ..
        } = &transmit.read
        else {
            unreachable!()
        };

        let WriteState::HostReady { guest_offset, .. } = &mut transmit.write else {
            unreachable!()
        };

        if *guest_offset + count > *read_count {
            panic!("write count ({count}) must be less than or equal to read count ({read_count})")
        } else {
            *guest_offset += count;
        }
    }
}

/// Represents the state of a `Stream{Producer,Consumer}`.
#[derive(Copy, Clone, Debug)]
pub enum StreamState {
    /// The producer or consumer may be able to produce or consume more items,
    /// respectively.
    Open,
    /// The producer or consumer is _not_ able to produce or consume more items,
    /// respectively.
    Closed,
}

/// Represents the host-owned write end of a stream.
pub trait StreamProducer<D, T>: Send + 'static {
    /// Handle a host- or guest-initiated read by delivering zero or more items
    /// to the specified destination.
    ///
    /// The returned future will resolve to `Ok(StreamState::Closed)` if and
    /// when this producer cannot produce any more items.
    fn produce(
        &mut self,
        accessor: &Accessor<D>,
        destination: &mut Destination<T>,
    ) -> impl Future<Output = Result<StreamState>> + Send;

    /// Handle a guest-initiated zero-length read by returning a future which
    /// resolves once this producer is either ready to produce more items or is
    /// closed.
    fn when_ready(
        &mut self,
        accessor: &Accessor<D>,
    ) -> impl Future<Output = Result<StreamState>> + Send;
}

/// Represents the buffer for a host- or guest-initiated stream write.
pub struct Source<'a, T> {
    instance: Instance,
    id: TableId<TransmitState>,
    host_buffer: Option<&'a mut dyn WriteBuffer<T>>,
}

impl<T> Source<'_, T> {
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
}

impl Source<'_, u8> {
    /// Return a `GuestSource` view of `self` if the guest is writing.
    pub fn as_guest_source<'a, D>(
        &'a mut self,
        store: StoreContextMut<'a, D>,
    ) -> Option<GuestSource<'a, D>> {
        if let WriteState::GuestReady { .. } = self
            .instance
            .concurrent_state_mut(store.0)
            .get_mut(self.id)
            .unwrap()
            .write
        {
            assert!(self.host_buffer.is_none());
            Some(GuestSource {
                instance: self.instance,
                id: self.id,
                store,
            })
        } else {
            None
        }
    }
}

/// Represents a guest write to a `stream<u8>`, providing direct access to the
/// guest's buffer.
pub struct GuestSource<'a, D: 'static> {
    instance: Instance,
    id: TableId<TransmitState>,
    store: StoreContextMut<'a, D>,
}

impl<D: 'static> GuestSource<'_, D> {
    /// Provide direct access to the guest's buffer.
    pub fn remaining(&mut self) -> &[u8] {
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

    /// Mark the specified number of bytes as read from the guest's buffer.
    ///
    /// This will panic if the count is larger than the size of the buffer
    /// returned by `Self::remaining`.
    pub fn mark_read(&mut self, count: usize) {
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
            panic!("read count ({count}) must be less than or equal to write count ({write_count})")
        } else {
            *guest_offset += count;
        }
    }
}

/// Represents the host-owned read end of a stream.
pub trait StreamConsumer<D, T>: Send + 'static {
    /// Handle a host- or guest-initiated write by accepting zero or more items
    /// from the specified source.
    ///
    /// The returned future will resolve to `Ok(StreamState::Closed)` if and
    /// when this consumer cannot accept any more items.
    fn consume(
        &mut self,
        accessor: &Accessor<D>,
        source: &mut Source<T>,
    ) -> impl Future<Output = Result<StreamState>> + Send;

    /// Handle a guest-initiated zero-length write by returning a future which
    /// resolves once this consumer is either ready to consume more items or is
    /// closed.
    fn when_ready(
        &mut self,
        accessor: &Accessor<D>,
    ) -> impl Future<Output = Result<StreamState>> + Send;
}

/// Represents a host-owned write end of a future.
pub trait FutureProducer<D, T>: Send + 'static {
    /// Handle a host- or guest-initiated read by producing a value.
    fn produce(self, accessor: &Accessor<D>) -> impl Future<Output = Result<T>> + Send;
}

/// Represents a host-owned read end of a future.
pub trait FutureConsumer<D, T>: Send + 'static {
    /// Handle a host- or guest-initiated write by consuming a value.
    fn consume(self, accessor: &Accessor<D>, value: T) -> impl Future<Output = Result<()>> + Send;
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
        store: S,
        producer: impl FutureProducer<S::Data, T>,
    ) -> Self
    where
        T: func::Lower + func::Lift + Send + Sync + 'static,
    {
        struct Producer<P>(Option<P>);

        impl<D, T: func::Lower + 'static, P: FutureProducer<D, T>> StreamProducer<D, T> for Producer<P> {
            async fn produce(
                &mut self,
                accessor: &Accessor<D>,
                destination: &mut Destination<T>,
            ) -> Result<StreamState> {
                let value = self.0.take().unwrap().produce(accessor).await?;
                let value = destination.write(accessor, Some(value)).await?;
                assert!(value.is_none());
                Ok(StreamState::Open)
            }

            async fn when_ready(&mut self, _: &Accessor<D>) -> Result<StreamState> {
                Ok(StreamState::Open)
            }
        }

        Self::new_(
            instance.new_transmit(store, TransmitKind::Future, Producer(Some(producer))),
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
    pub fn pipe<S: AsContextMut>(self, store: S, consumer: impl FutureConsumer<S::Data, T>)
    where
        T: func::Lift + 'static,
    {
        struct Consumer<C>(Option<C>);

        impl<D: 'static, T: func::Lift + 'static, C: FutureConsumer<D, T>> StreamConsumer<D, T>
            for Consumer<C>
        {
            async fn consume(
                &mut self,
                accessor: &Accessor<D>,
                source: &mut Source<'_, T>,
            ) -> Result<StreamState> {
                let value = &mut None;
                accessor.with(|access| source.read(access, value))?;
                self.0
                    .take()
                    .unwrap()
                    .consume(accessor, value.take().unwrap())
                    .await?;
                Ok(StreamState::Open)
            }

            async fn when_ready(&mut self, _: &Accessor<D>) -> Result<StreamState> {
                Ok(StreamState::Open)
            }
        }

        self.instance.set_consumer(
            store,
            self.id,
            TransmitKind::Future,
            Consumer(Some(consumer)),
        );
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
        instance.concurrent_state_mut(store.0).get(id)?; // Just make sure it's present
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

                if concurrent_state.get(state)?.done {
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
        let id = mem::replace(&mut self.id, TableId::new(0));
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
            let state = concurrent_state.get(id)?.state;
            let rep = concurrent_state.get(state)?.read_handle.rep();

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
        producer: impl StreamProducer<S::Data, T>,
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
    pub fn pipe<S: AsContextMut>(self, store: S, consumer: impl StreamConsumer<S::Data, T>)
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
        instance.concurrent_state_mut(store.0).get(id)?; // Just make sure it's present
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
        let id = mem::replace(&mut self.id, TableId::new(0));
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
            let state = concurrent_state.get(id)?.state;
            let rep = concurrent_state.get(state)?.read_handle.rep();

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
            write_handle: TableId::new(0),
            read_handle: TableId::new(0),
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
        produce: Box<
            dyn Fn() -> Pin<Box<dyn Future<Output = Result<StreamState>> + Send + 'static>>
                + Send
                + Sync,
        >,
        guest_offset: usize,
        join: Option<JoinHandle>,
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
        consume: Box<
            dyn Fn() -> Pin<Box<dyn Future<Output = Result<StreamState>> + Send + 'static>>
                + Send
                + Sync,
        >,
        guest_offset: usize,
        join: Option<JoinHandle>,
    },
    /// Both the read and write ends are owned by the host.
    HostToHost {
        accept: Box<
            dyn for<'a> Fn(
                    &'a mut UntypedWriteBuffer<'a>,
                )
                    -> Pin<Box<dyn Future<Output = Result<StreamState>> + Send + 'a>>
                + Send
                + Sync,
        >,
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

fn return_code(kind: TransmitKind, state: StreamState, guest_offset: usize) -> ReturnCode {
    let count = guest_offset.try_into().unwrap();
    match state {
        StreamState::Closed => ReturnCode::Dropped(count),
        StreamState::Open => ReturnCode::completed(kind, count),
    }
}

impl Instance {
    fn new_transmit<T, S: AsContextMut>(
        self,
        mut store: S,
        kind: TransmitKind,
        producer: impl StreamProducer<S::Data, T>,
    ) -> TableId<TransmitHandle> {
        let mut store = store.as_context_mut();
        let token = StoreToken::new(store.as_context_mut());
        let state = self.concurrent_state_mut(store.0);
        let (_, read) = state.new_transmit().unwrap();
        let producer = Arc::new(Mutex::new(Some(producer)));
        let id = state.get(read).unwrap().state;
        let produce = Box::new(move || {
            let producer = producer.clone();
            async move {
                let zero_length_read = tls::get(|store| {
                    anyhow::Ok(matches!(
                        self.concurrent_state_mut(store).get(id)?.read,
                        ReadState::GuestReady { count: 0, .. }
                    ))
                })?;

                let mut mine = producer.lock().unwrap().take().unwrap();
                let accessor = &Accessor::new(token, Some(self));
                let result = if zero_length_read {
                    mine.when_ready(accessor).await
                } else {
                    mine.produce(
                        accessor,
                        &mut Destination {
                            instance: self,
                            id,
                            kind,
                            _phantom: PhantomData,
                        },
                    )
                    .await
                };
                *producer.lock().unwrap() = Some(mine);
                result
            }
            .boxed()
        });
        state.get_mut(id).unwrap().write = WriteState::HostReady {
            produce,
            guest_offset: 0,
            join: None,
        };
        read
    }

    fn set_consumer<T: 'static, S: AsContextMut>(
        self,
        mut store: S,
        id: TableId<TransmitHandle>,
        kind: TransmitKind,
        consumer: impl StreamConsumer<S::Data, T>,
    ) {
        let mut store = store.as_context_mut();
        let token = StoreToken::new(store.as_context_mut());
        let state = self.concurrent_state_mut(store.0);
        let id = state.get(id).unwrap().state;
        let transmit = state.get_mut(id).unwrap();
        let consumer = Arc::new(Mutex::new(Some(consumer)));
        let consume = {
            let consumer = consumer.clone();
            Box::new(move || {
                let consumer = consumer.clone();
                async move {
                    let zero_length_write = tls::get(|store| {
                        anyhow::Ok(matches!(
                            self.concurrent_state_mut(store).get(id)?.write,
                            WriteState::GuestReady { count: 0, .. }
                        ))
                    })?;

                    let mut mine = consumer.lock().unwrap().take().unwrap();
                    let accessor = &Accessor::new(token, Some(self));
                    let result = if zero_length_write {
                        mine.when_ready(accessor).await
                    } else {
                        mine.consume(
                            accessor,
                            &mut Source {
                                instance: self,
                                id,
                                host_buffer: None,
                            },
                        )
                        .await
                    };
                    *consumer.lock().unwrap() = Some(mine);
                    result
                }
                .boxed()
            })
        };

        match &transmit.write {
            WriteState::Open => {
                transmit.read = ReadState::HostReady {
                    consume,
                    guest_offset: 0,
                    join: None,
                };
            }
            WriteState::GuestReady { .. } => {
                let future = consume();
                transmit.read = ReadState::HostReady {
                    consume,
                    guest_offset: 0,
                    join: None,
                };
                self.pipe_from_guest(store, kind, id, future).unwrap();
            }
            WriteState::HostReady { .. } => {
                let WriteState::HostReady { produce, .. } =
                    mem::replace(&mut transmit.write, WriteState::Open)
                else {
                    unreachable!();
                };

                transmit.read = ReadState::HostToHost {
                    accept: Box::new(move |input| {
                        let consumer = consumer.clone();
                        async move {
                            let mut mine = consumer.lock().unwrap().take().unwrap();
                            let result = mine
                                .consume(
                                    &Accessor::new(token, Some(self)),
                                    &mut Source {
                                        instance: self,
                                        id,
                                        host_buffer: Some(input.get_mut::<T>()),
                                    },
                                )
                                .await;
                            *consumer.lock().unwrap() = Some(mine);
                            result
                        }
                        .boxed()
                    }),
                };

                let future = async move {
                    loop {
                        if tls::get(|store| {
                            anyhow::Ok(matches!(
                                self.concurrent_state_mut(store).get(id)?.read,
                                ReadState::Dropped
                            ))
                        })? {
                            break Ok(());
                        }

                        match produce().await? {
                            StreamState::Open => {}
                            StreamState::Closed => break Ok(()),
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
            WriteState::Dropped => unreachable!(),
        }
    }

    fn pipe_from_guest(
        self,
        mut store: impl AsContextMut,
        kind: TransmitKind,
        id: TableId<TransmitState>,
        future: Pin<Box<dyn Future<Output = Result<StreamState>> + Send + 'static>>,
    ) -> Result<()> {
        let (join, future) = JoinHandle::run(async move {
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
                    StreamState::Closed => ReadState::Dropped,
                    StreamState::Open => ReadState::HostReady {
                        consume,
                        guest_offset: 0,
                        join: None,
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
        });
        let state = self.concurrent_state_mut(store.as_context_mut().0);
        state.push_future(future.map(|result| result.unwrap_or(Ok(()))).boxed());
        let ReadState::HostReady {
            join: state_join, ..
        } = &mut state.get_mut(id)?.read
        else {
            unreachable!()
        };
        *state_join = Some(join);
        Ok(())
    }

    fn pipe_to_guest(
        self,
        mut store: impl AsContextMut,
        kind: TransmitKind,
        id: TableId<TransmitState>,
        future: Pin<Box<dyn Future<Output = Result<StreamState>> + Send + 'static>>,
    ) -> Result<()> {
        let (join, future) = JoinHandle::run(async move {
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
                    StreamState::Closed => WriteState::Dropped,
                    StreamState::Open => WriteState::HostReady {
                        produce,
                        guest_offset: 0,
                        join: None,
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
        });
        let state = self.concurrent_state_mut(store.as_context_mut().0);
        state.push_future(future.map(|result| result.unwrap_or(Ok(()))).boxed());
        let WriteState::HostReady {
            join: state_join, ..
        } = &mut state.get_mut(id)?.write
        else {
            unreachable!()
        };
        *state_join = Some(join);
        Ok(())
    }

    /// Drop the read end of a stream or future read from the host.
    fn host_drop_reader(
        self,
        store: &mut dyn VMStore,
        id: TableId<TransmitHandle>,
        kind: TransmitKind,
    ) -> Result<()> {
        let transmit_id = self.concurrent_state_mut(store).get(id)?.state;
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
        let transmit_id = self.concurrent_state_mut(store.0).get(id)?.state;
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
        if !options.async_() {
            bail!("synchronous stream and future writes not yet supported");
        }
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
        let transmit_id = concurrent_state.get(transmit_handle)?.state;
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

        let result = match mem::replace(&mut transmit.read, new_state) {
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
                join,
            } => {
                assert!(join.is_none());
                assert_eq!(0, guest_offset);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let mut future = consume();
                transmit.read = ReadState::HostReady {
                    consume,
                    guest_offset: 0,
                    join: None,
                };
                set_guest_ready(concurrent_state)?;
                let poll = self.set_tls(store.0, || {
                    future
                        .as_mut()
                        .poll(&mut Context::from_waker(&Waker::noop()))
                });

                match poll {
                    Poll::Ready(state) => {
                        let transmit = self.concurrent_state_mut(store.0).get_mut(transmit_id)?;
                        let ReadState::HostReady { guest_offset, .. } = &mut transmit.read else {
                            unreachable!();
                        };
                        let code = return_code(ty.kind(), state?, mem::replace(guest_offset, 0));
                        transmit.write = WriteState::Open;
                        code
                    }
                    Poll::Pending => {
                        self.pipe_from_guest(
                            store.as_context_mut(),
                            ty.kind(),
                            transmit_id,
                            future,
                        )?;
                        ReturnCode::Blocked
                    }
                }
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

        if result != ReturnCode::Blocked {
            *self.id().get_mut(store.0).get_mut_by_index(ty, handle)?.1 =
                TransmitLocalState::Write {
                    done: matches!(
                        (result, ty),
                        (ReturnCode::Dropped(_), TransmitIndex::Stream(_))
                    ),
                };
        }

        Ok(result)
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
        if !options.async_() {
            bail!("synchronous stream and future reads not yet supported");
        }
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
        let transmit_id = concurrent_state.get(transmit_handle)?.state;
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

        let result = match mem::replace(&mut transmit.write, new_state) {
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
                join,
            } => {
                assert!(join.is_none());
                assert_eq!(0, guest_offset);

                if let TransmitIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let mut future = produce();
                transmit.write = WriteState::HostReady {
                    produce,
                    guest_offset: 0,
                    join: None,
                };
                set_guest_ready(concurrent_state)?;
                let poll = self.set_tls(store.0, || {
                    future
                        .as_mut()
                        .poll(&mut Context::from_waker(&Waker::noop()))
                });

                match poll {
                    Poll::Ready(state) => {
                        let transmit = self.concurrent_state_mut(store.0).get_mut(transmit_id)?;
                        let WriteState::HostReady { guest_offset, .. } = &mut transmit.write else {
                            unreachable!();
                        };
                        let code = return_code(ty.kind(), state?, mem::replace(guest_offset, 0));
                        transmit.read = ReadState::Open;
                        code
                    }
                    Poll::Pending => {
                        self.pipe_to_guest(store.as_context_mut(), ty.kind(), transmit_id, future)?;
                        ReturnCode::Blocked
                    }
                }
            }

            WriteState::Open => {
                set_guest_ready(concurrent_state)?;
                ReturnCode::Blocked
            }

            WriteState::Dropped => ReturnCode::Dropped(0),
        };

        if result != ReturnCode::Blocked {
            *self.id().get_mut(store.0).get_mut_by_index(ty, handle)?.1 =
                TransmitLocalState::Read {
                    done: matches!(
                        (result, ty),
                        (ReturnCode::Dropped(_), TransmitIndex::Stream(_))
                    ),
                };
        }

        Ok(result)
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

    /// Cancel a pending write for the specified stream or future from the guest.
    fn guest_cancel_write(
        mut self: Pin<&mut Self>,
        ty: TransmitIndex,
        writer: u32,
        _async_: bool,
    ) -> Result<ReturnCode> {
        let (rep, state) = get_mut_by_index_from(self.as_mut().table_for_transmit(ty), ty, writer)?;
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel write {id:?} (handle {writer})");
        match state {
            TransmitLocalState::Write { .. } => {
                bail!("stream or future write cancelled when no write is pending")
            }
            TransmitLocalState::Read { .. } => {
                bail!("passed read end to `{{stream|future}}.cancel-write`")
            }
            TransmitLocalState::Busy => {
                *state = TransmitLocalState::Write { done: false };
            }
        }
        let state = self.concurrent_state_mut();
        let rep = state.get(id)?.state.rep();
        state.host_cancel_write(rep)
    }

    /// Cancel a pending read for the specified stream or future from the guest.
    fn guest_cancel_read(
        mut self: Pin<&mut Self>,
        ty: TransmitIndex,
        reader: u32,
        _async_: bool,
    ) -> Result<ReturnCode> {
        let (rep, state) = get_mut_by_index_from(self.as_mut().table_for_transmit(ty), ty, reader)?;
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel read {id:?} (handle {reader})");
        match state {
            TransmitLocalState::Read { .. } => {
                bail!("stream or future read cancelled when no read is pending")
            }
            TransmitLocalState::Write { .. } => {
                bail!("passed write end to `{{stream|future}}.cancel-read`")
            }
            TransmitLocalState::Busy => {
                *state = TransmitLocalState::Read { done: false };
            }
        }
        let state = self.concurrent_state_mut();
        let rep = state.get(id)?.state.rep();
        state.host_cancel_read(rep)
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

    /// Implements the `future.cancel-write` intrinsic.
    pub(crate) fn future_cancel_write(
        self: Pin<&mut Self>,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(TransmitIndex::Future(ty), writer, async_)
            .map(|result| result.encode())
    }

    /// Implements the `future.cancel-read` intrinsic.
    pub(crate) fn future_cancel_read(
        self: Pin<&mut Self>,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(TransmitIndex::Future(ty), reader, async_)
            .map(|result| result.encode())
    }

    /// Implements the `stream.new` intrinsic.
    pub(crate) fn stream_new(
        self: Pin<&mut Self>,
        ty: TypeStreamTableIndex,
    ) -> Result<ResourcePair> {
        self.guest_new(TransmitIndex::Stream(ty))
    }

    /// Implements the `stream.cancel-write` intrinsic.
    pub(crate) fn stream_cancel_write(
        self: Pin<&mut Self>,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(TransmitIndex::Stream(ty), writer, async_)
            .map(|result| result.encode())
    }

    /// Implements the `stream.cancel-read` intrinsic.
    pub(crate) fn stream_cancel_read(
        self: Pin<&mut Self>,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(TransmitIndex::Stream(ty), reader, async_)
            .map(|result| result.encode())
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

    /// Cancel a pending stream or future write from the host.
    ///
    /// # Arguments
    ///
    /// * `rep` - The `TransmitState` rep for the stream or future.
    fn host_cancel_write(&mut self, rep: u32) -> Result<ReturnCode> {
        let transmit_id = TableId::<TransmitState>::new(rep);
        let transmit = self.get_mut(transmit_id)?;
        log::trace!(
            "host_cancel_write state {transmit_id:?}; write state {:?} read state {:?}",
            transmit.read,
            transmit.write
        );

        let code = if let Some(event) =
            Waitable::Transmit(transmit.write_handle).take_event(self)?
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
            join, guest_offset, ..
        } = &mut self.get_mut(transmit_id)?.read
        {
            if let Some(join) = join.take() {
                join.abort();
            }
            ReturnCode::Cancelled(u32::try_from(mem::replace(guest_offset, 0)).unwrap())
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.get_mut(transmit_id)?;

        match &transmit.write {
            WriteState::GuestReady { .. } => {
                transmit.write = WriteState::Open;
            }
            WriteState::HostReady { .. } => todo!("support host write cancellation"),
            WriteState::Open | WriteState::Dropped => {}
        }

        log::trace!("cancelled write {transmit_id:?}");

        Ok(code)
    }

    /// Cancel a pending stream or future read from the host.
    ///
    /// # Arguments
    ///
    /// * `rep` - The `TransmitState` rep for the stream or future.
    fn host_cancel_read(&mut self, rep: u32) -> Result<ReturnCode> {
        let transmit_id = TableId::<TransmitState>::new(rep);
        let transmit = self.get_mut(transmit_id)?;
        log::trace!(
            "host_cancel_read state {transmit_id:?}; read state {:?} write state {:?}",
            transmit.read,
            transmit.write
        );

        let code = if let Some(event) = Waitable::Transmit(transmit.read_handle).take_event(self)? {
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
            join, guest_offset, ..
        } = &mut self.get_mut(transmit_id)?.write
        {
            if let Some(join) = join.take() {
                join.abort();
            }
            ReturnCode::Cancelled(u32::try_from(mem::replace(guest_offset, 0)).unwrap())
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.get_mut(transmit_id)?;

        match &transmit.read {
            ReadState::GuestReady { .. } => {
                transmit.read = ReadState::Open;
            }
            ReadState::HostReady { .. } | ReadState::HostToHost { .. } => {
                todo!("support host read cancellation")
            }
            ReadState::Open | ReadState::Dropped => {}
        }

        log::trace!("cancelled read {transmit_id:?}");

        Ok(code)
    }
}

pub(crate) struct ResourcePair {
    pub(crate) write: u32,
    pub(crate) read: u32,
}
