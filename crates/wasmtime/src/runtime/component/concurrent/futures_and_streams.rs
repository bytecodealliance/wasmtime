use {
    super::{
        call_host_and_handle_result, events, table::TableId, GuestTask, HostTaskFuture,
        HostTaskResult, Promise,
    },
    crate::{
        component::{
            func::{self, LiftContext, LowerContext, Options},
            matching::InstanceType,
            values::{ErrorContextAny, FutureAny, StreamAny},
            Val, WasmList,
        },
        vm::{
            component::{
                ComponentInstance, StateTable, StreamFutureState, VMComponentContext, WaitableState,
            },
            SendSyncPtr, VMFuncRef, VMMemoryDefinition, VMOpaqueContext, VMStore,
        },
        AsContextMut, StoreContextMut,
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
        TypeErrorContextTableIndex, TypeFutureTableIndex, TypeStreamTableIndex,
    },
};

// TODO: add `validate_inbounds` calls where appropriate

const BLOCKED: usize = 0xffff_ffff;
const CLOSED: usize = 0x8000_0000;

#[derive(Copy, Clone, Debug)]
enum TableIndex {
    Stream(TypeStreamTableIndex),
    Future(TypeFutureTableIndex),
}

fn payload(ty: TableIndex, types: &Arc<ComponentTypes>) -> Option<InterfaceType> {
    match ty {
        TableIndex::Future(ty) => types[types[ty].ty].payload,
        TableIndex::Stream(ty) => Some(types[types[ty].ty].payload),
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
    event: u32,
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
                        TableIndex::Future(_) => events::EVENT_FUTURE_READ,
                        TableIndex::Stream(_) => events::EVENT_STREAM_READ,
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
                    TableIndex::Future(_) => events::EVENT_FUTURE_WRITE,
                    TableIndex::Stream(_) => events::EVENT_STREAM_WRITE,
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
                    TableIndex::Future(_) => events::EVENT_FUTURE_READ,
                    TableIndex::Stream(_) => events::EVENT_STREAM_READ,
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
                    TableIndex::Future(_) => events::EVENT_FUTURE_WRITE,
                    TableIndex::Stream(_) => events::EVENT_STREAM_WRITE,
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
struct FlatAbi {
    size: u32,
    align: u32,
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
                let dst = unsafe { &mut (*cx.instance).component_error_context_tables()[dst] };

                if let Some((dst_idx, dst_state)) = dst.get_mut_by_rep(self.rep) {
                    *dst_state += 1;
                    Ok(dst_idx)
                } else {
                    dst.insert(self.rep, 1)
                }
            }
            _ => func::bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::ErrorContext(src) => {
                let (rep, _) = unsafe {
                    (*cx.instance).component_error_context_tables()[src].get_mut_by_index(index)?
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

fn guest_new<T>(vmctx: *mut VMOpaqueContext, ty: TableIndex) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let mut cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let transmit = cx.concurrent_state().table.push(TransmitState {
                read: ReadState::Open,
                write: WriteState::Open,
            })?;
            state_table(&mut *instance, ty)
                .insert(transmit.rep(), waitable_state(ty, StreamFutureState::Local))
        })
    }
}

