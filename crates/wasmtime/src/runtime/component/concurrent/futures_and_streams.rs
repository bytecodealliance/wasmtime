use {
    super::{
        Event, GlobalErrorContextRefCount, HostTaskOutput, LocalErrorContextRefCount, StateTable,
        Waitable, WaitableCommon, WaitableState,
        table::{TableDebug, TableId},
    },
    crate::{
        AsContextMut, StoreContextMut, ValRaw,
        component::{
            Instance, Lower, Val, WasmList, WasmStr,
            concurrent::{ConcurrentState, tls},
            func::{self, LiftContext, LowerContext, Options},
            matching::InstanceType,
            values::{ErrorContextAny, FutureAny, StreamAny},
        },
        store::{StoreOpaque, StoreToken},
        vm::{VMFuncRef, VMMemoryDefinition, VMStore},
    },
    anyhow::{Context, Result, anyhow, bail},
    buffers::Extender,
    futures::{
        channel::{mpsc, oneshot},
        future::{self, FutureExt},
        stream::StreamExt,
    },
    std::{
        boxed::Box,
        fmt,
        future::Future,
        iter,
        marker::PhantomData,
        mem::{self, MaybeUninit},
        ops::DerefMut,
        ptr::NonNull,
        string::{String, ToString},
        sync::{Arc, Mutex},
        task::{Poll, Waker},
        vec::Vec,
    },
    wasmtime_environ::component::{
        CanonicalAbiInfo, ComponentTypes, InterfaceType, RuntimeComponentInstanceIndex,
        StringEncoding, TypeComponentGlobalErrorContextTableIndex,
        TypeComponentLocalErrorContextTableIndex, TypeFutureTableIndex, TypeStreamTableIndex,
    },
};

pub use buffers::{ReadBuffer, TakeBuffer, VecBuffer, WriteBuffer};

mod buffers;

/// Enum for distinguishing between a stream or future in functions that handle
/// both.
#[derive(Copy, Clone, Debug)]
enum TransmitKind {
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
pub(super) enum TableIndex {
    Stream(TypeStreamTableIndex),
    Future(TypeFutureTableIndex),
}

impl TableIndex {
    fn kind(&self) -> TransmitKind {
        match self {
            TableIndex::Stream(_) => TransmitKind::Stream,
            TableIndex::Future(_) => TransmitKind::Future,
        }
    }
}

/// Action to take after writing
enum PostWrite {
    /// Continue performing writes
    Continue,
    /// Drop the channel post-write
    Drop,
}

/// Represents the result of a host-initiated stream or future read or write.
struct HostResult<B> {
    /// The buffer provided when reading or writing.
    buffer: B,
    /// Whether the other end of the stream or future has been dropped.
    dropped: bool,
}

/// Retrieve the payload type of the specified stream or future, or `None` if it
/// has no payload type.
fn payload(ty: TableIndex, types: &Arc<ComponentTypes>) -> Option<InterfaceType> {
    match ty {
        TableIndex::Future(ty) => types[types[ty].ty].payload,
        TableIndex::Stream(ty) => types[types[ty].ty].payload,
    }
}

/// Retrieve the host rep and state for the specified guest-visible waitable
/// handle.
fn get_mut_by_index_from(
    state_table: &mut StateTable<WaitableState>,
    ty: TableIndex,
    index: u32,
) -> Result<(u32, &mut StreamFutureState)> {
    Ok(match ty {
        TableIndex::Stream(ty) => {
            let (rep, WaitableState::Stream(actual_ty, state)) =
                state_table.get_mut_by_index(index)?
            else {
                bail!("invalid stream handle");
            };
            if *actual_ty != ty {
                bail!("invalid stream handle");
            }
            (rep, state)
        }
        TableIndex::Future(ty) => {
            let (rep, WaitableState::Future(actual_ty, state)) =
                state_table.get_mut_by_index(index)?
            else {
                bail!("invalid future handle");
            };
            if *actual_ty != ty {
                bail!("invalid future handle");
            }
            (rep, state)
        }
    })
}

/// Construct a `WaitableState` using the specified type and state.
fn waitable_state(ty: TableIndex, state: StreamFutureState) -> WaitableState {
    match ty {
        TableIndex::Stream(ty) => WaitableState::Stream(ty, state),
        TableIndex::Future(ty) => WaitableState::Future(ty, state),
    }
}

/// Return a closure which matches a host write operation to a read (or drop)
/// operation.
///
/// This may be used when the host initiates a write but there is no read
/// pending at the other end, in which case we construct a
/// `WriteState::HostReady` using the closure created here and leave it in
/// `TransmitState::write` for the reader to find and call when it's ready.
fn accept_reader<T: func::Lower + Send + 'static, B: WriteBuffer<T>, U: 'static>(
    store: StoreContextMut<U>,
    mut buffer: B,
    tx: oneshot::Sender<HostResult<B>>,
    kind: TransmitKind,
) -> impl FnOnce(&mut dyn VMStore, Instance, Reader) -> Result<ReturnCode>
+ Send
+ Sync
+ 'static
+ use<T, B, U> {
    let token = StoreToken::new(store);
    move |store, instance, reader| {
        let code = match reader {
            Reader::Guest {
                options,
                ty,
                address,
                count,
            } => {
                let mut store = token.as_context_mut(store);
                let types = instance.id().get(store.0).component().types().clone();
                let count = buffer.remaining().len().min(count);

                let lower =
                    &mut LowerContext::new(store.as_context_mut(), options, &types, instance);
                if address % usize::try_from(T::ALIGN32)? != 0 {
                    bail!("read pointer not aligned");
                }
                lower
                    .as_slice_mut()
                    .get_mut(address..)
                    .and_then(|b| b.get_mut(..T::SIZE32 * count))
                    .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?;

                if let Some(ty) = payload(ty, &types) {
                    T::linear_store_list_to_memory(
                        lower,
                        ty,
                        address,
                        &buffer.remaining()[..count],
                    )?;
                }

                buffer.skip(count);
                _ = tx.send(HostResult {
                    buffer,
                    dropped: false,
                });
                ReturnCode::completed(kind, count.try_into().unwrap())
            }
            Reader::Host { accept } => {
                let count = buffer.remaining().len();
                let count = accept(&mut buffer, count);
                _ = tx.send(HostResult {
                    buffer,
                    dropped: false,
                });
                ReturnCode::completed(kind, count.try_into().unwrap())
            }
            Reader::End => {
                _ = tx.send(HostResult {
                    buffer,
                    dropped: true,
                });
                ReturnCode::Dropped(0)
            }
        };

        Ok(code)
    }
}

/// Return a closure which matches a host read operation to a write (or drop)
/// operation.
///
/// This may be used when the host initiates a read but there is no write
/// pending at the other end, in which case we construct a
/// `ReadState::HostReady` using the closure created here and leave it in
/// `TransmitState::read` for the writer to find and call when it's ready.
fn accept_writer<T: func::Lift + Send + 'static, B: ReadBuffer<T>, U>(
    mut buffer: B,
    tx: oneshot::Sender<HostResult<B>>,
    kind: TransmitKind,
) -> impl FnOnce(Writer) -> Result<ReturnCode> + Send + Sync + 'static {
    move |writer| {
        let count = match writer {
            Writer::Guest {
                lift,
                ty,
                address,
                count,
            } => {
                let count = count.min(buffer.remaining_capacity());
                if T::IS_RUST_UNIT_TYPE {
                    // SAFETY: `T::IS_RUST_UNIT_TYPE` is only true for `()`, a
                    // zero-sized type, so `MaybeUninit::uninit().assume_init()`
                    // is a valid way to populate the zero-sized buffer.
                    buffer.extend(
                        iter::repeat_with(|| unsafe { MaybeUninit::uninit().assume_init() })
                            .take(count),
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
                    T::load_into(lift, list, &mut Extender(&mut buffer), count)?
                }
                _ = tx.send(HostResult {
                    buffer,
                    dropped: false,
                });
                ReturnCode::completed(kind, count.try_into().unwrap())
            }
            Writer::Host {
                buffer: input,
                count,
            } => {
                let count = count.min(buffer.remaining_capacity());
                buffer.move_from(input, count);
                _ = tx.send(HostResult {
                    buffer,
                    dropped: false,
                });
                ReturnCode::completed(kind, count.try_into().unwrap())
            }
            Writer::End => {
                _ = tx.send(HostResult {
                    buffer,
                    dropped: true,
                });
                ReturnCode::Dropped(0)
            }
        };

        Ok(count)
    }
}

/// Represents the state of a stream or future handle from the perspective of a
/// given component instance.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum StreamFutureState {
    /// The write end of the stream or future.
    Write {
        /// Whether the component instance has been notified that the stream or
        /// future is "done" (i.e. the other end has dropped, or, in the case of
        /// a future, a value has been transmitted).
        done: bool,
    },
    /// The read end of the stream or future.
    Read {
        /// Whether the component instance has been notified that the stream or
        /// future is "done" (i.e. the other end has dropped, or, in the case of
        /// a future, a value has been transmitted).
        done: bool,
    },
    /// A read or write is in progress.
    Busy,
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

/// Represents a pending event on a host-owned write end of a stream or future.
///
/// See `ComponentInstance::start_write_event_loop` for details.
enum WriteEvent<B> {
    /// Write the items in the specified buffer to the stream or future, and
    /// return the result via the specified `Sender`.
    Write {
        buffer: B,
        tx: oneshot::Sender<HostResult<B>>,
    },
    /// Drop the write end of the stream or future.
    Drop(Option<Box<dyn FnOnce() -> B + Send + Sync>>),
    /// Watch the read (i.e. opposite) end of this stream or future, dropping
    /// the specified sender when it is dropped.
    Watch { tx: oneshot::Sender<()> },
}

