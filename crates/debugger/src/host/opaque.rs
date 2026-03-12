//! A type-erased trait wrapping the `Debugger<T>` to permit its use
//! within a resource.

use crate::host::wit;
use crate::host::{api::WasmValue, bindings::val_type_to_wasm_type};
use wasmtime::{
    Engine, ExnRef, ExnRefPre, ExnType, FrameHandle, Func, FuncType, Global, Instance, Memory,
    Module, OwnedRooted, Result, Table, Tag, TagType, Val, ValType,
};

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
pub(crate) trait OpaqueDebugger {
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
    async fn memory_read_bytes(
        &mut self,
        memory: Memory,
        addr: u64,
        len: u64,
    ) -> Result<Option<Vec<u8>>>;
    async fn memory_write_bytes(
        &mut self,
        memory: Memory,
        addr: u64,
        bytes: Vec<u8>,
    ) -> Result<Option<()>>;
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

#[async_trait::async_trait]
impl<T: Send + 'static> OpaqueDebugger for crate::Debuggee<T> {
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

    async fn memory_read_bytes(
        &mut self,
        memory: Memory,
        addr: u64,
        len: u64,
    ) -> Result<Option<Vec<u8>>> {
        self.with_store(move |store| {
            let data = memory.data(&store);
            let addr = usize::try_from(addr).unwrap();
            let len = usize::try_from(len).unwrap();
            data.get(addr..addr + len).map(|s| s.to_vec())
        })
        .await
    }

    async fn memory_write_bytes(
        &mut self,
        memory: Memory,
        addr: u64,
        bytes: Vec<u8>,
    ) -> Result<Option<()>> {
        self.with_store(move |mut store| {
            let data = memory.data_mut(&mut store);
            let addr = usize::try_from(addr).unwrap();
            let dest = data.get_mut(addr..addr + bytes.len())?;
            dest.copy_from_slice(&bytes);
            Some(())
        })
        .await
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