unsafe fn copy<T>(
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
                    let lift = &mut LiftContext::new(cx.0, write_options, types, instance);
                    Val::load(
                        lift,
                        ty,
                        &lift.memory()[usize::try_from(write_address).unwrap()..]
                            [..usize::try_from(types.canonical_abi(&ty).size32).unwrap()],
                    )
                })
                .transpose()?;

            let mut lower = LowerContext::new(cx.as_context_mut(), read_options, types, instance);
            if let Some(val) = val {
                val.store(
                    &mut lower,
                    types[types[read_ty].ty].payload.unwrap(),
                    usize::try_from(read_address).unwrap(),
                )?;
            }
        }
        (TableIndex::Stream(write_ty), TableIndex::Stream(read_ty)) => {
            let lift = &mut LiftContext::new(cx.0, write_options, types, instance);
            if let Some(flat_abi) = flat_abi {
                // Fast path memcpy for "flat" (i.e. no pointers or handles) payloads:
                let length_in_bytes = usize::try_from(flat_abi.size).unwrap() * count;

                {
                    let src =
                        write_options.memory(cx.0)[write_address..][..length_in_bytes].as_ptr();
                    let dst = read_options.memory_mut(cx.0)[read_address..][..length_in_bytes]
                        .as_mut_ptr();
                    src.copy_to(dst, length_in_bytes);
                }
            } else {
                let ty = types[types[write_ty].ty].payload;
                let abi = lift.types.canonical_abi(&ty);
                let size = usize::try_from(abi.size32).unwrap();
                let values = (0..count)
                    .map(|index| {
                        Val::load(
                            lift,
                            ty,
                            &lift.memory()[write_address + (index * size)..][..size],
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;

                log::trace!("copy values {values:?} for {rep}");

                let lower =
                    &mut LowerContext::new(cx.as_context_mut(), read_options, types, instance);
                let mut ptr = read_address;
                let ty = types[types[read_ty].ty].payload;
                let abi = lower.types.canonical_abi(&ty);
                let size = usize::try_from(abi.size32).unwrap();
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

fn guest_write<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TableIndex,
    flat_abi: Option<FlatAbi>,
    handle: u32,
    address: u32,
    count: u32,
) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let address = usize::try_from(address).unwrap();
            let count = usize::try_from(count).unwrap();
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let mut cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let options = Options::new(
                cx.0.id(),
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).unwrap(),
                true,
                None,
            );
            let types = (*instance).component_types();
            let (rep, state) = get_mut_by_index(&mut *instance, ty, handle)?;
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

                    *get_mut_by_index(&mut *instance, read_ty, read_handle)?.1 =
                        StreamFutureState::Read;

                    push_event(
                        cx,
                        transmit_id.rep(),
                        match read_ty {
                            TableIndex::Future(_) => events::EVENT_FUTURE_READ,
                            TableIndex::Stream(_) => events::EVENT_STREAM_READ,
                        },
                        count,
                        read_caller,
                    );

                    count
                }

                ReadState::HostReady { accept } => {
                    let lift = &mut LiftContext::new(cx.0, &options, types, instance);
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
                *get_mut_by_index(&mut *instance, ty, handle)?.1 = StreamFutureState::Write;
            }

            Ok(u32::try_from(result).unwrap())
        })
    }
}

fn guest_read<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TableIndex,
    flat_abi: Option<FlatAbi>,
    handle: u32,
    address: u32,
    count: u32,
) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let address = usize::try_from(address).unwrap();
            let count = usize::try_from(count).unwrap();
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let mut cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let options = Options::new(
                cx.0.id(),
                NonNull::new(memory),
                NonNull::new(realloc),
                StringEncoding::from_u8(string_encoding).unwrap(),
                true,
                None,
            );
            let types = (*instance).component_types();
            let (rep, state) = get_mut_by_index(&mut *instance, ty, handle)?;
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
                        cx.concurrent_state().table.get_mut(transmit_id)?.write =
                            WriteState::Closed;
                    } else {
                        *get_mut_by_index(&mut *instance, write_ty, write_handle)?.1 =
                            StreamFutureState::Write;
                    }

                    push_event(
                        cx,
                        transmit_id.rep(),
                        match write_ty {
                            TableIndex::Future(_) => events::EVENT_FUTURE_WRITE,
                            TableIndex::Stream(_) => events::EVENT_STREAM_WRITE,
                        },
                        count,
                        write_caller,
                    );

                    count
                }

                WriteState::HostReady { accept, close } => {
                    let count = accept(Reader::Guest {
                        lower: RawLowerContext {
                            store: cx.0.traitobj(),
                            options: &options,
                            types,
                            instance,
                        },
                        ty,
                        address: usize::try_from(address).unwrap(),
                        count,
                    })?;

                    if close {
                        cx.concurrent_state().table.get_mut(transmit_id)?.write =
                            WriteState::Closed;
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
                *get_mut_by_index(&mut *instance, ty, handle)?.1 = StreamFutureState::Read;
            }

            Ok(u32::try_from(result).unwrap())
        })
    }
}

