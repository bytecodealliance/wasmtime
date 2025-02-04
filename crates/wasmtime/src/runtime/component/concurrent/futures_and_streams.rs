use {
    super::{table::TableId, Event, GuestTask, HostTaskFuture, HostTaskResult, Promise},
    crate::{
        component::{
            func::{self, Lift, LiftContext, LowerContext, Options},
            matching::InstanceType,
            values::{ErrorContextAny, FutureAny, StreamAny},
            Lower, Val, WasmList, WasmStr,
        },
        vm::{
            component::{
                ComponentInstance, ErrorContextState, GlobalErrorContextRefCount,
                LocalErrorContextRefCount, StateTable, StreamFutureState, WaitableState,
            },
            SendSyncPtr, VMFuncRef, VMMemoryDefinition, VMStore,
        },
        AsContextMut, StoreContextMut, ValRaw,
    },
    anyhow::{anyhow, bail, Context, Result},
    futures::{
        channel::oneshot,
        future::{self, FutureExt},
    },
    std::{
        any::Any,
        boxed::Box,
        marker::PhantomData,
        mem::{self, MaybeUninit},
        ptr::NonNull,
        string::ToString,
        sync::Arc,
        vec::Vec,
    },
    wasmtime_environ::component::{
        CanonicalAbiInfo, ComponentTypes, InterfaceType, StringEncoding,
        TypeComponentGlobalErrorContextTableIndex, TypeComponentLocalErrorContextTableIndex,
        TypeFutureTableIndex, TypeStreamTableIndex,
    },
};

const BLOCKED: usize = 0xffff_ffff;
const CLOSED: usize = 0x8000_0000;

#[derive(Copy, Clone, Debug)]
pub(super) enum TableIndex {
    Stream(TypeStreamTableIndex),
    Future(TypeFutureTableIndex),
}

fn payload(ty: TableIndex, types: &Arc<ComponentTypes>) -> Option<InterfaceType> {
    match ty {
        TableIndex::Future(ty) => types[types[ty].ty].payload,
        TableIndex::Stream(ty) => types[types[ty].ty].payload,
    }
}

fn state_table(instance: &mut ComponentInstance, ty: TableIndex) -> &mut StateTable<WaitableState> {
    let runtime_instance = match ty {
        TableIndex::Stream(ty) => instance.component_types()[ty].instance,
        TableIndex::Future(ty) => instance.component_types()[ty].instance,
    };
    &mut instance.component_waitable_tables()[runtime_instance]
}

fn push_event<T>(
    mut store: StoreContextMut<T>,
    rep: u32,
    event: Event,
    param: usize,
    caller: TableId<GuestTask>,
) {
    store
        .concurrent_state()
        .futures
        .get_mut()
        .push(Box::pin(future::ready((
            rep,
            Box::new(move |_| {
                Ok(HostTaskResult {
                    event,
                    param: u32::try_from(param).unwrap(),
                    caller,
                })
            })
                as Box<dyn FnOnce(*mut dyn VMStore) -> Result<HostTaskResult> + Send + Sync>,
        ))) as HostTaskFuture);
}

fn get_mut_by_index(
    instance: &mut ComponentInstance,
    ty: TableIndex,
    index: u32,
) -> Result<(u32, &mut StreamFutureState)> {
    get_mut_by_index_from(state_table(instance, ty), ty, index)
}

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

fn waitable_state(ty: TableIndex, state: StreamFutureState) -> WaitableState {
    match ty {
        TableIndex::Stream(ty) => WaitableState::Stream(ty, state),
        TableIndex::Future(ty) => WaitableState::Future(ty, state),
    }
}

fn accept<T: func::Lower + Send + Sync + 'static, U>(
    values: Vec<T>,
    mut offset: usize,
    transmit_id: TableId<TransmitState>,
    tx: oneshot::Sender<()>,
) -> impl FnOnce(Reader) -> Result<usize> + Send + Sync + 'static {
    move |reader| {
        let count = match reader {
            Reader::Guest {
                lower:
                    RawLowerContext {
                        store,
                        options,
                        types,
                        instance,
                    },
                ty,
                address,
                count,
            } => {
                let mut store = unsafe { StoreContextMut::<U>(&mut *store.cast()) };
                let lower = &mut unsafe {
                    LowerContext::new(store.as_context_mut(), options, types, instance)
                };
                if address % usize::try_from(T::ALIGN32)? != 0 {
                    bail!("read pointer not aligned");
                }
                lower
                    .as_slice_mut()
                    .get_mut(address..)
                    .and_then(|b| b.get_mut(..T::SIZE32 * count))
                    .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?;

                let count = values.len().min(usize::try_from(count).unwrap());

                if let Some(ty) = payload(ty, types) {
                    T::store_list(lower, ty, address, &values[offset..][..count])?;
                }
                offset += count;

                if offset < values.len() {
                    let transmit = store.concurrent_state().table.get_mut(transmit_id)?;
                    assert!(matches!(&transmit.write, WriteState::Open));

                    transmit.write = WriteState::HostReady {
                        accept: Box::new(accept::<T, U>(values, offset, transmit_id, tx)),
                        close: false,
                    };
                }

                count
            }
            Reader::Host { accept } => {
                assert!(offset == 0); // todo: do we need to handle offset != 0?
                let count = values.len();
                accept(Box::new(values))?;

                count
            }
            Reader::None => 0,
        };

        Ok(count)
    }
}

