//! Host implementation for the debugger world.

use wasmtime::{
    Engine, ExnRef, ExnRefPre, ExnType, FrameHandle, Func, FuncType, Global, Instance, Memory,
    Module, OwnedRooted, Result, Table, Tag, TagType, Val, ValType,
    component::{Resource, ResourceTable},
};

mod generated {
    ::wasmtime::component::bindgen!({
        path: "wit",
        wasmtime_crate: ::wasmtime,
        world: "bytecodealliance:wasmtime/debug-main",
        imports: {
            // Everything is async, even the seemingly simple things
            // like unwrapping a Wasm value, because we need to access
            // the Store in many places and that is an async access
            // via channels within the debuggee.
            default: async | trappable
        },
        exports: {
            default: async,
        },
        with: {
            "bytecodealliance:wasmtime/debuggee.debuggee": super::Debuggee,
            "bytecodealliance:wasmtime/debuggee.event-future": super::EventFuture,
            "bytecodealliance:wasmtime/debuggee.frame": super::Frame,
            "bytecodealliance:wasmtime/debuggee.instance": ::wasmtime::Instance,
            "bytecodealliance:wasmtime/debuggee.module": ::wasmtime::Module,
            "bytecodealliance:wasmtime/debuggee.table": ::wasmtime::Table,
            "bytecodealliance:wasmtime/debuggee.global": ::wasmtime::Global,
            "bytecodealliance:wasmtime/debuggee.memory": ::wasmtime::Memory,
            "bytecodealliance:wasmtime/debuggee.wasm-tag": ::wasmtime::Tag,
            "bytecodealliance:wasmtime/debuggee.wasm-func": ::wasmtime::Func,
            "bytecodealliance:wasmtime/debuggee.wasm-exception": super::WasmException,
            "bytecodealliance:wasmtime/debuggee.wasm-value": super::WasmValue,

            "wasi": wasmtime_wasi::p2::bindings,
        },
        trappable_error_type: {
            "bytecodealliance:wasmtime/debuggee.error" => wasmtime::Error,
        },
        require_store_data_send: true,
    });
}

pub use generated::DebugMain as DebuggerComponent;
pub use generated::bytecodealliance::wasmtime::debuggee as wit;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};

use crate::{DebugRunResult, Debugger};

/// Representation of one debuggee: a store with debugged code inside,
/// under the control of the debugger.
pub struct Debuggee {
    /// The type-erased debugger implementation. This field is `Some`
    /// when execution is paused, and `None`, with ownership of the
    /// debugger (hence debuggee's store) passed to the future when
    /// executing.
    inner: Option<Box<dyn OpaqueDebugger + Send + 'static>>,

    /// A separate handle to the Engine, allowing incrementing the
    /// epoch (hence interrupting a running debuggee) without taking
    /// the mutex.
    engine: Engine,
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

/// Type-erased interface to the `Debugger<T>` implementing all
/// functionality necessary for the interfaces here. This needs to be
/// type-erased because the host-side resource APIs do not support
/// type-parameterized resource kinds -- e.g., we cannot have a
/// resource for a `Debugger<T>`, only a `Debugger`, so the debuggee
/// resource essentially needs to carry a vtable for the kind of store
/// the debuggee has.
///
/// Methods here return `wasmtime::Result<T>`, where `Err` may wrap
/// either a `wit::Error` (which `convert_error` will extract and
/// return as an in-band WIT-level error to the component) or any
/// other error (which becomes a trap).
///
/// These methods do not handle the "wrong state" errors (i.e.,
/// execution is continuing so we cannot query store state): those are
/// handled one level up, via moving ownership of the instance of this
/// trait between the execution future and the debuggee resource
/// itself.
#[async_trait::async_trait]
trait OpaqueDebugger {
    async fn all_instances(&mut self) -> Result<Vec<Instance>>;
    async fn all_modules(&mut self) -> Result<Vec<Module>>;
    async fn handle_resumption(&mut self, resumption: &wit::ResumptionValue) -> Result<()>;
    async fn single_step(&mut self) -> Result<crate::DebugRunResult>;
    async fn continue_(&mut self) -> Result<crate::DebugRunResult>;
    async fn exit_frames(&mut self) -> Result<Vec<FrameHandle>>;
    async fn get_instance_module(&mut self, instance: Instance) -> Result<Module>;