/// Represents a pending event on a host-owned read end of a stream or future.
///
/// See `ComponentInstance::start_read_event_loop` for details.
enum ReadEvent<B> {
    /// Read as many items as the specified buffer will hold from the stream or
    /// future, and return the result via the specified `Sender`.
    Read {
        buffer: B,
        tx: oneshot::Sender<HostResult<B>>,
    },
    /// Drop the read end of the stream or future.
    Drop,
    /// Watch the write (i.e. opposite) end of this stream or future, dropping
    /// the specified sender when it is dropped.
    Watch { tx: oneshot::Sender<()> },
}

/// Send the specified value to the specified `Sender`.
///
/// This will panic if there is no room in the channel's buffer, so it should
/// only be used in a context where there is at least one empty spot in the
/// buffer.  It will silently ignore any other error (e.g. if the `Receiver` has
/// been dropped).
fn send<T>(tx: &mut mpsc::Sender<T>, value: T) {
    if let Err(e) = tx.try_send(value) {
        if e.is_full() {
            unreachable!();
        }
    }
}

/// State shared between a `Watch` and the wrapped future it is associated with.
///
/// See `Watch` for details.
struct WatchInner<T> {
    inner: T,
    rx: oneshot::Receiver<()>,
    waker: Option<Waker>,
}

/// Wrapper struct which may be converted to the inner value as needed.
///
/// This object is normally paired with a `Future` which represents a state
/// change on the inner value, resolving when that state change happens _or_
/// when the `Watch` is converted back into the inner value -- whichever happens
/// first.
pub struct Watch<T>(Arc<Mutex<Option<WatchInner<T>>>>);

impl<T> Watch<T> {
    /// Convert this object into its inner value.
    ///
    /// Calling this function will cause the associated `Future` to resolve
    /// immediately if it hasn't already.
    pub fn into_inner(self) -> T {
        let inner = self.0.try_lock().unwrap().take().unwrap();
        if let Some(waker) = inner.waker {
            waker.wake();
        }
        inner.inner
    }
}

/// Wrap the specified `oneshot::Receiver` in a future which resolves when
/// either that `Receiver` resolves or `Watch::into_inner` has been called on
/// the returned `Watch`.
fn watch<T: Send + 'static>(
    instance: Instance,
    rx: oneshot::Receiver<()>,
    inner: T,
) -> (impl Future<Output = ()> + Send + 'static, Watch<T>) {
    let inner = Arc::new(Mutex::new(Some(WatchInner {
        inner,
        rx,
        waker: None,
    })));
    (
        super::checked(
            instance,
            future::poll_fn({
                let inner = inner.clone();

                move |cx| {
                    if let Some(inner) = inner.try_lock().unwrap().deref_mut() {
                        match inner.rx.poll_unpin(cx) {
                            Poll::Ready(_) => Poll::Ready(()),
                            Poll::Pending => {
                                inner.waker = Some(cx.waker().clone());
                                Poll::Pending
                            }
                        }
                    } else {
                        Poll::Ready(())
                    }
                }
            }),
        ),
        Watch(inner),
    )
}

/// Represents the writable end of a Component Model `future`.
pub struct FutureWriter<T: 'static> {
    default: Option<fn() -> T>,
    instance: Instance,
    tx: Option<mpsc::Sender<WriteEvent<Option<T>>>>,
}

impl<T> FutureWriter<T> {
    fn new(
        default: fn() -> T,
        tx: Option<mpsc::Sender<WriteEvent<Option<T>>>>,
        instance: Instance,
    ) -> Self {
        Self {
            default: Some(default),
            instance,
            tx,
        }
    }

    /// Write the specified value to this `future`.
    ///
    /// The returned `Future` will yield `true` if the read end accepted the
    /// value; otherwise it will return `false`, meaning the read end was dropped
    /// before the value could be delivered.
    ///
    /// Note that the returned `Future` must be polled from the event loop of
    /// the component instance from which this `FutureWriter` originated.  See
    /// [`Instance::run`] for details.
    pub fn write(mut self, value: T) -> impl Future<Output = bool> + Send + 'static
    where
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(
            &mut self.tx.as_mut().unwrap(),
            WriteEvent::Write {
                buffer: Some(value),
                tx,
            },
        );
        self.default = None;
        let instance = self.instance;
        super::checked(
            instance,
            rx.map(move |v| {
                drop(self);
                match v {
                    Ok(HostResult { dropped, .. }) => !dropped,
                    Err(_) => todo!("guarantee buffer recovery if event loop errors or panics"),
                }
            }),
        )
    }

    /// Convert this object into a `Future` which will resolve when the read end
    /// of this `future` is dropped, plus a `Watch` which can be used to retrieve
    /// the `FutureWriter` again.
    ///
    /// Note that calling `Watch::into_inner` on the returned `Watch` will have
    /// the side effect of causing the `Future` to resolve immediately if it
    /// hasn't already.
    ///
    /// Also note that the returned `Future` must be polled from the event loop
    /// of the component instance from which this `FutureWriter` originated.
    /// See [`Instance::run`] for details.
    pub fn watch_reader(mut self) -> (impl Future<Output = ()> + Send + 'static, Watch<Self>)
    where
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(&mut self.tx.as_mut().unwrap(), WriteEvent::Watch { tx });
        let instance = self.instance;
        watch(instance, rx, self)
    }
}

impl<T> Drop for FutureWriter<T> {
    fn drop(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            send(
                &mut tx,
                WriteEvent::Drop(self.default.take().map(|v| {
                    Box::new(move || Some(v()))
                        as Box<dyn FnOnce() -> Option<T> + Send + Sync + 'static>
                })),
            );
        }
    }
}

/// Represents the readable end of a Component Model `future`.
///
/// In order to actually read from or drop this `future`, first convert it to a
/// [`FutureReader`] using the `into_reader` method.
///
/// Note that if a value of this type is dropped without either being converted
/// to a `FutureReader` or passed to the guest, any writes on the write end may
/// block forever.
pub struct HostFuture<T> {
    instance: Instance,
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> HostFuture<T> {
    /// Create a new `HostFuture`.
    fn new(rep: u32, instance: Instance) -> Self {
        Self {
            instance,
            rep,
            _phantom: PhantomData,
        }
    }

    /// Convert this object into a [`FutureReader`].
    pub fn into_reader(self, mut store: impl AsContextMut) -> FutureReader<T>
    where
        T: func::Lower + func::Lift + Send + Sync + 'static,
    {
        FutureReader {
            instance: self.instance,
            rep: self.rep,
            tx: Some(self.instance.start_read_event_loop(
                store.as_context_mut(),
                self.rep,
                TransmitKind::Future,
            )),
        }
    }

    /// Convert this `FutureReader` into a [`Val`].
    // See TODO comment for `FutureAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        Val::Future(FutureAny(self.rep))
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
        instance
            .concurrent_state_mut(store.0)
            .get(TableId::<TransmitHandle>::new(*rep))?; // Just make sure it's present
        Ok(Self::new(*rep, instance))
    }

    /// Transfer ownership of the read end of a future from a guest to the host.
    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Future(src) => {
                let state_table = cx
                    .instance_mut()
                    .concurrent_state_mut()
                    .state_table(TableIndex::Future(src));
                let (rep, state) =
                    get_mut_by_index_from(state_table, TableIndex::Future(src), index)?;

                match state {
                    StreamFutureState::Read { .. } => {
                        state_table.remove_by_index(index)?;
                    }
                    StreamFutureState::Write { .. } => bail!("cannot transfer write end of future"),
                    StreamFutureState::Busy => bail!("cannot transfer busy future"),
                }

                let concurrent_state = cx.instance_mut().concurrent_state_mut();
                let state = concurrent_state
                    .get(TableId::<TransmitHandle>::new(rep))?
                    .state;

                if concurrent_state.get(state)?.done {
                    bail!("cannot lift future after previous read succeeded");
                }

                Ok(Self::new(rep, cx.instance_handle()))
            }
            _ => func::bad_type_info(),
        }
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
            let state = concurrent_state
                .get(TableId::<TransmitHandle>::new(rep))?
                .state;
            let rep = concurrent_state.get(state)?.read_handle.rep();

            concurrent_state
                .state_table(TableIndex::Future(dst))
                .insert(
                    rep,
                    WaitableState::Future(dst, StreamFutureState::Read { done: false }),
                )
        }
        _ => func::bad_type_info(),
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower future handles as `u32`s.
unsafe impl<T> func::ComponentType for HostFuture<T> {
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
unsafe impl<T> func::Lower for HostFuture<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        lower_future_to_index(self.rep, cx, ty)?.linear_lower_to_flat(cx, InterfaceType::U32, dst)
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        lower_future_to_index(self.rep, cx, ty)?.linear_lower_to_memory(
            cx,
            InterfaceType::U32,
            offset,
        )
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T> func::Lift for HostFuture<T> {
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

impl<T> From<FutureReader<T>> for HostFuture<T> {
    fn from(mut value: FutureReader<T>) -> Self {
        value.tx.take();

        Self {
            instance: value.instance,
            rep: value.rep,
            _phantom: PhantomData,
        }
    }
}

/// Represents the readable end of a Component Model `future`.
///
/// In order to pass this end to guest code, first convert it to a
/// [`HostFuture`] using the `into` method.
pub struct FutureReader<T> {
    instance: Instance,
    rep: u32,
    tx: Option<mpsc::Sender<ReadEvent<Option<T>>>>,
}

impl<T> FutureReader<T> {
    fn new(rep: u32, tx: Option<mpsc::Sender<ReadEvent<Option<T>>>>, instance: Instance) -> Self {
        Self { instance, rep, tx }
    }

    /// Read the value from this `future`.
    ///
    /// The returned `Future` will yield `None` if the guest has trapped
    /// before it could produce a result or if the write end belonged to the
    /// host and was dropped without writing a result.
    ///
    /// Note that the returned `Future` must be polled from the event loop of
    /// the component instance from which this `FutureReader` originated.  See
    /// [`Instance::run`] for details.
    pub fn read(mut self) -> impl Future<Output = Option<T>> + Send + 'static
    where
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(
            &mut self.tx.as_mut().unwrap(),
            ReadEvent::Read { buffer: None, tx },
        );
        let instance = self.instance;
        super::checked(
            instance,
            rx.map(move |v| {
                drop(self);

                if let Ok(HostResult {
                    mut buffer,
                    dropped: false,
                }) = v
                {
                    buffer.take()
                } else {
                    None
                }
            }),
        )
    }

    /// Convert this object into a `Future` which will resolve when the write
    /// end of this `future` is dropped, plus a `Watch` which can be used to
    /// retrieve the `FutureReader` again.
    ///
    /// Note that calling `Watch::into_inner` on the returned `Watch` will have
    /// the side effect of causing the `Future` to resolve immediately if it
    /// hasn't already.
    ///
    /// Also note that the returned `Future` must be polled from the event loop
    /// of the component instance from which this `FutureReader` originated.
    /// See [`Instance::run`] for details.
    pub fn watch_writer(mut self) -> (impl Future<Output = ()> + Send + 'static, Watch<Self>)
    where
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(&mut self.tx.as_mut().unwrap(), ReadEvent::Watch { tx });
        let instance = self.instance;
        watch(instance, rx, self)
    }
}

impl<T> Drop for FutureReader<T> {
    fn drop(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            send(&mut tx, ReadEvent::Drop);
        }
    }
}