fn guest_cancel_write<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TableIndex,
    writer: u32,
    _async_: bool,
) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
                state_table(&mut *instance, ty).get_mut_by_index(writer)?
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
        })
    }
}

fn guest_cancel_read<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TableIndex,
    reader: u32,
    _async_: bool,
) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
                state_table(&mut *instance, ty).get_mut_by_index(reader)?
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
        })
    }
}

fn guest_close_writable<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TableIndex,
    writer: u32,
    error: u32,
) -> bool {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            if error != 0 {
                bail!("todo: closing writable streams and futures with errors not yet implemented");
            }

            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
                state_table(&mut *instance, ty).remove_by_index(writer)?
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
        })
    }
}

fn guest_close_readable<T>(vmctx: *mut VMOpaqueContext, ty: TableIndex, reader: u32) -> bool {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let cx = StoreContextMut::<T>(&mut *(*instance).store().cast());
            let (rep, WaitableState::Stream(_, state) | WaitableState::Future(_, state)) =
                state_table(&mut *instance, ty).remove_by_index(reader)?
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
        })
    }
}

pub(crate) extern "C" fn future_new<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
) -> u64 {
    guest_new::<T>(vmctx, TableIndex::Future(ty))
}

pub(crate) extern "C" fn future_write<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeFutureTableIndex,
    future: u32,
    address: u32,
) -> u64 {
    guest_write::<T>(
        vmctx,
        memory,
        realloc,
        string_encoding,
        TableIndex::Future(ty),
        None,
        future,
        address,
        1,
    )
}

pub(crate) extern "C" fn future_read<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeFutureTableIndex,
    future: u32,
    address: u32,
) -> u64 {
    guest_read::<T>(
        vmctx,
        memory,
        realloc,
        string_encoding,
        TableIndex::Future(ty),
        None,
        future,
        address,
        1,
    )
}

pub(crate) extern "C" fn future_cancel_write<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    async_: bool,
    writer: u32,
) -> u64 {
    guest_cancel_write::<T>(vmctx, TableIndex::Future(ty), writer, async_)
}

pub(crate) extern "C" fn future_cancel_read<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    async_: bool,
    reader: u32,
) -> u64 {
    guest_cancel_read::<T>(vmctx, TableIndex::Future(ty), reader, async_)
}

pub(crate) extern "C" fn future_close_writable<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    writer: u32,
    error: u32,
) -> bool {
    guest_close_writable::<T>(vmctx, TableIndex::Future(ty), writer, error)
}

pub(crate) extern "C" fn future_close_readable<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeFutureTableIndex,
    reader: u32,
) -> bool {
    guest_close_readable::<T>(vmctx, TableIndex::Future(ty), reader)
}

pub(crate) extern "C" fn stream_new<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
) -> u64 {
    guest_new::<T>(vmctx, TableIndex::Stream(ty))
}

pub(crate) extern "C" fn stream_write<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeStreamTableIndex,
    stream: u32,
    address: u32,
    count: u32,
) -> u64 {
    guest_write::<T>(
        vmctx,
        memory,
        realloc,
        string_encoding,
        TableIndex::Stream(ty),
        None,
        stream,
        address,
        count,
    )
}

pub(crate) extern "C" fn stream_read<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeStreamTableIndex,
    stream: u32,
    address: u32,
    count: u32,
) -> u64 {
    guest_read::<T>(
        vmctx,
        memory,
        realloc,
        string_encoding,
        TableIndex::Stream(ty),
        None,
        stream,
        address,
        count,
    )
}