    async fn instance_get_memory(&mut self, instance: Instance, idx: u32)
    -> Result<Option<Memory>>;
    async fn instance_get_global(&mut self, instance: Instance, idx: u32)
    -> Result<Option<Global>>;
    async fn instance_get_table(&mut self, instance: Instance, idx: u32) -> Result<Option<Table>>;
    async fn instance_get_func(&mut self, instance: Instance, idx: u32) -> Result<Option<Func>>;
    async fn instance_get_tag(&mut self, instance: Instance, idx: u32) -> Result<Option<Tag>>;

    async fn memory_size_bytes(&mut self, memory: Memory) -> Result<u64>;
    async fn memory_page_size(&mut self, memory: Memory) -> Result<u64>;
    async fn memory_grow(&mut self, memory: Memory, delta_bytes: u64) -> Result<u64>;
    async fn memory_read_u8(&mut self, memory: Memory, addr: u64) -> Result<Option<u8>>;
    async fn memory_read_u16(&mut self, memory: Memory, addr: u64) -> Result<Option<u16>>;
    async fn memory_read_u32(&mut self, memory: Memory, addr: u64) -> Result<Option<u32>>;
    async fn memory_read_u64(&mut self, memory: Memory, addr: u64) -> Result<Option<u64>>;
    async fn memory_write_u8(&mut self, memory: Memory, addr: u64, data: u8) -> Result<Option<()>>;
    async fn memory_write_u16(
        &mut self,
        memory: Memory,
        addr: u64,
        data: u16,
    ) -> Result<Option<()>>;
    async fn memory_write_u32(
        &mut self,
        memory: Memory,
        addr: u64,
        data: u32,
    ) -> Result<Option<()>>;
    async fn memory_write_u64(
        &mut self,
        memory: Memory,
        addr: u64,
        data: u64,
    ) -> Result<Option<()>>;

    async fn global_get(&mut self, global: Global) -> Result<WasmValue>;
    async fn global_set(&mut self, global: Global, val: WasmValue) -> Result<()>;

    async fn table_len(&mut self, table: Table) -> Result<u64>;
    async fn table_get_element(&mut self, table: Table, index: u64) -> Result<WasmValue>;
    async fn table_set_element(&mut self, table: Table, index: u64, val: WasmValue) -> Result<()>;

    async fn func_params(&mut self, func: Func) -> Result<Vec<wit::WasmType>>;
    async fn func_results(&mut self, func: Func) -> Result<Vec<wit::WasmType>>;

    async fn tag_params(&mut self, tag: Tag) -> Result<Vec<wit::WasmType>>;
    async fn tag_new(&mut self, engine: Engine, params: Vec<ValType>) -> Result<Tag>;

    async fn exnref_get_tag(&mut self, exn: OwnedRooted<ExnRef>) -> Result<Tag>;
    async fn exnref_get_fields(&mut self, exn: OwnedRooted<ExnRef>) -> Result<Vec<WasmValue>>;
    async fn exnref_new(&mut self, tag: Tag, fields: Vec<WasmValue>)
    -> Result<OwnedRooted<ExnRef>>;

    async fn frame_instance(&mut self, frame: FrameHandle) -> Result<Instance>;
    async fn frame_func_and_pc(&mut self, frame: FrameHandle) -> Result<(u32, u32)>;
    async fn frame_locals(&mut self, frame: FrameHandle) -> Result<Vec<WasmValue>>;
    async fn frame_stack(&mut self, frame: FrameHandle) -> Result<Vec<WasmValue>>;
    async fn frame_parent(&mut self, frame: FrameHandle) -> Result<Option<FrameHandle>>;