/// Represents the writable end of a Component Model `stream`.
pub struct StreamWriter<B> {
    instance: Instance,
    tx: Option<mpsc::Sender<WriteEvent<B>>>,
}

impl<B> StreamWriter<B> {
    fn new(tx: Option<mpsc::Sender<WriteEvent<B>>>, instance: Instance) -> Self {
        Self { instance, tx }
    }

    /// Write the specified items to the `stream`.
    ///
    /// Note that this will only write as many items as the reader accepts
    /// during its current or next read.  Use `write_all` to loop until the
    /// buffer is drained or the read end is dropped.
    ///
    /// The returned `Future` will yield a `(Some(_), _)` if the write completed
    /// (possibly consuming a subset of the items or nothing depending on the
    /// number of items the reader accepted).  It will return `(None, _)` if the
    /// write failed due to the closure of the read end.  In either case, the
    /// returned buffer will be the same one passed as a parameter, possibly
    /// mutated to consume any written values.
    ///
    /// Note that the returned `Future` must be polled from the event loop of
    /// the component instance from which this `StreamWriter` originated.  See
    /// [`Instance::run`] for details.
    pub fn write(
        mut self,
        buffer: B,
    ) -> impl Future<Output = (Option<StreamWriter<B>>, B)> + Send + 'static
    where
        B: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(self.tx.as_mut().unwrap(), WriteEvent::Write { buffer, tx });
        let instance = self.instance;
        super::checked(
            instance,
            rx.map(move |v| match v {
                Ok(HostResult { buffer, dropped }) => ((!dropped).then_some(self), buffer),
                Err(_) => todo!("guarantee buffer recovery if event loop errors or panics"),
            }),
        )
    }

    /// Write the specified values until either the buffer is drained or the
    /// read end is dropped.
    ///
    /// The returned `Future` will yield a `(Some(_), _)` if the write completed
    /// (i.e. all the items were accepted).  It will return `(None, _)` if the
    /// write failed due to the closure of the read end.  In either case, the
    /// returned buffer will be the same one passed as a parameter, possibly
    /// mutated to consume any written values.
    ///
    /// Note that the returned `Future` must be polled from the event loop of
    /// the component instance from which this `StreamWriter` originated.  See
    /// [`Instance::run`] for details.
    pub fn write_all<T>(
        self,
        buffer: B,
    ) -> impl Future<Output = (Option<StreamWriter<B>>, B)> + Send + 'static
    where
        B: WriteBuffer<T>,
    {
        let instance = self.instance;
        super::checked(
            instance,
            self.write(buffer).then(|(me, buffer)| async move {
                if let Some(me) = me {
                    if buffer.remaining().len() > 0 {
                        // Note the use of `Box::pin` which is required due to
                        // the recursive nature of this function.
                        Box::pin(me.write_all(buffer)).await
                    } else {
                        (Some(me), buffer)
                    }
                } else {
                    (None, buffer)
                }
            }),
        )
    }

    /// Convert this object into a `Future` which will resolve when the read end
    /// of this `stream` is dropped, plus a `Watch` which can be used to retrieve
    /// the `StreamWriter` again.
    ///
    /// Note that calling `Watch::into_inner` on the returned `Watch` will have
    /// the side effect of causing the `Future` to resolve immediately if it
    /// hasn't already.
    ///
    /// Also note that the returned `Future` must be polled from the event loop
    /// of the component instance from which this `StreamWriter` originated.
    /// See [`Instance::run`] for details.
    pub fn watch_reader(mut self) -> (impl Future<Output = ()> + Send + 'static, Watch<Self>)
    where
        B: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(&mut self.tx.as_mut().unwrap(), WriteEvent::Watch { tx });
        let instance = self.instance;
        watch(instance, rx, self)
    }
}

impl<T> Drop for StreamWriter<T> {
    fn drop(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            send(&mut tx, WriteEvent::Drop(None));
        }
    }
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
    instance: Instance,
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> HostStream<T> {
    /// Create a new `HostStream`.
    fn new(rep: u32, instance: Instance) -> Self {
        Self {
            instance,
            rep,
            _phantom: PhantomData,
        }
    }

    /// Convert this object into a [`StreamReader`].
    pub fn into_reader<B>(self, mut store: impl AsContextMut) -> StreamReader<B>
    where
        T: func::Lower + func::Lift + Send + 'static,
        B: ReadBuffer<T>,
    {
        StreamReader {
            instance: self.instance,
            rep: self.rep,
            tx: Some(self.instance.start_read_event_loop(
                store.as_context_mut(),
                self.rep,
                TransmitKind::Stream,
            )),
        }
    }

    /// Convert this `HostStream` into a [`Val`].
    // See TODO comment for `StreamAny`; this is prone to handle leakage.
    pub fn into_val(self) -> Val {
        Val::Stream(StreamAny(self.rep))
    }

    /// Attempt to convert the specified [`Val`] to a `HostStream`.
    pub fn from_val(
        mut store: impl AsContextMut<Data: Send>,
        instance: Instance,
        value: &Val,
    ) -> Result<Self> {
        let Val::Stream(StreamAny(rep)) = value else {
            bail!("expected `stream`; got `{}`", value.desc());
        };
        let store = store.as_context_mut();
        instance
            .concurrent_state_mut(store.0)
            .get(TableId::<TransmitHandle>::new(*rep))?; // Just make sure it's present
        Ok(Self::new(*rep, instance))
    }

    /// Transfer ownership of the read end of a stream from a guest to the host.
    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Stream(src) => {
                let state_table = cx
                    .instance_mut()
                    .concurrent_state_mut()
                    .state_table(TableIndex::Stream(src));
                let (rep, state) =
                    get_mut_by_index_from(state_table, TableIndex::Stream(src), index)?;

                match state {
                    StreamFutureState::Read { done: true } => bail!(
                        "cannot lift stream after being notified that the writable end dropped"
                    ),
                    StreamFutureState::Read { done: false } => {
                        state_table.remove_by_index(index)?;
                    }
                    StreamFutureState::Write { .. } => bail!("cannot transfer write end of stream"),
                    StreamFutureState::Busy => bail!("cannot transfer busy stream"),
                }

                Ok(Self::new(rep, cx.instance_handle()))
            }
            _ => func::bad_type_info(),
        }
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
            let state = concurrent_state
                .get(TableId::<TransmitHandle>::new(rep))?
                .state;
            let rep = concurrent_state.get(state)?.read_handle.rep();

            concurrent_state
                .state_table(TableIndex::Stream(dst))
                .insert(
                    rep,
                    WaitableState::Stream(dst, StreamFutureState::Read { done: false }),
                )
        }
        _ => func::bad_type_info(),
    }
}

// SAFETY: This relies on the `ComponentType` implementation for `u32` being
// safe and correct since we lift and lower stream handles as `u32`s.
unsafe impl<T> func::ComponentType for HostStream<T> {
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
unsafe impl<T> func::Lower for HostStream<T> {
    fn linear_lower_to_flat<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        lower_stream_to_index(self.rep, cx, ty)?.linear_lower_to_flat(cx, InterfaceType::U32, dst)
    }

    fn linear_lower_to_memory<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        lower_stream_to_index(self.rep, cx, ty)?.linear_lower_to_memory(
            cx,
            InterfaceType::U32,
            offset,
        )
    }
}

// SAFETY: See the comment on the `ComponentType` `impl` for this type.
unsafe impl<T> func::Lift for HostStream<T> {
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

impl<T, B> From<StreamReader<B>> for HostStream<T> {
    fn from(mut value: StreamReader<B>) -> Self {
        value.tx.take();

        Self {
            instance: value.instance,
            rep: value.rep,
            _phantom: PhantomData,
        }
    }
}

/// Represents the readable end of a Component Model `stream`.
///
/// In order to pass this end to guest code, first convert it to a
/// [`HostStream`] using the `into` method.
pub struct StreamReader<B> {
    instance: Instance,
    rep: u32,
    tx: Option<mpsc::Sender<ReadEvent<B>>>,
}

impl<B> StreamReader<B> {
    fn new(rep: u32, tx: Option<mpsc::Sender<ReadEvent<B>>>, instance: Instance) -> Self {
        Self { instance, rep, tx }
    }