fn host_write<T: func::Lower + Send + Sync + 'static, U, S: AsContextMut<Data = U>>(
    mut store: S,
    rep: u32,
    values: Vec<T>,
    mut close: bool,
) -> Result<oneshot::Receiver<()>> {
    let mut store = store.as_context_mut();
    let (tx, rx) = oneshot::channel();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let mut offset = 0;

    loop {
        let transmit = store
            .concurrent_state()
            .table
            .get_mut(transmit_id)
            .with_context(|| rep.to_string())?;
        let new_state = if let ReadState::Closed = &transmit.read {
            ReadState::Closed
        } else {
            ReadState::Open
        };

        match mem::replace(&mut transmit.read, new_state) {
            ReadState::Open => {
                assert!(matches!(&transmit.write, WriteState::Open));

                transmit.write = WriteState::HostReady {
                    accept: Box::new(accept::<T, U>(values, offset, transmit_id, tx)),
                    close,
                };
                close = false;
            }

            ReadState::GuestReady {
                ty,
                flat_abi: _,
                options,
                address,
                count,
                instance,
                handle,
                caller,
            } => unsafe {
                let types = (*instance.as_ptr()).component_types();
                let lower = &mut LowerContext::new(
                    store.as_context_mut(),
                    &options,
                    types,
                    instance.as_ptr(),
                );
                if address % usize::try_from(T::ALIGN32)? != 0 {
                    bail!("read pointer not aligned");
                }
                lower
                    .as_slice_mut()
                    .get_mut(address..)
                    .and_then(|b| b.get_mut(..T::SIZE32 * count))
                    .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?;

                let count = values.len().min(count);
                if let Some(ty) = payload(ty, types) {
                    T::store_list(lower, ty, address, &values[offset..][..count])?;
                }
                offset += count;

                log::trace!(
                    "remove read child of {}: {}",
                    caller.rep(),
                    transmit_id.rep()
                );
                store
                    .concurrent_state()
                    .table
                    .remove_child(transmit_id, caller)?;

                *get_mut_by_index(&mut *instance.as_ptr(), ty, handle)?.1 = StreamFutureState::Read;

                push_event(
                    store.as_context_mut(),
                    transmit_id.rep(),
                    match ty {
                        TableIndex::Future(_) => Event::FutureRead,
                        TableIndex::Stream(_) => Event::StreamRead,
                    },
                    count,
                    caller,
                );

                if offset < values.len() {
                    continue;
                }
            },

            ReadState::HostReady { accept } => {
                accept(Writer::Host {
                    values: Box::new(values),
                })?;
            }

            ReadState::Closed => {}
        }

        if close {
            host_close_writer(store, rep)?;
        }

        break Ok(rx);
    }
}

pub fn host_read<T: func::Lift + Sync + Send + 'static, U, S: AsContextMut<Data = U>>(
    mut store: S,
    rep: u32,
) -> Result<oneshot::Receiver<Option<Vec<T>>>> {
    let mut store = store.as_context_mut();
    let (tx, rx) = oneshot::channel();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = store
        .concurrent_state()
        .table
        .get_mut(transmit_id)
        .with_context(|| rep.to_string())?;
    let new_state = if let WriteState::Closed = &transmit.write {
        WriteState::Closed
    } else {
        WriteState::Open
    };

    match mem::replace(&mut transmit.write, new_state) {
        WriteState::Open => {
            assert!(matches!(&transmit.read, ReadState::Open));

            transmit.read = ReadState::HostReady {
                accept: Box::new(move |writer| {
                    Ok(match writer {
                        Writer::Guest {
                            lift,
                            ty,
                            address,
                            count,
                        } => {
                            _ = tx.send(
                                ty.map(|ty| {
                                    if address % usize::try_from(T::ALIGN32)? != 0 {
                                        bail!("write pointer not aligned");
                                    }
                                    lift.memory()
                                        .get(address..)
                                        .and_then(|b| b.get(..T::SIZE32 * count))
                                        .ok_or_else(|| {
                                            anyhow::anyhow!("write pointer out of bounds of memory")
                                        })?;

                                    let list = &WasmList::new(address, count, lift, ty)?;
                                    T::load_list(lift, list)
                                })
                                .transpose()?,
                            );
                            count
                        }
                        Writer::Host { values } => {
                            let values = *values
                                .downcast::<Vec<T>>()
                                .map_err(|_| anyhow!("transmit type mismatch"))?;
                            let count = values.len();
                            _ = tx.send(Some(values));
                            count
                        }
                        Writer::None => 0,
                    })
                }),
            };
        }

        WriteState::GuestReady {
            ty,
            flat_abi: _,
            options,
            address,
            count,
            instance,
            handle,
            caller,
            close,
        } => unsafe {
            let types = (*instance.as_ptr()).component_types();
            let lift = &mut LiftContext::new(store.0, &options, types, instance.as_ptr());
            _ = tx.send(
                payload(ty, types)
                    .map(|ty| {
                        let list = &WasmList::new(address, count, lift, ty)?;
                        T::load_list(lift, list)
                    })
                    .transpose()?,
            );

            log::trace!(
                "remove write child of {}: {}",
                caller.rep(),
                transmit_id.rep()
            );
            store
                .concurrent_state()
                .table
                .remove_child(transmit_id, caller)?;

            if close {
                store.concurrent_state().table.get_mut(transmit_id)?.write = WriteState::Closed;
            } else {
                *get_mut_by_index(&mut *instance.as_ptr(), ty, handle)?.1 =
                    StreamFutureState::Write;
            }

            push_event(
                store,
                transmit_id.rep(),
                match ty {
                    TableIndex::Future(_) => Event::FutureWrite,
                    TableIndex::Stream(_) => Event::StreamWrite,
                },
                count,
                caller,
            );
        },

        WriteState::HostReady { accept, close } => {
            accept(Reader::Host {
                accept: Box::new(move |any| {
                    _ = tx.send(Some(
                        *any.downcast()
                            .map_err(|_| anyhow!("transmit type mismatch"))?,
                    ));
                    Ok(())
                }),
            })?;

            if close {
                store.concurrent_state().table.get_mut(transmit_id)?.write = WriteState::Closed;
            }
        }

        WriteState::Closed => {
            host_close_reader(store, rep)?;
        }
    }

    Ok(rx)
}

