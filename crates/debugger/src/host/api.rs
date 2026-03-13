//! Implementations of the host API traits.

use crate::host::opaque::OpaqueDebugger;
use crate::host::wit;
use crate::{DebugRunResult, host::bindings::wasm_type_to_val_type};
use std::pin::Pin;
use wasmtime::{
    Engine, ExnRef, FrameHandle, Func, Global, Instance, Memory, Module, OwnedRooted, Result,
    Table, Tag, Val, component::Resource, component::ResourceTable,
};
use wasmtime_wasi::p2::{DynPollable, Pollable, subscribe};

/// Representation of one debuggee: a store with debugged code inside,
/// under the control of the debugger.
pub struct Debuggee {
    /// The type-erased debugger implementation. This field is `Some`
    /// when execution is paused, and `None`, with ownership of the
    /// debugger (hence debuggee's store) passed to the future when
    /// executing.
    pub(crate) inner: Option<Box<dyn OpaqueDebugger + Send + 'static>>,

    /// A separate handle to the Engine, allowing incrementing the
    /// epoch (hence interrupting a running debuggee) without taking
    /// the mutex.
    pub(crate) engine: Engine,
}

impl Debuggee {
    /// Finish execution of the debuggee before returning.
    pub async fn finish(&mut self) -> Result<()> {
        if let Some(inner) = self.inner.as_mut() {
            inner.finish().await?;
        }
        Ok(())
    }
}

impl WasmValue {
    pub(crate) fn new(store: impl wasmtime::AsContextMut, val: Val) -> Result<WasmValue> {
        Ok(match val {
            Val::ExnRef(Some(rooted)) => {
                WasmValue::Exn(Some(rooted.to_owned_rooted(store).unwrap()))
            }
            Val::ExnRef(None) => WasmValue::Exn(None),
            Val::FuncRef(Some(f)) => WasmValue::Func(Some(f)),
            Val::FuncRef(None) => WasmValue::Func(None),
            Val::ExternRef(_) | Val::AnyRef(_) | Val::ContRef(_) => {
                return Err(wit::Error::UnsupportedType.into());
            }
            Val::I32(_) | Val::I64(_) | Val::F32(_) | Val::F64(_) | Val::V128(_) => {
                WasmValue::Primitive(val)
            }
        })
    }

    pub(crate) fn into_val(self, store: impl wasmtime::AsContextMut) -> Val {
        match self {
            WasmValue::Primitive(v) => v,
            WasmValue::Exn(Some(owned)) => Val::ExnRef(Some(owned.to_rooted(store))),
            WasmValue::Exn(None) => Val::ExnRef(None),
            WasmValue::Func(Some(f)) => Val::FuncRef(Some(f)),
            WasmValue::Func(None) => Val::FuncRef(None),
        }
    }
}

/// Representation of an async debug event that the debugger is
/// waiting on.
///
/// Cancel-safety: the non-cancel-safe OpaqueDebugger async methods
/// are called inside an `async move` block that owns the debugger.
/// `ready()` merely polls this stored future, which is always safe to
/// re-poll after cancelation. The debugger is returned in the `Done`
/// state and extracted by `finish()`.
pub struct EventFuture {
    state: EventFutureState,
}

enum EventFutureState {
    /// The future is running; owns the debugger.
    Running(
        Pin<
            Box<
                dyn Future<
                        Output = (
                            Box<dyn OpaqueDebugger + Send + 'static>,
                            Result<DebugRunResult>,
                        ),
                    > + Send,
            >,
        >,
    ),
    /// The future has completed; debugger is ready to be returned.
    Done {
        inner: Box<dyn OpaqueDebugger + Send + 'static>,
        result: Option<Result<DebugRunResult>>,
    },
}

impl EventFuture {
    fn new_single_step(
        mut inner: Box<dyn OpaqueDebugger + Send + 'static>,
        resumption: wit::ResumptionValue,
    ) -> Self {
        EventFuture {
            state: EventFutureState::Running(Box::pin(async move {
                if let Err(e) = inner.handle_resumption(&resumption).await {
                    return (inner, Err(e));
                }
                let result = inner.single_step().await;
                (inner, result)
            })),
        }
    }