    /// Read values from this `stream`.
    ///
    /// The returned `Future` will yield a `(Some(_), _)` if the read completed
    /// (possibly with zero items if the write was empty).  It will return
    /// `(None, _)` if the read failed due to the closure of the write end.  In
    /// either case, the returned buffer will be the same one passed as a
    /// parameter, with zero or more items added.
    ///
    /// Note that the returned `Future` must be polled from the event loop of
    /// the component instance from which this `StreamReader` originated.  See
    /// [`Instance::run`] for details.
    pub fn read(
        mut self,
        buffer: B,
    ) -> impl Future<Output = (Option<StreamReader<B>>, B)> + Send + 'static
    where
        B: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(self.tx.as_mut().unwrap(), ReadEvent::Read { buffer, tx });
        let instance = self.instance;
        super::checked(
            instance,
            rx.map(move |v| match v {
                Ok(HostResult { buffer, dropped }) => ((!dropped).then_some(self), buffer),
                Err(_) => {
                    todo!("guarantee buffer recovery if event loop errors or panics")
                }
            }),
        )
    }

    /// Convert this object into a `Future` which will resolve when the write
    /// end of this `stream` is dropped, plus a `Watch` which can be used to
    /// retrieve the `StreamReader` again.
    ///
    /// Note that calling `Watch::into_inner` on the returned `Watch` will have
    /// the side effect of causing the `Future` to resolve immediately if it
    /// hasn't already.
    ///
    /// Also note that the returned `Future` must be polled from the event loop
    /// of the component instance from which this `StreamReader` originated.
    /// See [`Instance::run`] for details.
    pub fn watch_writer(mut self) -> (impl Future<Output = ()> + Send + 'static, Watch<Self>)
    where
        B: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        send(&mut self.tx.as_mut().unwrap(), ReadEvent::Watch { tx });
        let instance = self.instance;
        watch(instance, rx, self)
    }
}

impl<B> Drop for StreamReader<B> {
    fn drop(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            send(&mut tx, ReadEvent::Drop);
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
                let (rep, _) = cx
                    .instance_mut()
                    .concurrent_state_mut()
                    .error_context_tables()
                    .get_mut(src)
                    .expect("error context table index present in (sub)component table during lift")
                    .get_mut_by_index(index)?;

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
            let tbl = cx
                .instance_mut()
                .concurrent_state_mut()
                .error_context_tables()
                .get_mut(dst)
                .expect("error context table index present in (sub)component table during lower");

            if let Some((dst_idx, dst_state)) = tbl.get_mut_by_rep(rep) {
                dst_state.0 += 1;
                Ok(dst_idx)
            } else {
                tbl.insert(rep, LocalErrorContextRefCount(1))
            }
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
    /// The `Sender`, if any, to be dropped when the write end of the stream or
    /// future is dropped.
    ///
    /// This will signal to the host-owned read end that the write end has been
    /// dropped.
    writer_watcher: Option<oneshot::Sender<()>>,
    /// Like `writer_watcher`, but for the reverse direction.
    reader_watcher: Option<oneshot::Sender<()>>,
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
            reader_watcher: None,
            writer_watcher: None,
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
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        handle: u32,
        post_write: PostWrite,
    },
    /// The write end is owned by a host task and a write is pending.
    HostReady {
        accept:
            Box<dyn FnOnce(&mut dyn VMStore, Instance, Reader) -> Result<ReturnCode> + Send + Sync>,
        post_write: PostWrite,
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
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        handle: u32,
    },
    /// The read end is owned by a host task and a read is pending.
    HostReady {
        accept: Box<dyn FnOnce(Writer) -> Result<ReturnCode> + Send + Sync>,
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
            Self::Dropped => f.debug_tuple("Dropped").finish(),
        }
    }
}

/// Parameter type to pass to a `ReadState::HostReady` closure.
///
/// See also `accept_writer`.
enum Writer<'a> {
    /// The write end is owned by a guest task.
    Guest {
        lift: &'a mut LiftContext<'a>,
        ty: Option<InterfaceType>,
        address: usize,
        count: usize,
    },
    /// The write end is owned by the host.
    Host {
        buffer: &'a mut dyn TakeBuffer,
        count: usize,
    },
    /// The write end has been dropped.
    End,
}

/// Parameter type to pass to a `WriteState::HostReady` closure.
///
/// See also `accept_reader`.
enum Reader<'a> {
    /// The read end is owned by a guest task.
    Guest {
        options: &'a Options,
        ty: TableIndex,
        address: usize,
        count: usize,
    },
    /// The read end is owned by the host.
    Host {
        accept: Box<dyn FnOnce(&mut dyn TakeBuffer, usize) -> usize>,
    },
    /// The read end has been dropped.
    End,
}