fn host_cancel_write<U, S: AsContextMut<Data = U>>(mut store: S, rep: u32) -> Result<u32> {
    let mut store = store.as_context_mut();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = store.concurrent_state().table.get_mut(transmit_id)?;

    match &transmit.write {
        WriteState::GuestReady { caller, .. } => {
            let caller = *caller;
            transmit.write = WriteState::Open;
            store
                .concurrent_state()
                .table
                .remove_child(transmit_id, caller)?;
        }

        WriteState::HostReady { .. } => {
            transmit.write = WriteState::Open;
        }

        WriteState::Open | WriteState::Closed => {
            bail!("stream or future write canceled when no write is pending")
        }
    }

    log::trace!("canceled write {rep}");

    Ok(0)
}

fn host_cancel_read<U, S: AsContextMut<Data = U>>(mut store: S, rep: u32) -> Result<u32> {
    let mut store = store.as_context_mut();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = store.concurrent_state().table.get_mut(transmit_id)?;

    match &transmit.read {
        ReadState::GuestReady { caller, .. } => {
            let caller = *caller;
            transmit.read = ReadState::Open;
            store
                .concurrent_state()
                .table
                .remove_child(transmit_id, caller)?;
        }

        ReadState::HostReady { .. } => {
            transmit.read = ReadState::Open;
        }

        ReadState::Open | ReadState::Closed => {
            bail!("stream or future read canceled when no read is pending")
        }
    }

    log::trace!("canceled read {rep}");

    Ok(0)
}

fn host_close_writer<U, S: AsContextMut<Data = U>>(mut store: S, rep: u32) -> Result<()> {
    let mut store = store.as_context_mut();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = store.concurrent_state().table.get_mut(transmit_id)?;

    match &mut transmit.write {
        WriteState::GuestReady { close, .. } => {
            *close = true;
        }

        WriteState::HostReady { close, .. } => {
            *close = true;
        }

        v @ WriteState::Open => {
            *v = WriteState::Closed;
        }

        WriteState::Closed => unreachable!(),
    }

    let new_state = if let ReadState::Closed = &transmit.read {
        ReadState::Closed
    } else {
        ReadState::Open
    };

    match mem::replace(&mut transmit.read, new_state) {
        ReadState::GuestReady {
            ty,
            instance,
            handle,
            caller,
            ..
        } => unsafe {
            push_event(
                store,
                transmit_id.rep(),
                match ty {
                    TableIndex::Future(_) => Event::FutureRead,
                    TableIndex::Stream(_) => Event::StreamRead,
                },
                CLOSED,
                caller,
            );

            *get_mut_by_index(&mut *instance.as_ptr(), ty, handle)?.1 = StreamFutureState::Read;
        },

        ReadState::HostReady { accept } => {
            accept(Writer::None)?;

            host_close_reader(store, rep)?;
        }

        ReadState::Open => {}

        ReadState::Closed => {
            log::trace!("host_close_writer delete {}", transmit_id.rep());
            store.concurrent_state().table.delete(transmit_id)?;
        }
    }
    Ok(())
}