    fn new_continue(
        mut inner: Box<dyn OpaqueDebugger + Send + 'static>,
        resumption: wit::ResumptionValue,
    ) -> Self {
        EventFuture {
            state: EventFutureState::Running(Box::pin(async move {
                if let Err(e) = inner.handle_resumption(&resumption).await {
                    return (inner, Err(e));
                }
                let result = inner.continue_().await;
                (inner, result)
            })),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi_io::poll::Pollable for EventFuture {
    async fn ready(&mut self) {
        match &mut self.state {
            EventFutureState::Running(future) => {
                let (inner, result) = future.await;
                self.state = EventFutureState::Done {
                    inner,
                    result: Some(result),
                };
            }
            EventFutureState::Done { .. } => {}
        }
    }
}

/// Representation of a frame within a debuggee.
#[derive(Clone)]
pub struct Frame(FrameHandle);

/// Representation of a Wasm exception object.
#[derive(Clone)]
pub struct WasmException(OwnedRooted<ExnRef>);

/// Representation of a Wasm value.
///
/// This is distinct from `wasmtime::Val` because we need the Owned
/// variants of GC references here.
#[derive(Clone)]
pub enum WasmValue {
    /// A primitive (non-GC) value.
    Primitive(Val),
    /// An exception object.
    Exn(Option<OwnedRooted<ExnRef>>),
    /// A funcref.
    Func(Option<Func>),
    // TODO: GC structs and arrays.
}

/// Get the `OpaqueDebugger` or raise an error.
fn debugger<'a>(
    table: &'a mut ResourceTable,
    debuggee: &Resource<Debuggee>,
) -> Result<&'a mut dyn OpaqueDebugger> {
    let d = table.get_mut(&debuggee)?.inner.as_mut().ok_or_else(|| {
        wasmtime::error::format_err!("Attempt to use debuggee API while a future is pending")
    })?;
    Ok(&mut **d)
}

impl wit::HostDebuggee for ResourceTable {
    async fn all_modules(&mut self, debuggee: Resource<Debuggee>) -> Result<Vec<Resource<Module>>> {
        let d = debugger(self, &debuggee)?;
        let modules = d.all_modules().await?;
        let mut resources = vec![];
        for module in modules {
            resources.push(self.push_child(module, &debuggee)?);
        }
        Ok(resources)
    }

    async fn all_instances(
        &mut self,
        debuggee: Resource<Debuggee>,
    ) -> Result<Vec<Resource<Instance>>> {
        let d = debugger(self, &debuggee)?;
        let instances = d.all_instances().await?;
        let mut resources = vec![];
        for instance in instances {
            resources.push(self.push_child(instance, &debuggee)?);
        }
        Ok(resources)
    }

    async fn interrupt(&mut self, debuggee: Resource<Debuggee>) -> Result<()> {
        let d = self.get_mut(&debuggee)?;
        d.engine.increment_epoch();
        Ok(())
    }

    async fn single_step(
        &mut self,
        debuggee: Resource<Debuggee>,
        resumption: wit::ResumptionValue,
    ) -> Result<Resource<EventFuture>> {
        let d = self.get_mut(&debuggee).unwrap().inner.take().unwrap();
        Ok(self.push_child(EventFuture::new_single_step(d, resumption), &debuggee)?)
    }

    async fn continue_(
        &mut self,
        debuggee: Resource<Debuggee>,
        resumption: wit::ResumptionValue,
    ) -> Result<Resource<EventFuture>> {
        let d = self.get_mut(&debuggee).unwrap().inner.take().unwrap();
        Ok(self.push_child(EventFuture::new_continue(d, resumption), &debuggee)?)
    }

    async fn exit_frames(&mut self, debuggee: Resource<Debuggee>) -> Result<Vec<Resource<Frame>>> {
        let d = debugger(self, &debuggee)?;
        let frames = d.exit_frames().await?;
        let mut result = vec![];
        for frame in frames {
            result.push(self.push_child(Frame(frame), &debuggee)?);
        }
        Ok(result)
    }