    async fn module_add_breakpoint(&mut self, module: Module, pc: u32) -> Result<()>;
    async fn module_remove_breakpoint(&mut self, module: Module, pc: u32) -> Result<()>;

    async fn finish(&mut self) -> Result<()>;
}

impl WasmValue {
    fn new(store: impl wasmtime::AsContextMut, val: Val) -> Result<WasmValue> {
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

    fn into_val(self, store: impl wasmtime::AsContextMut) -> Val {
        match self {
            WasmValue::Primitive(v) => v,
            WasmValue::Exn(Some(owned)) => Val::ExnRef(Some(owned.to_rooted(store))),
            WasmValue::Exn(None) => Val::ExnRef(None),
            WasmValue::Func(Some(f)) => Val::FuncRef(Some(f)),
            WasmValue::Func(None) => Val::FuncRef(None),
        }
    }
}

fn val_type_to_wasm_type(vt: &ValType) -> Result<wit::WasmType> {
    match vt {
        ValType::I32 => Ok(wit::WasmType::WasmI32),
        ValType::I64 => Ok(wit::WasmType::WasmI64),
        ValType::F32 => Ok(wit::WasmType::WasmF32),
        ValType::F64 => Ok(wit::WasmType::WasmF64),
        ValType::V128 => Ok(wit::WasmType::WasmV128),
        ValType::Ref(rt) if rt.heap_type().is_exn() => Ok(wit::WasmType::WasmExnref),
        ValType::Ref(rt) if rt.heap_type().is_func() => Ok(wit::WasmType::WasmFuncref),
        ValType::Ref(_) => Err(wit::Error::UnsupportedType.into()),
    }
}

fn wasm_type_to_val_type(wt: wit::WasmType) -> ValType {
    match wt {
        wit::WasmType::WasmI32 => ValType::I32,
        wit::WasmType::WasmI64 => ValType::I64,
        wit::WasmType::WasmF32 => ValType::F32,
        wit::WasmType::WasmF64 => ValType::F64,
        wit::WasmType::WasmV128 => ValType::V128,
        wit::WasmType::WasmFuncref => ValType::FUNCREF,
        wit::WasmType::WasmExnref => ValType::EXNREF,
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static> OpaqueDebugger for crate::Debugger<T> {
    async fn all_instances(&mut self) -> Result<Vec<Instance>> {
        self.with_store(|store| store.debug_all_instances()).await
    }

    async fn all_modules(&mut self) -> Result<Vec<Module>> {
        self.with_store(|store| store.debug_all_modules()).await
    }

    async fn single_step(&mut self) -> Result<crate::DebugRunResult> {
        self.with_store(|store| store.edit_breakpoints().unwrap().single_step(true).unwrap())
            .await?;

        self.run().await
    }

    async fn continue_(&mut self) -> Result<crate::DebugRunResult> {
        self.with_store(|store| {
            store
                .edit_breakpoints()
                .unwrap()
                .single_step(false)
                .unwrap()
        })
        .await?;

        self.run().await
    }

    async fn handle_resumption(&mut self, resumption: &wit::ResumptionValue) -> Result<()> {
        match resumption {
            wit::ResumptionValue::Normal => {}
            _ => {
                unimplemented!("Non-`Normal` resumption not yet supported");
            }
        }
        Ok(())
    }

    async fn exit_frames(&mut self) -> Result<Vec<FrameHandle>> {
        self.with_store(|mut store| store.debug_exit_frames().collect::<Vec<_>>())
            .await
    }

    async fn get_instance_module(&mut self, instance: Instance) -> Result<Module> {
        self.with_store(move |store| instance.module(&store).clone())
            .await
    }

    async fn instance_get_memory(
        &mut self,
        instance: Instance,
        idx: u32,
    ) -> Result<Option<Memory>> {
        self.with_store(move |mut store| instance.debug_memory(&mut store, idx))
            .await
    }

    async fn instance_get_global(
        &mut self,
        instance: Instance,
        idx: u32,
    ) -> Result<Option<Global>> {
        self.with_store(move |mut store| instance.debug_global(&mut store, idx))
            .await
    }

    async fn instance_get_table(&mut self, instance: Instance, idx: u32) -> Result<Option<Table>> {
        self.with_store(move |mut store| instance.debug_table(&mut store, idx))
            .await
    }

    async fn instance_get_func(&mut self, instance: Instance, idx: u32) -> Result<Option<Func>> {
        self.with_store(move |mut store| instance.debug_function(&mut store, idx))
            .await
    }

    async fn instance_get_tag(&mut self, instance: Instance, idx: u32) -> Result<Option<Tag>> {
        self.with_store(move |mut store| instance.debug_tag(&mut store, idx))
            .await
    }

    async fn memory_size_bytes(&mut self, memory: Memory) -> Result<u64> {
        self.with_store(move |store| u64::try_from(memory.data_size(&store)).unwrap())
            .await
    }

    async fn memory_page_size(&mut self, memory: Memory) -> Result<u64> {
        self.with_store(move |store| memory.page_size(&store)).await
    }

    async fn memory_grow(&mut self, memory: Memory, delta_bytes: u64) -> Result<u64> {
        self.with_store(move |mut store| -> Result<u64> {
            let page_size = memory.page_size(&store);
            if delta_bytes & (page_size - 1) != 0 {
                return Err(wit::Error::MemoryGrowFailure.into());
            }
            let delta_pages = delta_bytes / page_size;
            let old_pages = memory
                .grow(&mut store, delta_pages)
                .map_err(|_| wit::Error::MemoryGrowFailure)?;
            Ok(old_pages * page_size)
        })
        .await?
    }

    async fn memory_read_u8(&mut self, memory: Memory, addr: u64) -> Result<Option<u8>> {
        self.with_store(move |store| {
            let data = memory.data(&store);
            let addr = usize::try_from(addr).unwrap();
            Some(*data.get(addr)?)
        })
        .await
    }

    async fn memory_read_u16(&mut self, memory: Memory, addr: u64) -> Result<Option<u16>> {
        self.with_store(move |store| {
            let data = memory.data(&store);
            let addr = usize::try_from(addr).unwrap();
            Some(u16::from_le_bytes([*data.get(addr)?, *data.get(addr + 1)?]))
        })
        .await
    }

    async fn memory_read_u32(&mut self, memory: Memory, addr: u64) -> Result<Option<u32>> {
        self.with_store(move |store| {
            let data = memory.data(&store);
            let addr = usize::try_from(addr).unwrap();
            Some(u32::from_le_bytes([
                *data.get(addr)?,
                *data.get(addr + 1)?,
                *data.get(addr + 2)?,
                *data.get(addr + 3)?,
            ]))
        })
        .await
    }

    async fn memory_read_u64(&mut self, memory: Memory, addr: u64) -> Result<Option<u64>> {
        self.with_store(move |store| {
            let data = memory.data(&store);
            let addr = usize::try_from(addr).unwrap();
            Some(u64::from_le_bytes([
                *data.get(addr)?,
                *data.get(addr + 1)?,
                *data.get(addr + 2)?,
                *data.get(addr + 3)?,
                *data.get(addr + 4)?,
                *data.get(addr + 5)?,
                *data.get(addr + 6)?,
                *data.get(addr + 7)?,
            ]))
        })
        .await
    }

    async fn memory_write_u8(
        &mut self,
        memory: Memory,
        addr: u64,
        value: u8,
    ) -> Result<Option<()>> {
        self.with_store(move |mut store| {
            let data = memory.data_mut(&mut store);
            let addr = usize::try_from(addr).unwrap();
            *data.get_mut(addr)? = value;
            Some(())
        })
        .await
    }

    async fn memory_write_u16(
        &mut self,
        memory: Memory,
        addr: u64,
        value: u16,
    ) -> Result<Option<()>> {
        self.with_store(move |mut store| {
            let data = memory.data_mut(&mut store);
            let addr = usize::try_from(addr).unwrap();
            data.get_mut(addr..(addr + 2))?
                .copy_from_slice(&value.to_le_bytes());
            Some(())
        })
        .await
    }

    async fn memory_write_u32(
        &mut self,
        memory: Memory,
        addr: u64,
        value: u32,
    ) -> Result<Option<()>> {
        self.with_store(move |mut store| {
            let data = memory.data_mut(&mut store);
            let addr = usize::try_from(addr).unwrap();
            data.get_mut(addr..(addr + 4))?
                .copy_from_slice(&value.to_le_bytes());
            Some(())
        })
        .await
    }

    async fn memory_write_u64(
        &mut self,
        memory: Memory,
        addr: u64,
        value: u64,
    ) -> Result<Option<()>> {
        self.with_store(move |mut store| {
            let data = memory.data_mut(&mut store);
            let addr = usize::try_from(addr).unwrap();
            data.get_mut(addr..(addr + 8))?
                .copy_from_slice(&value.to_le_bytes());
            Some(())
        })
        .await
    }

    async fn global_get(&mut self, global: Global) -> Result<WasmValue> {
        self.with_store(move |mut store| {
            let val = global.get(&mut store);
            WasmValue::new(&mut store, val)
        })
        .await?
    }

    async fn global_set(&mut self, global: Global, val: WasmValue) -> Result<()> {
        self.with_store(move |mut store| -> Result<()> {
            let v = val.into_val(&mut store);
            global
                .set(&mut store, v)
                .map_err(|_| wit::Error::MismatchedType)?;
            Ok(())
        })
        .await?
    }

    async fn table_len(&mut self, table: Table) -> Result<u64> {
        self.with_store(move |store| table.size(&store)).await
    }

    async fn table_get_element(&mut self, table: Table, index: u64) -> Result<WasmValue> {
        self.with_store(move |mut store| -> Result<WasmValue> {
            let val = table
                .get(&mut store, index)
                .ok_or(wit::Error::OutOfBounds)?;
            WasmValue::new(&mut store, val.into())
        })
        .await?
    }

    async fn table_set_element(&mut self, table: Table, index: u64, val: WasmValue) -> Result<()> {
        self.with_store(move |mut store| -> Result<()> {
            let v = val.into_val(&mut store);
            let r = v.ref_().ok_or(wit::Error::MismatchedType)?;
            table
                .set(&mut store, index, r)
                .map_err(|_| wit::Error::MismatchedType)?;
            Ok(())
        })
        .await?
    }

    async fn func_params(&mut self, func: Func) -> Result<Vec<wit::WasmType>> {
        self.with_store(move |store| {
            let ty = func.ty(&store);
            ty.params()
                .map(|ty| val_type_to_wasm_type(&ty))
                .collect::<Result<Vec<_>>>()
        })
        .await?
    }

    async fn func_results(&mut self, func: Func) -> Result<Vec<wit::WasmType>> {
        self.with_store(move |store| {
            let ty = func.ty(&store);
            ty.results()
                .map(|ty| val_type_to_wasm_type(&ty))
                .collect::<Result<Vec<_>>>()
        })
        .await?
    }

    async fn tag_params(&mut self, tag: Tag) -> Result<Vec<wit::WasmType>> {
        self.with_store(move |store| {
            let ty = tag.ty(&store);
            ty.ty()
                .params()
                .map(|ty| val_type_to_wasm_type(&ty))
                .collect::<Result<Vec<_>>>()
        })
        .await?
    }

    async fn tag_new(&mut self, engine: Engine, params: Vec<ValType>) -> Result<Tag> {
        self.with_store(move |mut store| {
            let func_ty = FuncType::new(&engine, params, []);
            let tag_ty = TagType::new(func_ty);
            Tag::new(&mut store, &tag_ty)
        })
        .await?
    }

    async fn exnref_get_tag(&mut self, exn: OwnedRooted<ExnRef>) -> Result<Tag> {
        self.with_store(move |mut store| exn.tag(&mut store).expect("reference must be rooted"))
            .await
    }

    async fn exnref_get_fields(&mut self, exn: OwnedRooted<ExnRef>) -> Result<Vec<WasmValue>> {
        self.with_store(move |mut store| {
            let fields = exn
                .fields(&mut store)
                .expect("reference must be rooted")
                .collect::<Vec<Val>>();
            fields
                .into_iter()
                .map(|v| WasmValue::new(&mut store, v))
                .collect::<Result<Vec<_>>>()
        })
        .await?
    }

    async fn exnref_new(
        &mut self,
        tag: Tag,
        fields: Vec<WasmValue>,
    ) -> Result<OwnedRooted<ExnRef>> {
        self.with_store(move |mut store| -> Result<OwnedRooted<ExnRef>> {
            let exn_ty =
                ExnType::from_tag_type(&tag.ty(&store)).expect("tag type is already validated");
            let allocator = ExnRefPre::new(&mut store, exn_ty);
            let field_vals = fields
                .into_iter()
                .map(|v| v.into_val(&mut store))
                .collect::<Vec<_>>();
            let exn = ExnRef::new(&mut store, &allocator, &tag, &field_vals)
                .map_err(|_| wit::Error::AllocFailure)?;
            Ok(exn.to_owned_rooted(&mut store).unwrap())
        })
        .await?
    }

    async fn frame_instance(&mut self, frame: FrameHandle) -> Result<Instance> {
        self.with_store(move |mut store| -> Result<Instance> {
            Ok(frame
                .instance(&mut store)
                .map_err(|_| wit::Error::InvalidFrame)?)
        })
        .await?
    }

    async fn frame_func_and_pc(&mut self, frame: FrameHandle) -> Result<(u32, u32)> {
        self.with_store(move |mut store| -> Result<(u32, u32)> {
            let (func, pc) = frame
                .wasm_function_index_and_pc(&mut store)
                .map_err(|_| wit::Error::InvalidFrame)?
                .ok_or(wit::Error::NonWasmFrame)?;
            Ok((func.as_u32(), pc))
        })
        .await?
    }

    async fn frame_locals(&mut self, frame: FrameHandle) -> Result<Vec<WasmValue>> {
        self.with_store(move |mut store| -> Result<Vec<WasmValue>> {
            let n_locals = frame
                .num_locals(&mut store)
                .map_err(|_| wit::Error::InvalidFrame)?;
            let mut result = vec![];
            for i in 0..n_locals {
                let val = frame
                    .local(&mut store, i)
                    .expect("checked for validity above");
                result.push(WasmValue::new(&mut store, val)?);
            }
            Ok(result)
        })
        .await?
    }

    async fn frame_stack(&mut self, frame: FrameHandle) -> Result<Vec<WasmValue>> {
        self.with_store(move |mut store| -> Result<Vec<WasmValue>> {
            let n_stacks = frame
                .num_stacks(&mut store)
                .map_err(|_| wit::Error::InvalidFrame)?;
            let mut result = vec![];
            for i in 0..n_stacks {
                let val = frame
                    .stack(&mut store, i)
                    .expect("checked for validity above");
                result.push(WasmValue::new(&mut store, val)?);
            }
            Ok(result)
        })
        .await?
    }

    async fn frame_parent(&mut self, frame: FrameHandle) -> Result<Option<FrameHandle>> {
        self.with_store(move |mut store| -> Result<Option<FrameHandle>> {
            Ok(frame
                .parent(&mut store)
                .map_err(|_| wit::Error::InvalidFrame)?)
        })
        .await?
    }

    async fn module_add_breakpoint(&mut self, module: Module, pc: u32) -> Result<()> {
        self.with_store(move |store| -> Result<()> {
            store
                .edit_breakpoints()
                .expect("guest debugging is enabled")
                .add_breakpoint(&module, pc)
                .map_err(|_| wit::Error::InvalidPc)?;
            Ok(())
        })
        .await?
    }

    async fn module_remove_breakpoint(&mut self, module: Module, pc: u32) -> Result<()> {
        self.with_store(move |store| -> Result<()> {
            store
                .edit_breakpoints()
                .expect("guest debugging is enabled")
                .remove_breakpoint(&module, pc)
                .map_err(|_| wit::Error::InvalidPc)?;
            Ok(())
        })
        .await?
    }

    async fn finish(&mut self) -> Result<()> {
        self.finish().await?;
        Ok(())
    }
}

/// Representation of an async debug event that the debugger is
/// waiting on.
pub struct EventFuture {
    inner: Box<dyn OpaqueDebugger + Send + 'static>,
    state: EventFutureState,
}

enum EventFutureState {
    SingleStep(Option<wit::ResumptionValue>),
    Continue(Option<wit::ResumptionValue>),
    Done(Result<DebugRunResult>),
}

#[async_trait::async_trait]
impl wasmtime_wasi_io::poll::Pollable for EventFuture {
    async fn ready(&mut self) {
        match &mut self.state {
            EventFutureState::SingleStep(resumption) => {
                if let Some(r) = resumption.as_ref() {
                    if let Err(e) = self.inner.handle_resumption(r).await {
                        self.state = EventFutureState::Done(Err(e));
                        return;
                    }
                    // Remove only after success, for cancel safety.
                    resumption.take();
                }
                let result = self.inner.single_step().await;
                self.state = EventFutureState::Done(result);
            }
            EventFutureState::Continue(resumption) => {
                if let Some(r) = resumption.as_ref() {
                    if let Err(e) = self.inner.handle_resumption(r).await {
                        self.state = EventFutureState::Done(Err(e));
                        return;
                    }
                    // Remove only after success, for cancel safety.
                    resumption.take();
                }
                let result = self.inner.continue_().await;
                self.state = EventFutureState::Done(result);
            }
            EventFutureState::Done(_) => {}
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

/// Register a debuggee in a resource table.
pub fn add_debuggee<T: Send + 'static>(
    table: &mut ResourceTable,
    debuggee: Debugger<T>,
) -> Result<Resource<Debuggee>> {
    let engine = debuggee.engine().clone();
    let inner: Option<Box<dyn OpaqueDebugger + Send + 'static>> = Some(Box::new(debuggee));
    Ok(table.push(Debuggee { inner, engine })?)
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
        Ok(self.push_child(
            EventFuture {
                inner: d,
                state: EventFutureState::SingleStep(Some(resumption)),
            },
            &debuggee,
        )?)
    }

    async fn continue_(
        &mut self,
        debuggee: Resource<Debuggee>,
        resumption: wit::ResumptionValue,
    ) -> Result<Resource<EventFuture>> {
        let d = self.get_mut(&debuggee).unwrap().inner.take().unwrap();
        Ok(self.push_child(
            EventFuture {
                inner: d,
                state: EventFutureState::Continue(Some(resumption)),
            },
            &debuggee,
        )?)
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
        let EventFuture { inner, state } = f;
        self.get_mut(&debuggee)?.inner = Some(inner);
        match state {
            EventFutureState::SingleStep(..) | EventFutureState::Continue(..) => {
                unreachable!("ready() cannot return until setting Done state")
            }
            EventFutureState::Done(Ok(result)) => Ok(result_to_event(self, result)?),
            EventFutureState::Done(Err(e)) => Err(e),
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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
        addr: u64,
        d: Resource<Debuggee>,
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

/// A provider of a [`ResourceTable`] for debugger host APIs.
pub trait DebuggerView: Send {
    /// Provide a mutable borrow of the underlying resource table.
    fn table(&mut self) -> &mut ResourceTable;
}

/// Add the debugger world's host functions to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    linker: &mut wasmtime::component::Linker<T>,
    f: fn(&mut T) -> &mut ResourceTable,
) -> wasmtime::Result<()> {
    wit::add_to_linker::<_, wasmtime::component::HasSelf<ResourceTable>>(linker, f)
}