fn host_close_reader<U, S: AsContextMut<Data = U>>(mut store: S, rep: u32) -> Result<()> {
    let mut store = store.as_context_mut();
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = store.concurrent_state().table.get_mut(transmit_id)?;

    transmit.read = ReadState::Closed;

    let new_state = if let WriteState::Closed = &transmit.write {
        WriteState::Closed
    } else {
        WriteState::Open
    };

    match mem::replace(&mut transmit.write, new_state) {
        WriteState::GuestReady {
            ty,
            instance,
            handle,
            close,
            caller,
            ..
        } => unsafe {
            push_event(
                store.as_context_mut(),
                transmit_id.rep(),
                match ty {
                    TableIndex::Future(_) => Event::FutureRead,
                    TableIndex::Stream(_) => Event::StreamRead,
                },
                CLOSED,
                caller,
            );

            if close {
                store.concurrent_state().table.delete(transmit_id)?;
            } else {
                *get_mut_by_index(&mut *instance.as_ptr(), ty, handle)?.1 =
                    StreamFutureState::Write;
            }
        },

        WriteState::HostReady { accept, close } => {
            accept(Reader::None)?;

            if close {
                store.concurrent_state().table.delete(transmit_id)?;
            }
        }

        WriteState::Open => {}

        WriteState::Closed => {
            log::trace!("host_close_reader delete {}", transmit_id.rep());
            store.concurrent_state().table.delete(transmit_id)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FlatAbi {
    pub(super) size: u32,
    pub(super) align: u32,
}

/// Represents the writable end of a Component Model `future`.
pub struct FutureWriter<T> {
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> FutureWriter<T> {
    /// Write the specified value to this `future`.
    pub fn write<U, S: AsContextMut<Data = U>>(self, store: S, value: T) -> Result<Promise<()>>
    where
        T: func::Lower + Send + Sync + 'static,
    {
        Ok(Promise(Box::pin(
            host_write(store, self.rep, vec![value], true)?.map(drop),
        )))
    }

    /// Close this object without writing a value.
    ///
    /// If this object is dropped without calling either this method or `write`,
    /// any read on the readable end will remain pending forever.
    pub fn close<U, S: AsContextMut<Data = U>>(self, store: S) -> Result<()> {
        host_close_writer(store, self.rep)
    }
}

/// Represents the readable end of a Component Model `future`.
pub struct FutureReader<T> {
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> FutureReader<T> {
    pub(crate) fn new(rep: u32) -> Self {
        Self {
            rep,
            _phantom: PhantomData,
        }
    }

    /// Read the value from this `future`.
    pub fn read<U, S: AsContextMut<Data = U>>(self, store: S) -> Result<Promise<Option<T>>>
    where
        T: func::Lift + Sync + Send + 'static,
    {
        Ok(Promise(Box::pin(host_read(store, self.rep)?.map(|v| {
            v.ok()
                .and_then(|v| v.map(|v| v.into_iter().next().unwrap()))
        }))))
    }

    /// Convert this `FutureReader` into a [`Val`].
    pub fn into_val(self) -> Val {
        Val::Future(FutureAny(self.rep))
    }

    /// Attempt to convert the specified [`Val`] to a `FutureReader`.
    pub fn from_val<U, S: AsContextMut<Data = U>>(mut store: S, value: &Val) -> Result<Self> {
        let Val::Future(FutureAny(rep)) = value else {
            bail!("expected `future`; got `{}`", value.desc());
        };
        store
            .as_context_mut()
            .concurrent_state()
            .table
            .get(TableId::<TransmitState>::new(*rep))?;
        Ok(Self::new(*rep))
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Future(dst) => {
                state_table(unsafe { &mut *cx.instance }, TableIndex::Future(dst)).insert(
                    self.rep,
                    WaitableState::Future(dst, StreamFutureState::Read),
                )
            }
            _ => func::bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Future(src) => {
                let state_table =
                    state_table(unsafe { &mut *cx.instance }, TableIndex::Future(src));
                let (rep, state) =
                    get_mut_by_index_from(state_table, TableIndex::Future(src), index)?;

                match state {
                    StreamFutureState::Local => {
                        *state = StreamFutureState::Write;
                    }
                    StreamFutureState::Read => {
                        state_table.remove_by_index(index)?;
                    }
                    StreamFutureState::Write => bail!("cannot transfer write end of future"),
                    StreamFutureState::Busy => bail!("cannot transfer busy future"),
                }

                Ok(Self {
                    rep,
                    _phantom: PhantomData,
                })
            }
            _ => func::bad_type_info(),
        }
    }

    /// Close this object without reading the value.
    ///
    /// If this object is dropped without calling either this method or `read`,
    /// any write on the writable end will remain pending forever.
    pub fn close<U, S: AsContextMut<Data = U>>(self, store: S) -> Result<()> {
        host_close_reader(store, self.rep)
    }
}

unsafe impl<T> func::ComponentType for FutureReader<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Future(_) => Ok(()),
            other => bail!("expected `future`, found `{}`", func::desc(other)),
        }
    }
}

unsafe impl<T> func::Lower for FutureReader<T> {
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

unsafe impl<T> func::Lift for FutureReader<T> {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Self::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Self::lift_from_index(cx, ty, index)
    }
}

/// Create a new Component Model `future` as pair of writable and readable ends,
/// the latter of which may be passed to guest code.
pub fn future<T, U, S: AsContextMut<Data = U>>(
    mut store: S,
) -> Result<(FutureWriter<T>, FutureReader<T>)> {
    let mut store = store.as_context_mut();
    let transmit = store.concurrent_state().table.push(TransmitState {
        read: ReadState::Open,
        write: WriteState::Open,
    })?;

    Ok((
        FutureWriter {
            rep: transmit.rep(),
            _phantom: PhantomData,
        },
        FutureReader {
            rep: transmit.rep(),
            _phantom: PhantomData,
        },
    ))
}

/// Represents the writable end of a Component Model `stream`.
pub struct StreamWriter<T> {
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> StreamWriter<T> {
    /// Write the specified values to the `stream`.
    pub fn write<U, S: AsContextMut<Data = U>>(
        self,
        store: S,
        values: Vec<T>,
    ) -> Result<Promise<StreamWriter<T>>>
    where
        T: func::Lower + Send + Sync + 'static,
    {
        Ok(Promise(Box::pin(
            host_write(store, self.rep, values, false)?.map(move |_| self),
        )))
    }

    /// Close this object without writing any more values.
    ///
    /// If this object is dropped without calling this method, any read on the
    /// readable end will remain pending forever.
    pub fn close<U, S: AsContextMut<Data = U>>(self, store: S) -> Result<()> {
        host_close_writer(store, self.rep)
    }
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T> {
    rep: u32,
    _phantom: PhantomData<T>,
}

impl<T> StreamReader<T> {
    pub(crate) fn new(rep: u32) -> Self {
        Self {
            rep,
            _phantom: PhantomData,
        }
    }

    /// Read the next values (if any) from this `stream`.
    pub fn read<U, S: AsContextMut<Data = U>>(
        self,
        store: S,
    ) -> Result<Promise<Option<(StreamReader<T>, Vec<T>)>>>
    where
        T: func::Lift + Sync + Send + 'static,
    {
        Ok(Promise(Box::pin(
            host_read(store, self.rep)?.map(move |v| v.ok().and_then(|v| v.map(|v| (self, v)))),
        )))
    }

    /// Convert this `StreamReader` into a [`Val`].
    pub fn into_val(self) -> Val {
        Val::Stream(StreamAny(self.rep))
    }

    /// Attempt to convert the specified [`Val`] to a `StreamReader`.
    pub fn from_val<U, S: AsContextMut<Data = U>>(mut store: S, value: &Val) -> Result<Self> {
        let Val::Stream(StreamAny(rep)) = value else {
            bail!("expected `stream`; got `{}`", value.desc());
        };
        store
            .as_context_mut()
            .concurrent_state()
            .table
            .get(TableId::<TransmitState>::new(*rep))?;
        Ok(Self::new(*rep))
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Stream(dst) => {
                state_table(unsafe { &mut *cx.instance }, TableIndex::Stream(dst)).insert(
                    self.rep,
                    WaitableState::Stream(dst, StreamFutureState::Read),
                )
            }
            _ => func::bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Stream(src) => {
                let state_table =
                    state_table(unsafe { &mut *cx.instance }, TableIndex::Stream(src));
                let (rep, state) =
                    get_mut_by_index_from(state_table, TableIndex::Stream(src), index)?;

                match state {
                    StreamFutureState::Local => {
                        *state = StreamFutureState::Write;
                    }
                    StreamFutureState::Read => {
                        state_table.remove_by_index(index)?;
                    }
                    StreamFutureState::Write => bail!("cannot transfer write end of stream"),
                    StreamFutureState::Busy => bail!("cannot transfer busy stream"),
                }

                Ok(Self {
                    rep,
                    _phantom: PhantomData,
                })
            }
            _ => func::bad_type_info(),
        }
    }

    /// Close this object without reading any more values.
    ///
    /// If the object is dropped without either calling this method or reading
    /// until the end of the stream, any write on the writable end will remain
    /// pending forever.
    pub fn close<U, S: AsContextMut<Data = U>>(self, store: S) -> Result<()> {
        host_close_reader(store, self.rep)
    }
}

unsafe impl<T> func::ComponentType for StreamReader<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as func::ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Stream(_) => Ok(()),
            other => bail!("expected `stream`, found `{}`", func::desc(other)),
        }
    }
}