    async fn drop(&mut self, debuggee: Resource<Debuggee>) -> Result<()> {
        self.delete(debuggee)?;
        Ok(())
    }
}

fn result_to_event(table: &mut ResourceTable, value: DebugRunResult) -> Result<wit::Event> {
    Ok(match value {
        DebugRunResult::Finished => wit::Event::Complete,
        DebugRunResult::HostcallError => wit::Event::Trap,
        DebugRunResult::Trap(_t) => wit::Event::Trap,
        DebugRunResult::Breakpoint => wit::Event::Breakpoint,
        DebugRunResult::EpochYield => wit::Event::Interrupted,
        DebugRunResult::CaughtExceptionThrown(e) => {
            let e = table.push(WasmException(e))?;
            wit::Event::CaughtExceptionThrown(e)
        }
        DebugRunResult::UncaughtExceptionThrown(e) => {
            let e = table.push(WasmException(e))?;
            wit::Event::UncaughtExceptionThrown(e)
        }
    })
}

impl wit::HostEventFuture for ResourceTable {
    async fn finish(
        &mut self,
        self_: Resource<EventFuture>,
        debuggee: Resource<Debuggee>,
    ) -> Result<wit::Event> {
        let mut f = self.delete(self_)?;
        f.ready().await;
        match f.state {
            EventFutureState::Running(..) => {
                unreachable!("ready() cannot return until setting Done state")
            }
            EventFutureState::Done { inner, result } => {
                self.get_mut(&debuggee)?.inner = Some(inner);
                match result.unwrap() {
                    Ok(result) => Ok(result_to_event(self, result)?),
                    Err(e) => Err(e),
                }
            }
        }
    }

    async fn drop(&mut self, rep: Resource<EventFuture>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }

    async fn subscribe(&mut self, self_: Resource<EventFuture>) -> Result<Resource<DynPollable>> {
        subscribe(self, self_)
    }
}

impl wit::HostInstance for ResourceTable {
    async fn get_module(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
    ) -> Result<Resource<Module>> {
        let i = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let module = d.get_instance_module(i).await?;
        let module = self.push(module)?;
        Ok(module)
    }

    async fn get_memory(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
        memory_index: u32,
    ) -> Result<Resource<Memory>> {
        let instance = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let memory = d
            .instance_get_memory(instance, memory_index)
            .await?
            .ok_or(wit::Error::InvalidEntity)?;
        Ok(self.push(memory)?)
    }

    async fn get_global(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
        global_index: u32,
    ) -> Result<Resource<Global>> {
        let instance = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let global = d
            .instance_get_global(instance, global_index)
            .await?
            .ok_or(wit::Error::InvalidEntity)?;
        Ok(self.push(global)?)
    }

    async fn get_table(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
        table_index: u32,
    ) -> Result<Resource<Table>> {
        let instance = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let table = d
            .instance_get_table(instance, table_index)
            .await?
            .ok_or(wit::Error::InvalidEntity)?;
        Ok(self.push(table)?)
    }

    async fn get_func(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
        func_index: u32,
    ) -> Result<Resource<Func>> {
        let instance = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let func = d
            .instance_get_func(instance, func_index)
            .await?
            .ok_or(wit::Error::InvalidEntity)?;
        Ok(self.push(func)?)
    }

    async fn get_tag(
        &mut self,
        self_: Resource<Instance>,
        d: Resource<Debuggee>,
        tag_index: u32,
    ) -> Result<Resource<Tag>> {
        let instance = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let tag = d
            .instance_get_tag(instance, tag_index)
            .await?
            .ok_or(wit::Error::InvalidEntity)?;
        Ok(self.push(tag)?)
    }

    async fn clone(&mut self, self_: Resource<Instance>) -> Result<Resource<Instance>> {
        let instance = *self.get(&self_)?;
        Ok(self.push(instance)?)
    }

    async fn unique_id(&mut self, self_: Resource<Instance>) -> Result<u64> {
        let instance = self.get(&self_)?;
        Ok(u64::from(instance.debug_index_in_store()))
    }