impl Instance {
    /// Create a new Component Model `future` as pair of writable and readable ends,
    /// the latter of which may be passed to guest code.
    ///
    /// The `default` parameter will be used if the returned `FutureWriter` is
    /// dropped before `FutureWriter::write` is called.  Since the write end of
    /// a Component Model `future` must be written to before it is dropped, and
    /// since Rust does not currently provide a way to statically enforce that
    /// (e.g. linear typing), we use this mechanism to ensure a value is always
    /// written prior to closing.
    ///
    /// If there's no plausible default value, and you're sure
    /// `FutureWriter::write` will be called, you can consider passing `||
    /// unreachable!()` as the `default` parameter.
    pub fn future<T: func::Lower + func::Lift + Send + Sync + 'static>(
        self,
        default: fn() -> T,
        mut store: impl AsContextMut,
    ) -> Result<(FutureWriter<T>, FutureReader<T>)> {
        let mut store = store.as_context_mut();
        let (write, read) = self.concurrent_state_mut(store.0).new_transmit()?;

        Ok((
            FutureWriter::new(
                default,
                Some(self.start_write_event_loop(
                    store.as_context_mut(),
                    write.rep(),
                    TransmitKind::Future,
                )),
                self,
            ),
            FutureReader::new(
                read.rep(),
                Some(self.start_read_event_loop(
                    store.as_context_mut(),
                    read.rep(),
                    TransmitKind::Future,
                )),
                self,
            ),
        ))
    }

    /// Create a new Component Model `stream` as pair of writable and readable ends,
    /// the latter of which may be passed to guest code.
    pub fn stream<
        T: func::Lower + func::Lift + Send + 'static,
        W: WriteBuffer<T>,
        R: ReadBuffer<T>,
    >(
        self,
        mut store: impl AsContextMut,
    ) -> Result<(StreamWriter<W>, StreamReader<R>)> {
        let mut store = store.as_context_mut();
        let (write, read) = self.concurrent_state_mut(store.0).new_transmit()?;

        Ok((
            StreamWriter::new(
                Some(self.start_write_event_loop(
                    store.as_context_mut(),
                    write.rep(),
                    TransmitKind::Stream,
                )),
                self,
            ),
            StreamReader::new(
                read.rep(),
                Some(self.start_read_event_loop(
                    store.as_context_mut(),
                    read.rep(),
                    TransmitKind::Stream,
                )),
                self,
            ),
        ))
    }

    /// Spawn a background task to be polled in this instance's event loop.
    ///
    /// The spawned task will accept host events from the `Receiver` corresponding to
    /// the returned `Sender`, handling each event it receives and then exiting
    /// when the channel is dropped.
    ///
    /// We handle `StreamWriter` and `FutureWriter` operations this way so that
    /// they can be initiated without access to the store and possibly outside
    /// the instance's event loop, improving the ergonmics for host embedders.
    fn start_write_event_loop<
        T: func::Lower + func::Lift + Send + 'static,
        B: WriteBuffer<T>,
        U,
    >(
        self,
        mut store: StoreContextMut<U>,
        rep: u32,
        kind: TransmitKind,
    ) -> mpsc::Sender<WriteEvent<B>> {
        let (tx, mut rx) = mpsc::channel(1);
        let id = TableId::<TransmitHandle>::new(rep);
        let run_on_drop =
            RunOnDrop::new(move || log::trace!("write event loop for {id:?} dropped"));
        let token = StoreToken::new(store.as_context_mut());
        let task = Box::pin(
            async move {
                log::trace!("write event loop for {id:?} started");
                let mut my_rep = None;
                while let Some(event) = rx.next().await {
                    if my_rep.is_none() {
                        my_rep = Some(self.get_state_rep(rep)?);
                    }
                    let rep = my_rep.unwrap();
                    match event {
                        WriteEvent::Write { buffer, tx } => tls::get(|store| {
                            self.host_write::<_, _, U>(
                                token.as_context_mut(store),
                                rep,
                                buffer,
                                PostWrite::Continue,
                                tx,
                                kind,
                            )
                        })?,
                        WriteEvent::Drop(default) => tls::get(|store| {
                            if let Some(default) = default {
                                self.host_write::<_, _, U>(
                                    token.as_context_mut(store),
                                    rep,
                                    default(),
                                    PostWrite::Continue,
                                    oneshot::channel().0,
                                    kind,
                                )?;
                            }
                            self.concurrent_state_mut(store).host_drop_writer(rep, kind)
                        })?,
                        WriteEvent::Watch { tx } => tls::get(|store| {
                            let state =
                                self.concurrent_state_mut(store)
                                    .get_mut(TableId::<TransmitState>::new(rep))?;
                            if !matches!(&state.read, ReadState::Dropped) {
                                state.reader_watcher = Some(tx);
                            }
                            Ok::<_, anyhow::Error>(())
                        })?,
                    }
                }
                Ok(())
            }
            .map(move |v| {
                run_on_drop.cancel();
                log::trace!("write event loop for {id:?} finished: {v:?}");
                HostTaskOutput::Result(v)
            }),
        );
        self.concurrent_state_mut(store.0).push_future(task);
        tx
    }

    /// Same as `Self::start_write_event_loop`, but for the read end of a stream
    /// or future.
    fn start_read_event_loop<T: func::Lower + func::Lift + Send + 'static, B: ReadBuffer<T>, U>(
        self,
        mut store: StoreContextMut<U>,
        rep: u32,
        kind: TransmitKind,
    ) -> mpsc::Sender<ReadEvent<B>> {
        let (tx, mut rx) = mpsc::channel(1);
        let id = TableId::<TransmitHandle>::new(rep);
        let run_on_drop = RunOnDrop::new(move || log::trace!("read event loop for {id:?} dropped"));
        let token = StoreToken::new(store.as_context_mut());
        let task = Box::pin(
            async move {
                log::trace!("read event loop for {id:?} started");
                let mut my_rep = None;
                while let Some(event) = rx.next().await {
                    if my_rep.is_none() {
                        my_rep = Some(self.get_state_rep(rep)?);
                    }
                    let rep = my_rep.unwrap();
                    match event {
                        ReadEvent::Read { buffer, tx } => tls::get(|store| {
                            self.host_read::<_, _, U>(
                                token.as_context_mut(store),
                                rep,
                                buffer,
                                tx,
                                kind,
                            )
                        })?,
                        ReadEvent::Drop => {
                            tls::get(|store| self.host_drop_reader(store, rep, kind))?
                        }
                        ReadEvent::Watch { tx } => tls::get(|store| {
                            let state =
                                self.concurrent_state_mut(store)
                                    .get_mut(TableId::<TransmitState>::new(rep))?;
                            if !matches!(
                                &state.write,
                                WriteState::Dropped
                                    | WriteState::GuestReady {
                                        post_write: PostWrite::Drop,
                                        ..
                                    }
                                    | WriteState::HostReady {
                                        post_write: PostWrite::Drop,
                                        ..
                                    }
                            ) {
                                state.writer_watcher = Some(tx);
                            }
                            Ok::<_, anyhow::Error>(())
                        })?,
                    }
                }
                Ok(())
            }
            .map(move |v| {
                run_on_drop.cancel();
                log::trace!("read event loop for {id:?} finished: {v:?}");
                HostTaskOutput::Result(v)
            }),
        );
        self.concurrent_state_mut(store.0).push_future(task);
        tx
    }

    /// Write to the specified stream or future from the host.
    ///
    /// # Arguments
    ///
    /// * `store` - The store to which this instance belongs
    /// * `transmit_rep` - The `TransmitState` rep for the stream or future
    /// * `buffer` - Buffer of values that should be written
    /// * `post_write` - Whether the transmit should be dropped after write, possibly with an error context
    /// * `tx` - Oneshot channel to notify when operation completes (or drop on error)
    /// * `kind` - whether this is a stream or a future
    fn host_write<T: func::Lower + Send + 'static, B: WriteBuffer<T>, U>(
        self,
        mut store: StoreContextMut<U>,
        transmit_rep: u32,
        mut buffer: B,
        mut post_write: PostWrite,
        tx: oneshot::Sender<HostResult<B>>,
        kind: TransmitKind,
    ) -> Result<()> {
        let mut store = store.as_context_mut();
        let transmit_id = TableId::<TransmitState>::new(transmit_rep);
        let transmit = self
            .concurrent_state_mut(store.0)
            .get_mut(transmit_id)
            .with_context(|| format!("retrieving state for transmit [{transmit_rep}]"))?;
        log::trace!("host_write state {transmit_id:?}; {:?}", transmit.read);

        let new_state = if let ReadState::Dropped = &transmit.read {
            ReadState::Dropped
        } else {
            ReadState::Open
        };

        match mem::replace(&mut transmit.read, new_state) {
            ReadState::Open => {
                assert!(matches!(&transmit.write, WriteState::Open));

                let state = WriteState::HostReady {
                    accept: Box::new(accept_reader::<T, B, U>(
                        store.as_context_mut(),
                        buffer,
                        tx,
                        kind,
                    )),
                    post_write,
                };
                self.concurrent_state_mut(store.0)
                    .get_mut(transmit_id)?
                    .write = state;
                post_write = PostWrite::Continue;
            }

            ReadState::GuestReady {
                ty,
                flat_abi: _,
                options,
                address,
                count,
                handle,
                ..
            } => {
                if let TransmitKind::Future = kind {
                    transmit.done = true;
                }

                let read_handle = transmit.read_handle;
                let code = accept_reader::<T, B, U>(store.as_context_mut(), buffer, tx, kind)(
                    store.0.traitobj_mut(),
                    self,
                    Reader::Guest {
                        options: &options,
                        ty,
                        address,
                        count,
                    },
                )?;

                self.concurrent_state_mut(store.0).set_event(
                    read_handle.rep(),
                    match ty {
                        TableIndex::Future(ty) => Event::FutureRead {
                            code,
                            pending: Some((ty, handle)),
                        },
                        TableIndex::Stream(ty) => Event::StreamRead {
                            code,
                            pending: Some((ty, handle)),
                        },
                    },
                )?;
            }

            ReadState::HostReady { accept } => {
                let count = buffer.remaining().len();
                let code = accept(Writer::Host {
                    buffer: &mut buffer,
                    count,
                })?;
                let (ReturnCode::Completed(_) | ReturnCode::Dropped(_)) = code else {
                    unreachable!()
                };

                _ = tx.send(HostResult {
                    buffer,
                    dropped: false,
                });
            }

            ReadState::Dropped => {
                _ = tx.send(HostResult {
                    buffer,
                    dropped: true,
                });
            }
        }

        if let PostWrite::Drop = post_write {
            self.concurrent_state_mut(store.0)
                .host_drop_writer(transmit_rep, kind)?;
        }

        Ok(())
    }

    /// Read from the specified stream or future from the host.
    ///
    /// # Arguments
    ///
    /// * `store` - The store to which this instance belongs
    /// * `rep` - The `TransmitState` rep for the stream or future
    /// * `buffer` - Buffer to receive values
    /// * `tx` - Oneshot channel to notify when operation completes (or drop on error)
    /// * `kind` - whether this is a stream or a future
    fn host_read<T: func::Lift + Send + 'static, B: ReadBuffer<T>, U>(
        self,
        mut store: StoreContextMut<U>,
        rep: u32,
        mut buffer: B,
        tx: oneshot::Sender<HostResult<B>>,
        kind: TransmitKind,
    ) -> Result<()> {
        let store = store.as_context_mut();
        let transmit_id = TableId::<TransmitState>::new(rep);
        let transmit = self
            .concurrent_state_mut(store.0)
            .get_mut(transmit_id)
            .with_context(|| rep.to_string())?;
        log::trace!("host_read state {transmit_id:?}; {:?}", transmit.write);

        let new_state = if let WriteState::Dropped = &transmit.write {
            WriteState::Dropped
        } else {
            WriteState::Open
        };

        match mem::replace(&mut transmit.write, new_state) {
            WriteState::Open => {
                assert!(matches!(&transmit.read, ReadState::Open));

                transmit.read = ReadState::HostReady {
                    accept: Box::new(accept_writer::<T, B, U>(buffer, tx, kind)),
                };
            }

            WriteState::GuestReady {
                ty,
                flat_abi: _,
                options,
                address,
                count,
                handle,
                post_write,
                ..
            } => {
                if let TableIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let write_handle = transmit.write_handle;
                let types = self.id().get(store.0).component().types().clone();
                let lift =
                    &mut LiftContext::new(store.0.store_opaque_mut(), &options, &types, self);
                let code = accept_writer::<T, B, U>(buffer, tx, kind)(Writer::Guest {
                    lift,
                    ty: payload(ty, &types),
                    address,
                    count,
                })?;

                let state = self.concurrent_state_mut(store.0);
                let pending = if let PostWrite::Drop = post_write {
                    state.get_mut(transmit_id)?.write = WriteState::Dropped;
                    false
                } else {
                    true
                };

                state.set_event(
                    write_handle.rep(),
                    match ty {
                        TableIndex::Future(ty) => Event::FutureWrite {
                            code,
                            pending: pending.then_some((ty, handle)),
                        },
                        TableIndex::Stream(ty) => Event::StreamWrite {
                            code,
                            pending: pending.then_some((ty, handle)),
                        },
                    },
                )?;
            }

            WriteState::HostReady { accept, post_write } => {
                accept(
                    store.0.traitobj_mut(),
                    self,
                    Reader::Host {
                        accept: Box::new(move |input, count| {
                            let count = count.min(buffer.remaining_capacity());
                            buffer.move_from(input, count);
                            _ = tx.send(HostResult {
                                buffer,
                                dropped: false,
                            });
                            count
                        }),
                    },
                )?;

                if let PostWrite::Drop = post_write {
                    self.concurrent_state_mut(store.0)
                        .get_mut(transmit_id)?
                        .write = WriteState::Dropped;
                }
            }

            WriteState::Dropped => {
                _ = tx.send(HostResult {
                    buffer,
                    dropped: true,
                });
            }
        }

        Ok(())
    }

    /// Drop the read end of a stream or future read from the host.
    ///
    /// # Arguments
    ///
    /// * `store` - The store to which this instance belongs
    /// * `transmit_rep` - The `TransmitState` rep for the stream or future.
    fn host_drop_reader(
        self,
        store: &mut dyn VMStore,
        transmit_rep: u32,
        kind: TransmitKind,
    ) -> Result<()> {
        let transmit_id = TableId::<TransmitState>::new(transmit_rep);
        let state = self.concurrent_state_mut(store);
        let transmit = state
            .get_mut(transmit_id)
            .with_context(|| format!("error closing reader {transmit_rep}"))?;
        log::trace!(
            "host_drop_reader state {transmit_id:?}; read state {:?} write state {:?}",
            transmit.read,
            transmit.write
        );

        transmit.read = ReadState::Dropped;
        transmit.reader_watcher = None;

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
            WriteState::GuestReady {
                ty,
                handle,
                post_write,
                ..
            } => {
                if let PostWrite::Drop = post_write {
                    state.delete_transmit(transmit_id)?;
                } else {
                    state.update_event(
                        write_handle.rep(),
                        match ty {
                            TableIndex::Future(ty) => Event::FutureWrite {
                                code: ReturnCode::Dropped(0),
                                pending: Some((ty, handle)),
                            },
                            TableIndex::Stream(ty) => Event::StreamWrite {
                                code: ReturnCode::Dropped(0),
                                pending: Some((ty, handle)),
                            },
                        },
                    )?;
                };
            }

            WriteState::HostReady { accept, .. } => {
                accept(store, self, Reader::End)?;
            }

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
                log::trace!("host_drop_reader delete {transmit_rep}");
                state.delete_transmit(transmit_id)?;
            }
        }
        Ok(())
    }

    /// Copy `count` items from `read_address` to `write_address` for the
    /// specified stream or future.
    fn copy<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        flat_abi: Option<FlatAbi>,
        write_ty: TableIndex,
        write_options: &Options,
        write_address: usize,
        read_ty: TableIndex,
        read_options: &Options,
        read_address: usize,
        count: usize,
        rep: u32,
    ) -> Result<()> {
        let types = self.id().get(store.0).component().types().clone();
        match (write_ty, read_ty) {
            (TableIndex::Future(write_ty), TableIndex::Future(read_ty)) => {
                assert_eq!(count, 1);

                let val = types[types[write_ty].ty]
                    .payload
                    .map(|ty| {
                        let abi = types.canonical_abi(&ty);
                        // FIXME: needs to read an i64 for memory64
                        if write_address % usize::try_from(abi.align32)? != 0 {
                            bail!("write pointer not aligned");
                        }

                        let lift = &mut LiftContext::new(
                            store.0.store_opaque_mut(),
                            write_options,
                            &types,
                            self,
                        );
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
            (TableIndex::Stream(write_ty), TableIndex::Stream(read_ty)) => {
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
                    let lift = &mut LiftContext::new(store_opaque, write_options, &types, self);
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

    /// Write to the specified stream or future from the guest.
    ///
    /// SAFETY: `memory` and `realloc` must be valid pointers to their
    /// respective guest entities.
    pub(super) unsafe fn guest_write<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        handle: u32,
        address: u32,
        count: u32,
    ) -> Result<ReturnCode> {
        if !async_ {
            bail!("synchronous stream and future writes not yet supported");
        }

        let address = usize::try_from(address).unwrap();
        let count = usize::try_from(count).unwrap();
        // SAFETY: Per this function's contract, `memory` and `realloc` are
        // valid.
        let options = unsafe {
            Options::new(
                store.0.store_opaque().id(),
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).unwrap(),
                true,
                None,
            )
        };
        let concurrent_state = self.concurrent_state_mut(store.0);
        let (rep, state) = concurrent_state.get_mut_by_index(ty, handle)?;
        let StreamFutureState::Write { done } = *state else {
            bail!(
                "invalid handle {handle}; expected `Write`; got {:?}",
                *state
            );
        };

        if done {
            bail!("cannot write to stream after being notified that the readable end dropped");
        }

        *state = StreamFutureState::Busy;
        let transmit_handle = TableId::<TransmitHandle>::new(rep);
        let transmit_id = concurrent_state.get(transmit_handle)?.state;
        let transmit = concurrent_state.get_mut(transmit_id)?;
        log::trace!(
            "guest_write {transmit_handle:?} (handle {handle}; state {transmit_id:?}); {:?}",
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
                post_write: PostWrite::Continue,
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

                if let TableIndex::Future(_) = ty {
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

                    concurrent_state.set_event(
                        read_handle_rep,
                        match read_ty {
                            TableIndex::Future(ty) => Event::FutureRead {
                                code,
                                pending: Some((ty, read_handle)),
                            },
                            TableIndex::Stream(ty) => Event::StreamRead {
                                code,
                                pending: Some((ty, read_handle)),
                            },
                        },
                    )?;
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

            ReadState::HostReady { accept } => {
                if let TableIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let types = self.id().get(store.0).component().types().clone();
                let lift =
                    &mut LiftContext::new(store.0.store_opaque_mut(), &options, &types, self);
                accept(Writer::Guest {
                    lift,
                    ty: payload(ty, &types),
                    address,
                    count,
                })?
            }

            ReadState::Open => {
                set_guest_ready(concurrent_state)?;
                ReturnCode::Blocked
            }

            ReadState::Dropped => {
                if let TableIndex::Future(_) = ty {
                    transmit.done = true;
                }

                ReturnCode::Dropped(0)
            }
        };

        if result != ReturnCode::Blocked {
            let state = self.concurrent_state_mut(store.0);
            *state.get_mut_by_index(ty, handle)?.1 = StreamFutureState::Write {
                done: matches!(
                    (result, ty),
                    (ReturnCode::Dropped(_), TableIndex::Stream(_))
                ),
            };
        }

        Ok(result)
    }

    /// Read from the specified stream or future from the guest.
    ///
    /// SAFETY: `memory` and `realloc` must be valid pointers to their
    /// respective guest entities.
    pub(super) unsafe fn guest_read<T: 'static>(
        self,
        mut store: StoreContextMut<T>,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        async_: bool,
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        handle: u32,
        address: u32,
        count: u32,
    ) -> Result<ReturnCode> {
        if !async_ {
            bail!("synchronous stream and future reads not yet supported");
        }

        let address = usize::try_from(address).unwrap();
        // SAFETY: Per this function's contract, `memory` and `realloc` must be
        // valid.
        let options = unsafe {
            Options::new(
                store.0.store_opaque().id(),
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).unwrap(),
                true,
                None,
            )
        };
        let concurrent_state = self.concurrent_state_mut(store.0);
        let (rep, state) = concurrent_state.get_mut_by_index(ty, handle)?;
        let StreamFutureState::Read { done } = *state else {
            bail!("invalid handle {handle}; expected `Read`; got {:?}", *state);
        };

        if done {
            bail!("cannot read from stream after being notified that the writable end dropped");
        }

        *state = StreamFutureState::Busy;
        let transmit_handle = TableId::<TransmitHandle>::new(rep);
        let transmit_id = concurrent_state.get(transmit_handle)?.state;
        let transmit = concurrent_state.get_mut(transmit_id)?;
        log::trace!(
            "guest_read {transmit_handle:?} (handle {handle}; state {transmit_id:?}); {:?}",
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
                count: usize::try_from(count).unwrap(),
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
                post_write,
            } => {
                assert_eq!(flat_abi, write_flat_abi);

                if let TableIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let write_handle_rep = transmit.write_handle.rep();

                // See the comment in `guest_write` for the
                // `ReadState::GuestReady` case concerning zero-length reads and
                // writes.

                let count = usize::try_from(count).unwrap();

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
                let pending = if let PostWrite::Drop = post_write {
                    concurrent_state.get_mut(transmit_id)?.write = WriteState::Dropped;
                    false
                } else {
                    true
                };

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

                    concurrent_state.set_event(
                        write_handle_rep,
                        match write_ty {
                            TableIndex::Future(ty) => Event::FutureWrite {
                                code,
                                pending: pending.then_some((ty, write_handle)),
                            },
                            TableIndex::Stream(ty) => Event::StreamWrite {
                                code,
                                pending: pending.then_some((ty, write_handle)),
                            },
                        },
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
                        post_write,
                    };
                }

                if read_complete {
                    ReturnCode::completed(ty.kind(), count.try_into().unwrap())
                } else {
                    set_guest_ready(concurrent_state)?;
                    ReturnCode::Blocked
                }
            }

            WriteState::HostReady { accept, post_write } => {
                if let TableIndex::Future(_) = ty {
                    transmit.done = true;
                }

                let code = accept(
                    store.0.traitobj_mut(),
                    self,
                    Reader::Guest {
                        options: &options,
                        ty,
                        address,
                        count: count.try_into().unwrap(),
                    },
                )?;

                if let PostWrite::Drop = post_write {
                    self.concurrent_state_mut(store.0)
                        .get_mut(transmit_id)?
                        .write = WriteState::Dropped;
                }

                code
            }

            WriteState::Open => {
                set_guest_ready(concurrent_state)?;
                ReturnCode::Blocked
            }

            WriteState::Dropped => ReturnCode::Dropped(0),
        };

        if result != ReturnCode::Blocked {
            let state = self.concurrent_state_mut(store.0);
            *state.get_mut_by_index(ty, handle)?.1 = StreamFutureState::Read {
                done: matches!(
                    (result, ty),
                    (ReturnCode::Dropped(_), TableIndex::Stream(_))
                ),
            };
        }

        Ok(result)
    }

    /// Drop the readable end of the specified stream or future from the guest.
    fn guest_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TableIndex,
        reader: u32,
    ) -> Result<()> {
        let concurrent_state = self.concurrent_state_mut(store);
        let (rep, state) = concurrent_state.state_table(ty).remove_by_index(reader)?;
        let (state, kind) = match state {
            WaitableState::Stream(_, state) => (state, TransmitKind::Stream),
            WaitableState::Future(_, state) => (state, TransmitKind::Future),
            _ => {
                bail!("invalid stream or future handle");
            }
        };
        match state {
            StreamFutureState::Read { .. } => {}
            StreamFutureState::Write { .. } => {
                bail!("passed write end to `{{stream|future}}.drop-readable`")
            }
            StreamFutureState::Busy => bail!("cannot drop busy stream or future"),
        }
        let id = TableId::<TransmitHandle>::new(rep);
        let rep = concurrent_state.get(id)?.state.rep();
        log::trace!("guest_drop_readable: drop reader {id:?}");
        self.host_drop_reader(store, rep, kind)
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
        // SAFETY: Per this function's contract, `memory` and `realloc` must be
        // valid.
        let options = unsafe {
            Options::new(
                store.id(),
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).ok_or_else(|| {
                    anyhow::anyhow!("failed to convert u8 string encoding [{string_encoding}]")
                })?,
                false,
                None,
            )
        };
        let types = self.id().get(store).component().types().clone();
        let lift_ctx = &mut LiftContext::new(store, &options, &types, self);
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
            .global_error_context_ref_counts()
            .insert(global_ref_count_idx, GlobalErrorContextRefCount(1));

        // Error context are tracked both locally (to a single component instance) and globally
        // the counts for both must stay in sync.
        //
        // Here we reflect the newly created global concurrent error context state into the
        // component instance's locally tracked count, along with the appropriate key into the global
        // ref tracking data structures to enable later lookup
        let local_tbl = &mut state.error_context_tables()[ty];

        assert!(
            !local_tbl.has_handle(table_id.rep()),
            "newly created error context state already tracked by component"
        );
        let local_idx = local_tbl.insert(table_id.rep(), LocalErrorContextRefCount(1))?;

        Ok(local_idx)
    }

    /// Retrieve the debug message from the specified error context.
    ///
    /// SAFETY: `memory` and `realloc` must be valid pointers to their
    /// respective guest entities.
    pub(super) unsafe fn error_context_debug_message<T>(
        self,
        store: StoreContextMut<T>,
        memory: *mut VMMemoryDefinition,
        realloc: *mut VMFuncRef,
        string_encoding: u8,
        ty: TypeComponentLocalErrorContextTableIndex,
        err_ctx_handle: u32,
        debug_msg_address: u32,
    ) -> Result<()> {
        // Retrieve the error context and internal debug message
        let id = store.0.store_opaque().id();
        let state = self.concurrent_state_mut(store.0);
        let (state_table_id_rep, _) = state
            .error_context_tables()
            .get_mut(ty)
            .context("error context table index present in (sub)component lookup during debug_msg")?
            .get_mut_by_index(err_ctx_handle)?;

        // Get the state associated with the error context
        let ErrorContextState { debug_msg } =
            state.get_mut(TableId::<ErrorContextState>::new(state_table_id_rep))?;
        let debug_msg = debug_msg.clone();

        // SAFETY: Per this function's contract, `memory` and `realloc` are
        // valid.
        let options = unsafe {
            Options::new(
                id,
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).ok_or_else(|| {
                    anyhow::anyhow!("failed to convert u8 string encoding [{string_encoding}]")
                })?,
                false,
                None,
            )
        };
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
        self.guest_drop_readable(store, TableIndex::Future(ty), reader)
    }

    /// Implements the `stream.drop-readable` intrinsic.
    pub(crate) fn stream_drop_readable(
        self,
        store: &mut dyn VMStore,
        ty: TypeStreamTableIndex,
        reader: u32,
    ) -> Result<()> {
        self.guest_drop_readable(store, TableIndex::Stream(ty), reader)
    }

    /// Retrieve the `TransmitState` rep for the specified `TransmitHandle` rep.
    fn get_state_rep(&self, rep: u32) -> Result<u32> {
        tls::get(|store| {
            let transmit_handle = TableId::<TransmitHandle>::new(rep);
            Ok(self
                .concurrent_state_mut(store)
                .get(transmit_handle)
                .with_context(|| format!("stream or future {transmit_handle:?} not found"))?
                .state
                .rep())
        })
    }
}