pub(crate) extern "C" fn stream_cancel_write<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    async_: bool,
    writer: u32,
) -> u64 {
    guest_cancel_write::<T>(vmctx, TableIndex::Stream(ty), writer, async_)
}

pub(crate) extern "C" fn stream_cancel_read<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    async_: bool,
    reader: u32,
) -> u64 {
    guest_cancel_read::<T>(vmctx, TableIndex::Stream(ty), reader, async_)
}

pub(crate) extern "C" fn stream_close_writable<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    writer: u32,
    error: u32,
) -> bool {
    guest_close_writable::<T>(vmctx, TableIndex::Stream(ty), writer, error)
}

pub(crate) extern "C" fn stream_close_readable<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeStreamTableIndex,
    reader: u32,
) -> bool {
    guest_close_readable::<T>(vmctx, TableIndex::Stream(ty), reader)
}

pub(crate) extern "C" fn flat_stream_write<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    ty: TypeStreamTableIndex,
    payload_size: u32,
    payload_align: u32,
    stream: u32,
    address: u32,
    count: u32,
) -> u64 {
    guest_write::<T>(
        vmctx,
        memory,
        realloc,
        StringEncoding::Utf8 as u8,
        TableIndex::Stream(ty),
        Some(FlatAbi {
            size: payload_size,
            align: payload_align,
        }),
        stream,
        address,
        count,
    )
}

pub(crate) extern "C" fn flat_stream_read<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    ty: TypeStreamTableIndex,
    payload_size: u32,
    payload_align: u32,
    stream: u32,
    address: u32,
    count: u32,
) -> u64 {
    guest_read::<T>(
        vmctx,
        memory,
        realloc,
        StringEncoding::Utf8 as u8,
        TableIndex::Stream(ty),
        Some(FlatAbi {
            size: payload_size,
            align: payload_align,
        }),
        stream,
        address,
        count,
    )
}

pub(crate) extern "C" fn error_context_new<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeErrorContextTableIndex,
    address: u32,
    count: u32,
) -> u64 {
    unsafe {
        call_host_and_handle_result::<T, u32>(vmctx, || {
            _ = (
                vmctx,
                memory,
                realloc,
                StringEncoding::from_u8(string_encoding).unwrap(),
                ty,
                address,
                count,
            );
            bail!("todo: `error.new` not yet implemented");
        })
    }
}

pub(crate) extern "C" fn error_context_debug_message<T>(
    vmctx: *mut VMOpaqueContext,
    memory: *mut VMMemoryDefinition,
    realloc: *mut VMFuncRef,
    string_encoding: u8,
    ty: TypeErrorContextTableIndex,
    handle: u32,
    address: u32,
) -> bool {
    unsafe {
        call_host_and_handle_result::<T, ()>(vmctx, || {
            _ = (
                vmctx,
                memory,
                realloc,
                StringEncoding::from_u8(string_encoding).unwrap(),
                ty,
                handle,
                address,
            );
            bail!("todo: `error.debug-message` not yet implemented");
        })
    }
}

pub(crate) extern "C" fn error_context_drop<T>(
    vmctx: *mut VMOpaqueContext,
    ty: TypeErrorContextTableIndex,
    error_context: u32,
) -> bool {
    unsafe {
        call_host_and_handle_result::<T, _>(vmctx, || {
            let cx = VMComponentContext::from_opaque(vmctx);
            let instance = (*cx).instance();
            let (_, count) =
                (*instance).component_error_context_tables()[ty].get_mut_by_index(error_context)?;
            assert!(*count > 0);
            *count -= 1;

            if *count == 0 {
                (*instance).component_error_context_tables()[ty].remove_by_index(error_context)?;
            }

            Ok(())
        })
    }
}