    async fn drop(&mut self, rep: Resource<Instance>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostModule for ResourceTable {
    async fn add_breakpoint(
        &mut self,
        self_: Resource<Module>,
        d: Resource<Debuggee>,
        pc: u32,
    ) -> Result<()> {
        let module = self.get(&self_)?.clone();
        let d = debugger(self, &d)?;
        d.module_add_breakpoint(module, pc).await
    }

    async fn remove_breakpoint(
        &mut self,
        self_: Resource<Module>,
        d: Resource<Debuggee>,
        pc: u32,
    ) -> Result<()> {
        let module = self.get(&self_)?.clone();
        let d = debugger(self, &d)?;
        d.module_remove_breakpoint(module, pc).await
    }

    async fn bytecode(&mut self, self_: Resource<Module>) -> Result<Option<Vec<u8>>> {
        let module = self.get(&self_)?;
        Ok(module.debug_bytecode().map(|b| b.to_vec()))
    }

    async fn clone(&mut self, self_: Resource<Module>) -> Result<Resource<Module>> {
        let module = self.get(&self_)?.clone();
        Ok(self.push(module)?)
    }

    async fn unique_id(&mut self, self_: Resource<Module>) -> Result<u64> {
        let module = self.get(&self_)?;
        Ok(module.debug_index_in_engine())
    }

    async fn drop(&mut self, rep: Resource<Module>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostMemory for ResourceTable {
    async fn size_bytes(&mut self, self_: Resource<Memory>, d: Resource<Debuggee>) -> Result<u64> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_size_bytes(memory).await
    }

    async fn page_size_bytes(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
    ) -> Result<u64> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_page_size(memory).await
    }

    async fn grow_to_bytes(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        delta_bytes: u64,
    ) -> Result<u64> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_grow(memory, delta_bytes).await
    }

    async fn get_bytes(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        len: u64,
    ) -> Result<Vec<u8>> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        Ok(d.memory_read_bytes(memory, addr, len)
            .await?
            .ok_or(wit::Error::OutOfBounds)?)
    }

    async fn set_bytes(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_write_bytes(memory, addr, bytes)
            .await?
            .ok_or(wit::Error::OutOfBounds)?;
        Ok(())
    }

    async fn get_u8(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
    ) -> Result<u8> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        Ok(d.memory_read_u8(memory, addr)
            .await?
            .ok_or(wit::Error::OutOfBounds)?)
    }