/// Helper struct for running a closure on drop, e.g. for logging purposes.
struct RunOnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> RunOnDrop<F> {
    fn new(fun: F) -> Self {
        Self(Some(fun))
    }

    fn cancel(mut self) {
        self.0 = None;
    }
}

impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(fun) = self.0.take() {
            fun();
        }
    }
}

impl ConcurrentState {
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

    fn get_mut_by_index(
        &mut self,
        ty: TableIndex,
        index: u32,
    ) -> Result<(u32, &mut StreamFutureState)> {
        get_mut_by_index_from(self.state_table(ty), ty, index)
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

    fn state_table(&mut self, ty: TableIndex) -> &mut StateTable<WaitableState> {
        let runtime_instance = match ty {
            TableIndex::Stream(ty) => self.component.types()[ty].instance,
            TableIndex::Future(ty) => self.component.types()[ty].instance,
        };
        &mut self.waitable_tables()[runtime_instance]
    }

    /// Allocate a new future or stream and grant ownership of both the read and
    /// write ends to the (sub-)component instance to which the specified
    /// `TableIndex` belongs.
    fn guest_new(&mut self, ty: TableIndex) -> Result<ResourcePair> {
        let (write, read) = self.new_transmit()?;
        let read = self.state_table(ty).insert(
            read.rep(),
            waitable_state(ty, StreamFutureState::Read { done: false }),
        )?;
        let write = self.state_table(ty).insert(
            write.rep(),
            waitable_state(ty, StreamFutureState::Write { done: false }),
        )?;
        Ok(ResourcePair { write, read })
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
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.get_mut(transmit_id)?;

        match &transmit.write {
            WriteState::GuestReady { .. } | WriteState::HostReady { .. } => {
                transmit.write = WriteState::Open;
            }

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
        } else {
            ReturnCode::Cancelled(0)
        };

        let transmit = self.get_mut(transmit_id)?;

        match &transmit.read {
            ReadState::GuestReady { .. } | ReadState::HostReady { .. } => {
                transmit.read = ReadState::Open;
            }

            ReadState::Open | ReadState::Dropped => {}
        }

        log::trace!("cancelled read {transmit_id:?}");

        Ok(code)
    }

    /// Drop the write end of a stream or future read from the host.
    ///
    /// # Arguments
    ///
    /// * `transmit_rep` - The `TransmitState` rep for the stream or future.
    fn host_drop_writer(&mut self, transmit_rep: u32, kind: TransmitKind) -> Result<()> {
        let transmit_id = TableId::<TransmitState>::new(transmit_rep);
        let transmit = self
            .get_mut(transmit_id)
            .with_context(|| format!("error closing writer {transmit_rep}"))?;
        log::trace!(
            "host_drop_writer state {transmit_id:?}; write state {:?} read state {:?}",
            transmit.read,
            transmit.write
        );

        transmit.writer_watcher = None;

        // Existing queued transmits must be updated with information for the impending writer closure
        match &mut transmit.write {
            WriteState::GuestReady { post_write, .. } => {
                *post_write = PostWrite::Drop;
            }
            WriteState::HostReady { post_write, .. } => {
                *post_write = PostWrite::Drop;
            }
            v @ WriteState::Open => {
                if let (TransmitKind::Future, false) = (
                    kind,
                    transmit.done || matches!(transmit.read, ReadState::Dropped),
                ) {
                    bail!("cannot drop future write end without first writing a value")
                }

                *v = WriteState::Dropped;
            }
            WriteState::Dropped => unreachable!("write state is already dropped"),
        }

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
            // If the guest was ready to read, then we cannot drop the reader (or writer)
            // we must deliver the event, and update the state associated with the handle to
            // represent that a read must be performed
            ReadState::GuestReady { ty, handle, .. } => {
                // Ensure the final read of the guest is queued, with appropriate closure indicator
                self.update_event(
                    read_handle.rep(),
                    match ty {
                        TableIndex::Future(ty) => Event::FutureRead {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                        TableIndex::Stream(ty) => Event::StreamRead {
                            code: ReturnCode::Dropped(0),
                            pending: Some((ty, handle)),
                        },
                    },
                )?;
            }

            // If the host was ready to read, and the writer end is being dropped (host->host write?)
            // signal to the reader that we've reached the end of the stream
            ReadState::HostReady { accept } => {
                accept(Writer::End)?;
            }

            // If the read state is open, then there are no registered readers of the stream/future
            ReadState::Open => {
                self.update_event(
                    read_handle.rep(),
                    match kind {
                        TransmitKind::Future => Event::FutureRead {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                        TransmitKind::Stream => Event::StreamRead {
                            code: ReturnCode::Dropped(0),
                            pending: None,
                        },
                    },
                )?;
            }

            // If the read state was already dropped, then we can remove the transmit state completely
            // (both writer and reader have been dropped)
            ReadState::Dropped => {
                log::trace!("host_drop_writer delete {transmit_rep}");
                self.delete_transmit(transmit_id)?;
            }
        }
        Ok(())
    }

    /// Cancel a pending write for the specified stream or future from the guest.
    fn guest_cancel_write(
        &mut self,
        ty: TableIndex,
        writer: u32,
        _async_: bool,
    ) -> Result<ReturnCode> {
        let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
            self.state_table(ty).get_mut_by_index(writer)?
        else {
            bail!("invalid stream or future handle");
        };
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel write {id:?} (handle {writer})");
        match state {
            StreamFutureState::Write { .. } => {
                bail!("stream or future write cancelled when no write is pending")
            }
            StreamFutureState::Read { .. } => {
                bail!("passed read end to `{{stream|future}}.cancel-write`")
            }
            StreamFutureState::Busy => {
                *state = StreamFutureState::Write { done: false };
            }
        }
        let rep = self.get(id)?.state.rep();
        self.host_cancel_write(rep)
    }

    /// Cancel a pending read for the specified stream or future from the guest.
    fn guest_cancel_read(
        &mut self,
        ty: TableIndex,
        reader: u32,
        _async_: bool,
    ) -> Result<ReturnCode> {
        let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
            self.state_table(ty).get_mut_by_index(reader)?
        else {
            bail!("invalid stream or future handle");
        };
        let id = TableId::<TransmitHandle>::new(rep);
        log::trace!("guest cancel read {id:?} (handle {reader})");
        match state {
            StreamFutureState::Read { .. } => {
                bail!("stream or future read cancelled when no read is pending")
            }
            StreamFutureState::Write { .. } => {
                bail!("passed write end to `{{stream|future}}.cancel-read`")
            }
            StreamFutureState::Busy => {
                *state = StreamFutureState::Read { done: false };
            }
        }
        let rep = self.get(id)?.state.rep();
        self.host_cancel_read(rep)
    }

    /// Drop the writable end of the specified stream or future from the guest.
    fn guest_drop_writable(&mut self, ty: TableIndex, writer: u32) -> Result<()> {
        let (transmit_rep, state) = self
            .state_table(ty)
            .remove_by_index(writer)
            .context("failed to find writer")?;
        let (state, kind) = match state {
            WaitableState::Stream(_, state) => (state, TransmitKind::Stream),
            WaitableState::Future(_, state) => (state, TransmitKind::Future),
            _ => {
                bail!("invalid stream or future handle");
            }
        };
        match state {
            StreamFutureState::Write { .. } => {}
            StreamFutureState::Read { .. } => {
                bail!("passed read end to `{{stream|future}}.drop-writable`")
            }
            StreamFutureState::Busy => bail!("cannot drop busy stream or future"),
        }

        let id = TableId::<TransmitHandle>::new(transmit_rep);
        let transmit_rep = self.get(id)?.state.rep();
        log::trace!("guest_drop_writable: drop writer {id:?}");
        self.host_drop_writer(transmit_rep, kind)
    }

    /// Drop the specified error context.
    pub(crate) fn error_context_drop(
        &mut self,
        ty: TypeComponentLocalErrorContextTableIndex,
        error_context: u32,
    ) -> Result<()> {
        let local_state_table = self
            .error_context_tables()
            .get_mut(ty)
            .context("error context table index present in (sub)component table during drop")?;

        // Reduce the local (sub)component ref count, removing tracking if necessary
        let (rep, local_ref_removed) = {
            let (rep, LocalErrorContextRefCount(local_ref_count)) =
                local_state_table.get_mut_by_index(error_context)?;
            assert!(*local_ref_count > 0);
            *local_ref_count -= 1;
            let mut local_ref_removed = false;
            if *local_ref_count == 0 {
                local_ref_removed = true;
                local_state_table
                    .remove_by_index(error_context)
                    .context("removing error context from component-local tracking")?;
            }
            (rep, local_ref_removed)
        };

        let global_ref_count_idx = TypeComponentGlobalErrorContextTableIndex::from_u32(rep);

        let GlobalErrorContextRefCount(global_ref_count) = self
            .global_error_context_ref_counts()
            .get_mut(&global_ref_count_idx)
            .expect("retrieve concurrent state for error context during drop");

        // Reduce the component-global ref count, removing tracking if necessary
        assert!(*global_ref_count >= 1);
        *global_ref_count -= 1;
        if *global_ref_count == 0 {
            assert!(local_ref_removed);

            self.global_error_context_ref_counts()
                .remove(&global_ref_count_idx);

            self.delete(TableId::<ErrorContextState>::new(rep))
                .context("deleting component-global error context data")?;
        }

        Ok(())
    }

    /// Transfer ownership of the specified stream or future read end from one
    /// guest to another.
    fn guest_transfer<U: PartialEq + Eq + std::fmt::Debug>(
        &mut self,
        src_idx: u32,
        src: U,
        src_instance: RuntimeComponentInstanceIndex,
        dst: U,
        dst_instance: RuntimeComponentInstanceIndex,
        match_state: impl Fn(&mut WaitableState) -> Result<(U, &mut StreamFutureState)>,
        make_state: impl Fn(U, StreamFutureState) -> WaitableState,
    ) -> Result<u32> {
        let src_table = &mut self.waitable_tables()[src_instance];
        let (_rep, src_state) = src_table.get_mut_by_index(src_idx)?;
        let (src_ty, _) = match_state(src_state)?;
        if src_ty != src {
            bail!("invalid future handle");
        }

        let src_table = &mut self.waitable_tables()[src_instance];
        let (rep, src_state) = src_table.get_mut_by_index(src_idx)?;
        let (_, src_state) = match_state(src_state)?;

        match src_state {
            StreamFutureState::Read { done: true } => {
                bail!("cannot lift stream after being notified that the writable end dropped")
            }
            StreamFutureState::Read { done: false } => {
                src_table.remove_by_index(src_idx)?;

                let dst_table = &mut self.waitable_tables()[dst_instance];
                dst_table.insert(
                    rep,
                    make_state(dst, StreamFutureState::Read { done: false }),
                )
            }
            StreamFutureState::Write { .. } => {
                bail!("cannot transfer write end of stream or future")
            }
            StreamFutureState::Busy => bail!("cannot transfer busy stream or future"),
        }
    }

    /// Implements the `future.new` intrinsic.
    pub(crate) fn future_new(&mut self, ty: TypeFutureTableIndex) -> Result<ResourcePair> {
        self.guest_new(TableIndex::Future(ty))
    }

    /// Implements the `future.cancel-write` intrinsic.
    pub(crate) fn future_cancel_write(
        &mut self,
        ty: TypeFutureTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(TableIndex::Future(ty), writer, async_)
            .map(|result| result.encode())
    }

    /// Implements the `future.cancel-read` intrinsic.
    pub(crate) fn future_cancel_read(
        &mut self,
        ty: TypeFutureTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(TableIndex::Future(ty), reader, async_)
            .map(|result| result.encode())
    }

    /// Implements the `future.drop-writable` intrinsic.
    pub(crate) fn future_drop_writable(
        &mut self,
        ty: TypeFutureTableIndex,
        writer: u32,
    ) -> Result<()> {
        self.guest_drop_writable(TableIndex::Future(ty), writer)
    }

    /// Implements the `stream.new` intrinsic.
    pub(crate) fn stream_new(&mut self, ty: TypeStreamTableIndex) -> Result<ResourcePair> {
        self.guest_new(TableIndex::Stream(ty))
    }

    /// Implements the `stream.cancel-write` intrinsic.
    pub(crate) fn stream_cancel_write(
        &mut self,
        ty: TypeStreamTableIndex,
        async_: bool,
        writer: u32,
    ) -> Result<u32> {
        self.guest_cancel_write(TableIndex::Stream(ty), writer, async_)
            .map(|result| result.encode())
    }

    /// Implements the `stream.cancel-read` intrinsic.
    pub(crate) fn stream_cancel_read(
        &mut self,
        ty: TypeStreamTableIndex,
        async_: bool,
        reader: u32,
    ) -> Result<u32> {
        self.guest_cancel_read(TableIndex::Stream(ty), reader, async_)
            .map(|result| result.encode())
    }

    /// Implements the `stream.drop-writable` intrinsic.
    pub(crate) fn stream_drop_writable(
        &mut self,
        ty: TypeStreamTableIndex,
        writer: u32,
    ) -> Result<()> {
        self.guest_drop_writable(TableIndex::Stream(ty), writer)
    }

    /// Transfer ownership of the specified future read end from one guest to
    /// another.
    pub(crate) fn future_transfer(
        &mut self,
        src_idx: u32,
        src: TypeFutureTableIndex,
        dst: TypeFutureTableIndex,
    ) -> Result<u32> {
        self.guest_transfer(
            src_idx,
            src,
            self.component.types()[src].instance,
            dst,
            self.component.types()[dst].instance,
            |state| {
                if let WaitableState::Future(ty, state) = state {
                    Ok((*ty, state))
                } else {
                    Err(anyhow!("invalid future handle"))
                }
            },
            WaitableState::Future,
        )
    }

    /// Transfer ownership of the specified stream read end from one guest to
    /// another.
    pub(crate) fn stream_transfer(
        &mut self,
        src_idx: u32,
        src: TypeStreamTableIndex,
        dst: TypeStreamTableIndex,
    ) -> Result<u32> {
        self.guest_transfer(
            src_idx,
            src,
            self.component.types()[src].instance,
            dst,
            self.component.types()[dst].instance,
            |state| {
                if let WaitableState::Stream(ty, state) = state {
                    Ok((*ty, state))
                } else {
                    Err(anyhow!("invalid stream handle"))
                }
            },
            WaitableState::Stream,
        )
    }

    /// Copy the specified error context from one component to another.
    pub(crate) fn error_context_transfer(
        &mut self,
        src_idx: u32,
        src: TypeComponentLocalErrorContextTableIndex,
        dst: TypeComponentLocalErrorContextTableIndex,
    ) -> Result<u32> {
        let (rep, _) = {
            let rep = self
                .error_context_tables()
                .get_mut(src)
                .context("error context table index present in (sub)component lookup")?
                .get_mut_by_index(src_idx)?;
            rep
        };
        let dst = self
            .error_context_tables()
            .get_mut(dst)
            .context("error context table index present in (sub)component lookup")?;

        // Update the component local for the destination
        let updated_count = if let Some((dst_idx, count)) = dst.get_mut_by_rep(rep) {
            (*count).0 += 1;
            dst_idx
        } else {
            dst.insert(rep, LocalErrorContextRefCount(1))?
        };

        // Update the global (cross-subcomponent) count for error contexts
        // as the new component has essentially created a new reference that will
        // be dropped/handled independently
        let global_ref_count = self
            .global_error_context_ref_counts()
            .get_mut(&TypeComponentGlobalErrorContextTableIndex::from_u32(rep))
            .context("global ref count present for existing (sub)component error context")?;
        global_ref_count.0 += 1;

        Ok(updated_count)
    }
}

pub(crate) struct ResourcePair {
    pub(crate) write: u32,
    pub(crate) read: u32,
}