unsafe impl<T> func::Lower for StreamReader<T> {
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

unsafe impl<T> func::Lift for StreamReader<T> {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Self::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Self::lift_from_index(cx, ty, index)
    }
}

/// Create a new Component Model `stream` as pair of writable and readable ends,
/// the latter of which may be passed to guest code.
pub fn stream<T, U, S: AsContextMut<Data = U>>(
    mut store: S,
) -> Result<(StreamWriter<T>, StreamReader<T>)> {
    let mut store = store.as_context_mut();
    let transmit = store.concurrent_state().table.push(TransmitState {
        read: ReadState::Open,
        write: WriteState::Open,
    })?;

    Ok((
        StreamWriter {
            rep: transmit.rep(),
            _phantom: PhantomData,
        },
        StreamReader {
            rep: transmit.rep(),
            _phantom: PhantomData,
        },
    ))
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
    pub fn from_val<U, S: AsContextMut<Data = U>>(_: S, value: &Val) -> Result<Self> {
        let Val::ErrorContext(ErrorContextAny(rep)) = value else {
            bail!("expected `error-context`; got `{}`", value.desc());
        };
        Ok(Self::new(*rep))
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::ErrorContext(dst) => {
                let tbl = unsafe {
                    &mut (*cx.instance)
                        .component_error_context_tables()
                        .get_mut(dst)
                        .expect("error context table index present in (sub)component table during lower")
                };

                if let Some((dst_idx, dst_state)) = tbl.get_mut_by_rep(self.rep) {
                    dst_state.0 += 1;
                    Ok(dst_idx)
                } else {
                    tbl.insert(self.rep, LocalErrorContextRefCount(1))
                }
            }
            _ => func::bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::ErrorContext(src) => {
                let (rep, _) = unsafe {
                    (*cx.instance)
                        .component_error_context_tables()
                        .get_mut(src)
                        .expect(
                            "error context table index present in (sub)component table during lift",
                        )
                        .get_mut_by_index(index)?
                };

                Ok(Self { rep })
            }
            _ => func::bad_type_info(),
        }
    }
}

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

unsafe impl func::Lower for ErrorContext {
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

unsafe impl func::Lift for ErrorContext {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Self::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Self::lift_from_index(cx, ty, index)
    }
}

pub(super) struct TransmitState {
    write: WriteState,
    read: ReadState,
}

enum WriteState {
    Open,
    GuestReady {
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        instance: SendSyncPtr<ComponentInstance>,
        handle: u32,
        caller: TableId<GuestTask>,
        close: bool,
    },
    HostReady {
        accept: Box<dyn FnOnce(Reader) -> Result<usize> + Send + Sync>,
        close: bool,
    },
    Closed,
}

enum ReadState {
    Open,
    GuestReady {
        ty: TableIndex,
        flat_abi: Option<FlatAbi>,
        options: Options,
        address: usize,
        count: usize,
        instance: SendSyncPtr<ComponentInstance>,
        handle: u32,
        caller: TableId<GuestTask>,
    },
    HostReady {
        accept: Box<dyn FnOnce(Writer) -> Result<usize> + Send + Sync>,
    },
    Closed,
}

enum Writer<'a> {
    Guest {
        lift: &'a mut LiftContext<'a>,
        ty: Option<InterfaceType>,
        address: usize,
        count: usize,
    },
    Host {
        values: Box<dyn Any>,
    },
    None,
}

struct RawLowerContext<'a> {
    store: *mut dyn VMStore,
    options: &'a Options,
    types: &'a Arc<ComponentTypes>,
    instance: *mut ComponentInstance,
}

enum Reader<'a> {
    Guest {
        lower: RawLowerContext<'a>,
        ty: TableIndex,
        address: usize,
        count: usize,
    },
    Host {
        accept: Box<dyn FnOnce(Box<dyn Any>) -> Result<()>>,
    },
    None,
}

pub(super) fn guest_new<T>(
    mut cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TableIndex,
) -> Result<u32> {
    let transmit = cx.concurrent_state().table.push(TransmitState {
        read: ReadState::Open,
        write: WriteState::Open,
    })?;
    state_table(instance, ty).insert(transmit.rep(), waitable_state(ty, StreamFutureState::Local))
}