    async fn get_u16(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
    ) -> Result<u16> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        Ok(d.memory_read_u16(memory, addr)
            .await?
            .ok_or(wit::Error::OutOfBounds)?)
    }

    async fn get_u32(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
    ) -> Result<u32> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        Ok(d.memory_read_u32(memory, addr)
            .await?
            .ok_or(wit::Error::OutOfBounds)?)
    }

    async fn get_u64(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
    ) -> Result<u64> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        Ok(d.memory_read_u64(memory, addr)
            .await?
            .ok_or(wit::Error::OutOfBounds)?)
    }

    async fn set_u8(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        value: u8,
    ) -> Result<()> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_write_u8(memory, addr, value)
            .await?
            .ok_or(wit::Error::OutOfBounds)?;
        Ok(())
    }

    async fn set_u16(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        value: u16,
    ) -> Result<()> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_write_u16(memory, addr, value)
            .await?
            .ok_or(wit::Error::OutOfBounds)?;
        Ok(())
    }

    async fn set_u32(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        value: u32,
    ) -> Result<()> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_write_u32(memory, addr, value)
            .await?
            .ok_or(wit::Error::OutOfBounds)?;
        Ok(())
    }

    async fn set_u64(
        &mut self,
        self_: Resource<Memory>,
        d: Resource<Debuggee>,
        addr: u64,
        value: u64,
    ) -> Result<()> {
        let memory = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.memory_write_u64(memory, addr, value)
            .await?
            .ok_or(wit::Error::OutOfBounds)?;
        Ok(())
    }

    async fn clone(&mut self, self_: Resource<Memory>) -> Result<Resource<Memory>> {
        let memory = *self.get(&self_)?;
        Ok(self.push(memory)?)
    }

    async fn unique_id(&mut self, self_: Resource<Memory>) -> Result<u64> {
        Ok(self.get(&self_)?.debug_index_in_store())
    }

    async fn drop(&mut self, rep: Resource<Memory>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostGlobal for ResourceTable {
    async fn get(
        &mut self,
        self_: Resource<Global>,
        d: Resource<Debuggee>,
    ) -> Result<Resource<WasmValue>> {
        // N.B.: we use UFCS here because `HostGlobal::get` conflicts
        // with `ResourceTable::get` and we're implementing the WIT
        // trait directly on the `ResourceTable`.
        let global = *ResourceTable::get(self, &self_)?;
        let d = debugger(self, &d)?;
        let value = d.global_get(global).await?;
        Ok(self.push(value)?)
    }

    async fn set(
        &mut self,
        self_: Resource<Global>,
        d: Resource<Debuggee>,
        val: Resource<WasmValue>,
    ) -> Result<()> {
        let global = *ResourceTable::get(self, &self_)?;
        let value = ResourceTable::get(self, &val)?.clone();
        let d = debugger(self, &d)?;
        d.global_set(global, value).await
    }

    async fn clone(&mut self, self_: Resource<Global>) -> Result<Resource<Global>> {
        let global = *ResourceTable::get(self, &self_)?;
        Ok(self.push(global)?)
    }

    async fn unique_id(&mut self, self_: Resource<Global>) -> Result<u64> {
        let global = *ResourceTable::get(self, &self_)?;
        Ok(global.debug_index_in_store())
    }

    async fn drop(&mut self, rep: Resource<Global>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostTable for ResourceTable {
    async fn len(&mut self, self_: Resource<Table>, d: Resource<Debuggee>) -> Result<u64> {
        let table = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.table_len(table).await
    }

    async fn get_element(
        &mut self,
        self_: Resource<Table>,
        d: Resource<Debuggee>,
        index: u64,
    ) -> Result<Resource<WasmValue>> {
        let table = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        let value = d.table_get_element(table, index).await?;
        Ok(self.push(value)?)
    }

    async fn set_element(
        &mut self,
        self_: Resource<Table>,
        d: Resource<Debuggee>,
        index: u64,
        val: Resource<WasmValue>,
    ) -> Result<()> {
        let table = *self.get(&self_)?;
        let value = self.get(&val)?.clone();
        let d = debugger(self, &d)?;
        d.table_set_element(table, index, value).await
    }

    async fn clone(&mut self, self_: Resource<Table>) -> Result<Resource<Table>> {
        let table = *self.get(&self_)?;
        Ok(self.push(table)?)
    }

    async fn unique_id(&mut self, self_: Resource<Table>) -> Result<u64> {
        Ok(self.get(&self_)?.debug_index_in_store())
    }

    async fn drop(&mut self, rep: Resource<Table>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostWasmFunc for ResourceTable {
    async fn params(
        &mut self,
        self_: Resource<Func>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<wit::WasmType>> {
        let func = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.func_params(func).await
    }

    async fn results(
        &mut self,
        self_: Resource<Func>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<wit::WasmType>> {
        let func = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.func_results(func).await
    }

    async fn clone(&mut self, self_: Resource<Func>) -> Result<Resource<Func>> {
        let func = *self.get(&self_)?;
        Ok(self.push(func)?)
    }

    async fn drop(&mut self, rep: Resource<Func>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostWasmException for ResourceTable {
    async fn get_tag(
        &mut self,
        self_: Resource<WasmException>,
        d: Resource<Debuggee>,
    ) -> Result<Resource<Tag>> {
        let exn = self.get(&self_)?.clone();
        let d = debugger(self, &d)?;
        let tag = d.exnref_get_tag(exn.0).await?;
        Ok(self.push(tag)?)
    }

    async fn get_values(
        &mut self,
        self_: Resource<WasmException>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<Resource<WasmValue>>> {
        let exn = self.get(&self_)?.clone();
        let d = debugger(self, &d)?;
        let values = d.exnref_get_fields(exn.0).await?;
        let mut resources = vec![];
        for v in values {
            resources.push(self.push(v)?);
        }
        Ok(resources)
    }

    async fn clone(
        &mut self,
        self_: Resource<WasmException>,
        _d: Resource<Debuggee>,
    ) -> Result<Resource<WasmException>> {
        let exn = self.get(&self_)?.clone();
        Ok(self.push(exn)?)
    }

    async fn make(
        &mut self,
        d: Resource<Debuggee>,
        tag: Resource<Tag>,
        values: Vec<Resource<WasmValue>>,
    ) -> Result<Resource<WasmException>> {
        let tag_val = *self.get(&tag)?;
        let mut wasm_values = vec![];
        for v in &values {
            wasm_values.push(self.get(v)?.clone());
        }
        let d = debugger(self, &d)?;
        let owned = d.exnref_new(tag_val, wasm_values).await?;
        Ok(self.push(WasmException(owned))?)
    }

    async fn drop(&mut self, rep: Resource<WasmException>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostWasmTag for ResourceTable {
    async fn params(
        &mut self,
        self_: Resource<Tag>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<wit::WasmType>> {
        let tag = *self.get(&self_)?;
        let d = debugger(self, &d)?;
        d.tag_params(tag).await
    }

    async fn unique_id(&mut self, self_: Resource<Tag>) -> Result<u64> {
        Ok(self.get(&self_)?.debug_index_in_store())
    }

    async fn clone(&mut self, self_: Resource<Tag>) -> Result<Resource<Tag>> {
        let tag = *self.get(&self_)?;
        Ok(self.push(tag)?)
    }

    async fn make(
        &mut self,
        d: Resource<Debuggee>,
        params: Vec<wit::WasmType>,
    ) -> Result<Resource<Tag>> {
        let engine = self.get(&d)?.engine.clone();
        let val_types = params.into_iter().map(wasm_type_to_val_type).collect();
        let d = debugger(self, &d)?;
        let tag = d.tag_new(engine, val_types).await?;
        Ok(self.push(tag)?)
    }

    async fn drop(&mut self, rep: Resource<Tag>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostFrame for ResourceTable {
    async fn get_instance(
        &mut self,
        self_: Resource<Frame>,
        d: Resource<Debuggee>,
    ) -> Result<Resource<Instance>> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let instance = d.frame_instance(frame).await?;
        Ok(self.push(instance)?)
    }

    async fn get_func_index(
        &mut self,
        self_: Resource<Frame>,
        d: Resource<Debuggee>,
    ) -> Result<u32> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let (f, _) = d.frame_func_and_pc(frame).await?;
        Ok(f)
    }

    async fn get_pc(&mut self, self_: Resource<Frame>, d: Resource<Debuggee>) -> Result<u32> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let (_, pc) = d.frame_func_and_pc(frame).await?;
        Ok(pc)
    }

    async fn get_locals(
        &mut self,
        self_: Resource<Frame>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<Resource<WasmValue>>> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let locals = d.frame_locals(frame).await?;
        let mut resources = vec![];
        for local in locals {
            resources.push(self.push(local)?);
        }
        Ok(resources)
    }

    async fn get_stack(
        &mut self,
        self_: Resource<Frame>,
        d: Resource<Debuggee>,
    ) -> Result<Vec<Resource<WasmValue>>> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let stacks = d.frame_stack(frame).await?;
        let mut resources = vec![];
        for val in stacks {
            resources.push(self.push(val)?);
        }
        Ok(resources)
    }

    async fn parent_frame(
        &mut self,
        self_: Resource<Frame>,
        d: Resource<Debuggee>,
    ) -> Result<Option<Resource<Frame>>> {
        let frame = self.get(&self_)?.0.clone();
        let d = debugger(self, &d)?;
        let parent = d.frame_parent(frame).await?;
        match parent {
            Some(p) => Ok(Some(self.push(Frame(p))?)),
            None => Ok(None),
        }
    }

    async fn drop(&mut self, rep: Resource<Frame>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::HostWasmValue for ResourceTable {
    async fn get_type(&mut self, self_: Resource<WasmValue>) -> Result<wit::WasmType> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::I32(_)) => Ok(wit::WasmType::WasmI32),
            WasmValue::Primitive(Val::I64(_)) => Ok(wit::WasmType::WasmI64),
            WasmValue::Primitive(Val::F32(_)) => Ok(wit::WasmType::WasmF32),
            WasmValue::Primitive(Val::F64(_)) => Ok(wit::WasmType::WasmF64),
            WasmValue::Primitive(Val::V128(_)) => Ok(wit::WasmType::WasmV128),
            WasmValue::Func(_) => Ok(wit::WasmType::WasmFuncref),
            WasmValue::Exn(_) => Ok(wit::WasmType::WasmExnref),
            WasmValue::Primitive(_) => unreachable!(),
        }
    }

    async fn unwrap_i32(&mut self, self_: Resource<WasmValue>) -> Result<u32> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::I32(x)) => Ok(x.cast_unsigned()),
            _ => wasmtime::bail!("Wasm value is not an i32."),
        }
    }

    async fn unwrap_i64(&mut self, self_: Resource<WasmValue>) -> Result<u64> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::I64(x)) => Ok(x.cast_unsigned()),
            _ => wasmtime::bail!("Wasm value is not an i64."),
        }
    }

    async fn unwrap_f32(&mut self, self_: Resource<WasmValue>) -> Result<f32> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::F32(x)) => Ok(f32::from_bits(*x)),
            _ => wasmtime::bail!("Wasm value is not an f32."),
        }
    }

    async fn unwrap_f64(&mut self, self_: Resource<WasmValue>) -> Result<f64> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::F64(x)) => Ok(f64::from_bits(*x)),
            _ => wasmtime::bail!("Wasm value is not an f64."),
        }
    }

    async fn unwrap_v128(&mut self, self_: Resource<WasmValue>) -> Result<Vec<u8>> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Primitive(Val::V128(x)) => Ok(x.as_u128().to_le_bytes().to_vec()),
            _ => wasmtime::bail!("Wasm value is not a v128."),
        }
    }

    async fn unwrap_func(&mut self, self_: Resource<WasmValue>) -> Result<Option<Resource<Func>>> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Func(Some(f)) => {
                let f = *f;
                Ok(Some(self.push(f)?))
            }
            WasmValue::Func(None) => Ok(None),
            _ => wasmtime::bail!("Wasm value is not a funcref."),
        }
    }

    async fn unwrap_exception(
        &mut self,
        self_: Resource<WasmValue>,
    ) -> Result<Option<Resource<WasmException>>> {
        let value = self.get(&self_)?;
        match value {
            WasmValue::Exn(Some(e)) => {
                let e = e.clone();
                Ok(Some(self.push(WasmException(e))?))
            }
            WasmValue::Exn(None) => Ok(None),
            _ => wasmtime::bail!("Wasm value is not an exnref."),
        }
    }

    async fn make_i32(&mut self, value: u32) -> Result<Resource<WasmValue>> {
        Ok(self.push(WasmValue::Primitive(Val::I32(value.cast_signed())))?)
    }

    async fn make_i64(&mut self, value: u64) -> Result<Resource<WasmValue>> {
        Ok(self.push(WasmValue::Primitive(Val::I64(value.cast_signed())))?)
    }

    async fn make_f32(&mut self, value: f32) -> Result<Resource<WasmValue>> {
        Ok(self.push(WasmValue::Primitive(Val::F32(value.to_bits())))?)
    }

    async fn make_f64(&mut self, value: f64) -> Result<Resource<WasmValue>> {
        Ok(self.push(WasmValue::Primitive(Val::F64(value.to_bits())))?)
    }

    async fn make_v128(&mut self, value: Vec<u8>) -> Result<Resource<WasmValue>> {
        let bytes: [u8; 16] = value
            .try_into()
            .map_err(|_| wasmtime::format_err!("v128 requires exactly 16 bytes"))?;
        Ok(self.push(WasmValue::Primitive(Val::V128(
            u128::from_le_bytes(bytes).into(),
        )))?)
    }

    async fn clone(&mut self, self_: Resource<WasmValue>) -> Result<Resource<WasmValue>> {
        let value = self.get(&self_)?.clone();
        Ok(self.push(value)?)
    }

    async fn drop(&mut self, rep: Resource<WasmValue>) -> Result<()> {
        self.delete(rep)?;
        Ok(())
    }
}

impl wit::Host for ResourceTable {
    fn convert_error(&mut self, err: wasmtime::Error) -> Result<wit::Error> {
        err.downcast()
    }
}