fn copy<T>(
    mut cx: StoreContextMut<'_, T>,
    types: &Arc<ComponentTypes>,
    instance: *mut ComponentInstance,
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

                    let lift =
                        &mut unsafe { LiftContext::new(cx.0, write_options, types, instance) };

                    let bytes = lift
                        .memory()
                        .get(write_address..)
                        .and_then(|b| b.get(..usize::try_from(abi.size32).unwrap()))
                        .ok_or_else(|| anyhow::anyhow!("write pointer out of bounds of memory"))?;

                    Val::load(lift, ty, bytes)
                })
                .transpose()?;

            if let Some(val) = val {
                let mut lower = unsafe {
                    LowerContext::new(cx.as_context_mut(), read_options, types, instance)
                };
                let ty = types[types[read_ty].ty].payload.unwrap();
                let ptr = func::validate_inbounds_dynamic(
                    types.canonical_abi(&ty),
                    lower.as_slice_mut(),
                    &ValRaw::u32(read_address.try_into().unwrap()),
                )?;
                val.store(&mut lower, ty, ptr)?;
            }
        }
        (TableIndex::Stream(write_ty), TableIndex::Stream(read_ty)) => {
            let lift = &mut unsafe { LiftContext::new(cx.0, write_options, types, instance) };
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

                    {
                        let src = write_options
                            .memory(cx.0)
                            .get(write_address..)
                            .and_then(|b| b.get(..length_in_bytes))
                            .ok_or_else(|| {
                                anyhow::anyhow!("write pointer out of bounds of memory")
                            })?
                            .as_ptr();
                        let dst = read_options
                            .memory_mut(cx.0)
                            .get_mut(read_address..)
                            .and_then(|b| b.get_mut(..length_in_bytes))
                            .ok_or_else(|| anyhow::anyhow!("read pointer out of bounds of memory"))?
                            .as_mut_ptr();
                        unsafe { src.copy_to(dst, length_in_bytes) };
                    }
                }
            } else {
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

                log::trace!("copy values {values:?} for {rep}");

                let lower = &mut unsafe {
                    LowerContext::new(cx.as_context_mut(), read_options, types, instance)
                };
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

pub(super) fn guest_write<T>(
    mut cx: StoreContextMut<T>,
    instance: *mut ComponentInstance,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TableIndex,
    flat_abi: Option<FlatAbi>,
    handle: u32,
    address: u32,
    count: u32,
) -> Result<u32> {
    let address = usize::try_from(address).unwrap();
    let count = usize::try_from(count).unwrap();
    let options = unsafe {
        Options::new(
            cx.0.id(),
            NonNull::new(memory),
            NonNull::new(realloc),
            StringEncoding::from_u8(string_encoding).unwrap(),
            true,
            None,
        )
    };
    let types = unsafe { (*instance).component_types() };
    let (rep, state) = unsafe { get_mut_by_index(&mut *instance, ty, handle)? };
    let StreamFutureState::Write = *state else {
        bail!("invalid handle");
    };
    *state = StreamFutureState::Busy;
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = cx.concurrent_state().table.get_mut(transmit_id)?;
    let new_state = if let ReadState::Closed = &transmit.read {
        ReadState::Closed
    } else {
        ReadState::Open
    };

    let result = match mem::replace(&mut transmit.read, new_state) {
        ReadState::GuestReady {
            ty: read_ty,
            flat_abi: read_flat_abi,
            options: read_options,
            address: read_address,
            count: read_count,
            instance: _,
            handle: read_handle,
            caller: read_caller,
        } => {
            assert_eq!(flat_abi, read_flat_abi);

            let count = count.min(read_count);

            copy(
                cx.as_context_mut(),
                types,
                instance,
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

            log::trace!(
                "remove read child of {}: {}",
                read_caller.rep(),
                transmit_id.rep()
            );
            cx.concurrent_state()
                .table
                .remove_child(transmit_id, read_caller)?;

            unsafe {
                *get_mut_by_index(&mut *instance, read_ty, read_handle)?.1 =
                    StreamFutureState::Read;
            }

            push_event(
                cx,
                transmit_id.rep(),
                match read_ty {
                    TableIndex::Future(_) => Event::FutureRead,
                    TableIndex::Stream(_) => Event::StreamRead,
                },
                count,
                read_caller,
            );

            count
        }

        ReadState::HostReady { accept } => {
            let lift = &mut unsafe { LiftContext::new(cx.0, &options, types, instance) };
            accept(Writer::Guest {
                lift,
                ty: payload(ty, types),
                address,
                count,
            })?
        }

        ReadState::Open => {
            assert!(matches!(&transmit.write, WriteState::Open));

            let caller = cx.concurrent_state().guest_task.unwrap();
            log::trace!(
                "add write {} child of {}: {}",
                match ty {
                    TableIndex::Future(_) => "future",
                    TableIndex::Stream(_) => "stream",
                },
                caller.rep(),
                transmit_id.rep()
            );
            cx.concurrent_state().table.add_child(transmit_id, caller)?;

            let transmit = cx.concurrent_state().table.get_mut(transmit_id)?;
            transmit.write = WriteState::GuestReady {
                ty,
                flat_abi,
                options,
                address: usize::try_from(address).unwrap(),
                count: usize::try_from(count).unwrap(),
                instance: SendSyncPtr::new(NonNull::new(instance).unwrap()),
                handle,
                caller,
                close: false,
            };

            BLOCKED
        }

        ReadState::Closed => CLOSED,
    };

    if result != BLOCKED {
        unsafe {
            *get_mut_by_index(&mut *instance, ty, handle)?.1 = StreamFutureState::Write;
        }
    }

    Ok(u32::try_from(result).unwrap())
}

pub(super) fn guest_read<T>(
    mut cx: StoreContextMut<T>,
    instance: *mut ComponentInstance,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TableIndex,
    flat_abi: Option<FlatAbi>,
    handle: u32,
    address: u32,
    count: u32,
) -> Result<u32> {
    let address = usize::try_from(address).unwrap();
    let count = usize::try_from(count).unwrap();
    let options = unsafe {
        Options::new(
            cx.0.id(),
            NonNull::new(memory),
            NonNull::new(realloc),
            StringEncoding::from_u8(string_encoding).unwrap(),
            true,
            None,
        )
    };
    let types = unsafe { (*instance).component_types() };
    let (rep, state) = unsafe { get_mut_by_index(&mut *instance, ty, handle)? };
    let StreamFutureState::Read = *state else {
        bail!("invalid handle");
    };
    *state = StreamFutureState::Busy;
    let transmit_id = TableId::<TransmitState>::new(rep);
    let transmit = cx.concurrent_state().table.get_mut(transmit_id)?;
    let new_state = if let WriteState::Closed = &transmit.write {
        WriteState::Closed
    } else {
        WriteState::Open
    };

    let result = match mem::replace(&mut transmit.write, new_state) {
        WriteState::GuestReady {
            ty: write_ty,
            flat_abi: write_flat_abi,
            options: write_options,
            address: write_address,
            count: write_count,
            instance: _,
            handle: write_handle,
            caller: write_caller,
            close,
        } => {
            assert_eq!(flat_abi, write_flat_abi);

            let count = count.min(write_count);

            copy(
                cx.as_context_mut(),
                types,
                instance,
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

            log::trace!(
                "remove write child of {}: {}",
                write_caller.rep(),
                transmit_id.rep()
            );
            cx.concurrent_state()
                .table
                .remove_child(transmit_id, write_caller)?;

            if close {
                cx.concurrent_state().table.get_mut(transmit_id)?.write = WriteState::Closed;
            } else {
                unsafe {
                    *get_mut_by_index(&mut *instance, write_ty, write_handle)?.1 =
                        StreamFutureState::Write;
                }
            }

            push_event(
                cx,
                transmit_id.rep(),
                match write_ty {
                    TableIndex::Future(_) => Event::FutureWrite,
                    TableIndex::Stream(_) => Event::StreamWrite,
                },
                count,
                write_caller,
            );

            count
        }

        WriteState::HostReady { accept, close } => {
            let count = accept(Reader::Guest {
                lower: RawLowerContext {
                    store: cx.0.traitobj().as_ptr(),
                    options: &options,
                    types,
                    instance,
                },
                ty,
                address: usize::try_from(address).unwrap(),
                count,
            })?;

            if close {
                cx.concurrent_state().table.get_mut(transmit_id)?.write = WriteState::Closed;
            }

            count
        }

        WriteState::Open => {
            assert!(matches!(&transmit.read, ReadState::Open));

            let caller = cx.concurrent_state().guest_task.unwrap();
            log::trace!(
                "add read {} child of {}: {}",
                match ty {
                    TableIndex::Future(_) => "future",
                    TableIndex::Stream(_) => "stream",
                },
                caller.rep(),
                transmit_id.rep()
            );
            cx.concurrent_state().table.add_child(transmit_id, caller)?;

            let transmit = cx.concurrent_state().table.get_mut(transmit_id)?;
            transmit.read = ReadState::GuestReady {
                ty,
                flat_abi,
                options,
                address: usize::try_from(address).unwrap(),
                count: usize::try_from(count).unwrap(),
                instance: SendSyncPtr::new(NonNull::new(instance).unwrap()),
                handle,
                caller,
            };

            BLOCKED
        }

        WriteState::Closed => CLOSED,
    };

    if result != BLOCKED {
        unsafe {
            *get_mut_by_index(&mut *instance, ty, handle)?.1 = StreamFutureState::Read;
        }
    }

    Ok(u32::try_from(result).unwrap())
}

pub(super) fn guest_cancel_write<T>(
    cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TableIndex,
    writer: u32,
    _async_: bool,
) -> Result<u32> {
    let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
        state_table(instance, ty).get_mut_by_index(writer)?
    else {
        bail!("invalid stream or future handle");
    };
    match state {
        StreamFutureState::Local | StreamFutureState::Write => {
            bail!("stream or future write canceled when no write is pending")
        }
        StreamFutureState::Read => {
            bail!("passed read end to `{{stream|future}}.cancel-write`")
        }
        StreamFutureState::Busy => {
            *state = StreamFutureState::Write;
        }
    }
    host_cancel_write(cx, rep)
}

pub(super) fn guest_cancel_read<T>(
    cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TableIndex,
    reader: u32,
    _async_: bool,
) -> Result<u32> {
    let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
        state_table(instance, ty).get_mut_by_index(reader)?
    else {
        bail!("invalid stream or future handle");
    };
    match state {
        StreamFutureState::Local | StreamFutureState::Read => {
            bail!("stream or future read canceled when no read is pending")
        }
        StreamFutureState::Write => {
            bail!("passed write end to `{{stream|future}}.cancel-read`")
        }
        StreamFutureState::Busy => {
            *state = StreamFutureState::Read;
        }
    }
    host_cancel_read(cx, rep)
}

pub(super) fn guest_close_writable<T>(
    cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TableIndex,
    writer: u32,
    error: u32,
) -> Result<()> {
    if error != 0 {
        bail!("todo: closing writable streams and futures with errors not yet implemented");
    }

    let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
        state_table(instance, ty).remove_by_index(writer)?
    else {
        bail!("invalid stream or future handle");
    };
    match state {
        StreamFutureState::Local | StreamFutureState::Write => {}
        StreamFutureState::Read => {
            bail!("passed read end to `{{stream|future}}.close-writable`")
        }
        StreamFutureState::Busy => bail!("cannot drop busy stream or future"),
    }
    host_close_writer(cx, rep)
}

pub(super) fn guest_close_readable<T>(
    cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TableIndex,
    reader: u32,
) -> Result<()> {
    let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
        state_table(instance, ty).remove_by_index(reader)?
    else {
        bail!("invalid stream or future handle");
    };
    match state {
        StreamFutureState::Local | StreamFutureState::Read => {}
        StreamFutureState::Write => {
            bail!("passed write end to `{{stream|future}}.close-readable`")
        }
        StreamFutureState::Busy => bail!("cannot drop busy stream or future"),
    }
    host_close_reader(cx, rep)
}

/// Create a new error context for the given component
pub(super) fn error_context_new<T>(
    mut cx: StoreContextMut<T>,
    instance: *mut ComponentInstance,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeComponentLocalErrorContextTableIndex,
    debug_msg_address: u32,
    debug_msg_len: u32,
) -> Result<u32> {
    //  Read string from guest memory
    let options = unsafe {
        Options::new(
            cx.0.id(),
            NonNull::new(memory),
            NonNull::new(realloc),
            StringEncoding::from_u8(string_encoding).ok_or_else(|| {
                anyhow::anyhow!("failed to convert u8 string encoding [{string_encoding}]")
            })?,
            false,
            None,
        )
    };
    let lift_ctx =
        &mut unsafe { LiftContext::new(cx.0, &options, (*instance).component_types(), instance) };
    let s = {
        let address = usize::try_from(debug_msg_address)?;
        let len = usize::try_from(debug_msg_len)?;
        WasmStr::load(
            lift_ctx,
            InterfaceType::String,
            &lift_ctx
                .memory()
                .get(address..)
                .and_then(|b| b.get(..len))
                .map(|_| [debug_msg_address.to_le_bytes(), debug_msg_len.to_le_bytes()].concat())
                .ok_or_else(|| anyhow::anyhow!("invalid debug message pointer: out of bounds"))?,
        )?
    };

    // Create a new ErrorContext that is tracked along with other concurrent state
    let err_ctx = ErrorContextState {
        debug_msg: s.to_str(&cx)?.to_string(),
    };
    let table_id = cx.concurrent_state().table.push(err_ctx)?;
    let global_ref_count_idx = TypeComponentGlobalErrorContextTableIndex::from_u32(table_id.rep());

    // Add to the global error context ref counts
    unsafe {
        let _ = (*instance)
            .component_global_error_context_ref_counts()
            .insert(global_ref_count_idx, GlobalErrorContextRefCount(1));
    }

    // Error context are tracked both locally (to a single component instance) and globally
    // the counts for both must stay in sync.
    //
    // Here we reflect the newly created global concurrent error context state into the
    // component instance's locally tracked count, along with the appropriate key into the global
    // ref tracking data structures to enable later lookup
    let local_tbl = unsafe {
        (*instance)
            .component_error_context_tables()
            .get_mut_or_insert_with(ty, || StateTable::default())
    };
    assert!(
        !local_tbl.has_handle(table_id.rep()),
        "newly created error context state already tracked by component"
    );
    let local_idx = local_tbl.insert(table_id.rep(), LocalErrorContextRefCount(1))?;

    Ok(local_idx)
}

pub(super) fn error_context_debug_message<T>(
    mut cx: StoreContextMut<T>,
    instance: *mut ComponentInstance,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeComponentLocalErrorContextTableIndex,
    err_ctx_handle: u32,
    debug_msg_address: u32,
) -> Result<()> {
    let store_id = cx.0.id();

    // Retrieve the error context and internal debug message
    let (state_table_id_rep, _) = unsafe {
        (*instance)
            .component_error_context_tables()
            .get_mut(ty)
            .context("error context table index present in (sub)component lookup during debug_msg")?
            .get_mut_by_index(err_ctx_handle)?
    };

    // Get the state associated with the error context
    let ErrorContextState { debug_msg } =
        cx.concurrent_state()
            .table
            .get_mut(TableId::<ErrorContextState>::new(state_table_id_rep))?;
    let debug_msg = debug_msg.clone();

    // Lower the string into the component's memory
    let options = unsafe {
        Options::new(
            store_id,
            NonNull::new(memory),
            NonNull::new(realloc),
            StringEncoding::from_u8(string_encoding).ok_or_else(|| {
                anyhow::anyhow!("failed to convert u8 string encoding [{string_encoding}]")
            })?,
            false,
            None,
        )
    };
    let lower_cx =
        &mut unsafe { LowerContext::new(cx, &options, (*instance).component_types(), instance) };
    let debug_msg_address = usize::try_from(debug_msg_address)?;
    let offset = lower_cx
        .as_slice_mut()
        .get(debug_msg_address..)
        .and_then(|b| b.get(..debug_msg.bytes().len()))
        .map(|_| debug_msg_address)
        .ok_or_else(|| anyhow::anyhow!("invalid debug message pointer: out of bounds"))?;
    debug_msg
        .as_str()
        .store(lower_cx, InterfaceType::String, offset)?;

    Ok(())
}

pub(super) fn error_context_drop<T>(
    mut cx: StoreContextMut<T>,
    instance: &mut ComponentInstance,
    ty: TypeComponentLocalErrorContextTableIndex,
    error_context: u32,
) -> Result<()> {
    let local_state_table = instance
        .component_error_context_tables()
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

    let GlobalErrorContextRefCount(global_ref_count) = instance
        .component_global_error_context_ref_counts()
        .get_mut(&global_ref_count_idx)
        .expect("retrieve concurrent state for error context during drop");

    // Reduce the component-global ref count, removing tracking if necessary
    assert!(*global_ref_count >= 1);
    *global_ref_count -= 1;
    if *global_ref_count == 0 {
        assert!(local_ref_removed);

        instance
            .component_global_error_context_ref_counts()
            .remove(&global_ref_count_idx);

        cx.concurrent_state()
            .table
            .delete(TableId::<ErrorContextState>::new(rep))
            .context("deleting component-global error context data")?;
    }

    Ok(())
}
