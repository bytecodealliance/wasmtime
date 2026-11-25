//! Core object construction. Modules are recorded as the checksums of their
//! Wasm bytecode, and host functions as their signatures.

use super::*;
use crate::{Global, Instance, Module};
use objects::RecordedGlobal;
use wasmtime_environ::packed_option::ReservedValue;
use wasmtime_environ::{DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, EntityIndex};

impl Session {
    fn import_func(&mut self, store: &StoreOpaque, func: Func) -> Result<RecordedFunc> {
        if let Ok(id) = self.objects.find_func(store, func) {
            return Ok(id);
        }
        let id = self.objects.add_func(store, func)?;
        let f = &self.objects.funcs[id];
        ensure!(f.host, "unregistered guest function in instance imports");
        let mut body = TryVec::new();
        body.extend_from_slice(&id.as_u32().to_le_bytes())?;
        for kinds in [&f.params, &f.results] {
            let mut encoded = TryVec::new();
            for kind in kinds {
                kind.encode(&mut encoded)?;
            }
            codec::blob(&mut body, &encoded)?;
        }
        self.append(codec::HOST, &body)?;
        Ok(id)
    }

    fn import_global(&mut self, store: &mut StoreOpaque, global: Global) -> Result<RecordedGlobal> {
        if let Some(id) = self.objects.find_global(store, global) {
            return Ok(id);
        }
        let ty = global._ty(store);
        let kind = Kind::new(ty.content());
        ensure!(
            kind != Kind::Unsupported,
            "record/replay does not support globals of GC or typed references"
        );
        let raw = match global._get(store) {
            crate::Val::I32(v) => ValRaw::i32(v),
            crate::Val::I64(v) => ValRaw::i64(v),
            crate::Val::F32(v) => ValRaw::f32(v),
            crate::Val::F64(v) => ValRaw::f64(v),
            crate::Val::V128(v) => ValRaw::v128(v.as_u128()),
            crate::Val::FuncRef(None) => ValRaw::funcref(core::ptr::null_mut()),
            crate::Val::FuncRef(Some(func)) => {
                self.import_func(store, func)?;
                ValRaw::funcref(func.vm_func_ref(store).as_ptr().cast())
            }
            _ => unreachable!(),
        };
        let id = self.objects.add_global(store, global)?;
        let mut body = TryVec::new();
        body.extend_from_slice(&id.as_u32().to_le_bytes())?;
        kind.encode(&mut body)?;
        body.push(u8::from(ty.mutability() == crate::Mutability::Var))?;
        // SAFETY: kind and raw both came from the global's checked type, and
        // the store owns any function that raw refers to.
        unsafe {
            codec::values(&mut body, &[kind], &raw, |ptr| self.objects.encode_ref(ptr))?;
        }
        self.append(codec::GLOBAL, &body)?;
        Ok(id)
    }

    /// Records the construction of `instance` from `module`: the checksum of
    /// the module's bytecode, the first time it is instantiated, and the
    /// instance's imports.
    fn record_instance(
        &mut self,
        store: &mut StoreOpaque,
        instance: Instance,
        module: &Module,
    ) -> Result<()> {
        let module_id = match self
            .objects
            .modules
            .iter()
            .find(|(_, m)| Module::same(m, module))
        {
            Some((id, _)) => id,
            None => {
                let id = self.objects.modules.push(module.clone())?;
                let mut body = TryVec::new();
                body.extend_from_slice(&id.as_u32().to_le_bytes())?;
                body.extend_from_slice(&**module.checksum())?;
                self.append(codec::MODULE, &body)?;
                id
            }
        };
        let mut body = TryVec::new();
        body.extend_from_slice(&module_id.as_u32().to_le_bytes())?;
        for initializer in &module.env_module().initializers {
            let wasmtime_environ::Initializer::Import { index: ty, .. } = *initializer;
            let object = match ty {
                EntityIndex::Function(index) => {
                    let store_id = store.id();
                    let (inst, registry) = instance.id.get_mut_and_module_registry(store);
                    // SAFETY: the new instance and its imports belong to store.
                    let func = unsafe { inst.get_exported_func(registry, store_id, index) };
                    self.import_func(store, func)?.as_u32()
                }
                EntityIndex::Memory(index) => {
                    let memory = store[instance.id]
                        .get_exported_memory(store.id(), index)
                        .unshared()
                        .ok_or_else(|| format_err!("shared memory import in recording"))?;
                    self.objects
                        .find_memory(store, memory)
                        .ok_or_else(|| format_err!("unregistered imported memory"))?
                        .as_u32()
                }
                EntityIndex::Table(index) => {
                    let table = store[instance.id].get_exported_table(store.id(), index);
                    self.objects
                        .find_table(store, table)
                        .ok_or_else(|| format_err!("unregistered imported table"))?
                        .as_u32()
                }
                EntityIndex::Global(index) => {
                    let global = store[instance.id].get_exported_global(store.id(), index);
                    self.import_global(store, global)?.as_u32()
                }
                _ => bail!("unsupported record/replay import"),
            };
            body.extend_from_slice(&object.to_le_bytes())?;
        }
        self.append(codec::INSTANCE, &body)
    }
}

impl StoreOpaque {
    // Detach metadata while enumerating runtime objects; no guest code or
    // embedder callbacks run in these closures. Always restore it on error.
    fn rr_register(&mut self, f: impl FnOnce(&mut Self, &mut Session) -> Result<()>) -> Result<()> {
        if !self.rr.active() {
            return Ok(());
        }
        self.rr_flush()?;
        let mut session = self.rr.session.take().unwrap();
        let result = f(self, &mut session);
        if result.is_err() {
            session.fail(format_err!("failed to record object construction"));
        }
        self.rr.session = Some(session);
        result
    }

    /// Register a reference supplied by the host before recording its value.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid function reference that this store owns.
    pub(super) unsafe fn rr_register_func_reference(
        &mut self,
        raw: NonNull<VMFuncRef>,
    ) -> Result<()> {
        // SAFETY: inherited from this function's contract.
        let func_ref = unsafe { raw.as_ref() };
        if self.rr_session().objects.find_func_ref(func_ref).is_ok() {
            return Ok(());
        }
        self.rr_register(|store, session| {
            // SAFETY: inherited from this function's contract.
            let func = unsafe { Func::from_vm_func_ref(store.id(), raw) };
            session.import_func(store, func)?;
            Ok(())
        })
    }

    pub(crate) fn rr_created_global(&mut self, global: Global) -> Result<()> {
        self.rr_register(|store, session| {
            session.import_global(store, global)?;
            Ok(())
        })
    }

    pub(crate) fn rr_created_memory(&mut self, memory: Memory) -> Result<()> {
        self.rr_register(|store, session| {
            let ty = memory.wasmtime_ty(store);
            let id = session.objects.add_memory(store, memory)?;
            let mut body = TryVec::new();
            body.extend_from_slice(&id.as_u32().to_le_bytes())?;
            body.push(u8::from(ty.idx_type == wasmtime_environ::IndexType::I64))?;
            body.push(ty.page_size_log2)?;
            body.extend_from_slice(&ty.limits.min.to_le_bytes())?;
            body.push(u8::from(ty.limits.max.is_some()))?;
            body.extend_from_slice(&ty.limits.max.unwrap_or_default().to_le_bytes())?;
            session.append(codec::MEMORY, &body)
        })
    }

    pub(crate) fn rr_created_table(&mut self, table: crate::Table, init: crate::Ref) -> Result<()> {
        self.rr_register(|store, session| {
            let ty = table.ty_(store);
            let kind = Kind::new(&crate::ValType::Ref(ty.element().clone()));
            ensure!(
                matches!(kind, Kind::FuncRef { .. }),
                "record/replay requires a function table"
            );
            let func = match init {
                crate::Ref::Func(None) => 0,
                crate::Ref::Func(Some(func)) => 1 + session.import_func(store, func)?.as_u32(),
                _ => bail!("unsupported initial table element"),
            };
            let id = session.objects.add_table(store, table)?;
            let mut body = TryVec::new();
            body.extend_from_slice(&id.as_u32().to_le_bytes())?;
            kind.encode(&mut body)?;
            body.push(u8::from(ty.is_64()))?;
            body.extend_from_slice(&ty.minimum().to_le_bytes())?;
            body.push(u8::from(ty.maximum().is_some()))?;
            body.extend_from_slice(&ty.maximum().unwrap_or_default().to_le_bytes())?;
            body.extend_from_slice(&func.to_le_bytes())?;
            session.append(codec::TABLE, &body)
        })
    }

    pub(crate) fn rr_created_instance(
        &mut self,
        instance: Instance,
        module: &Module,
    ) -> Result<()> {
        self.rr_register(|store, session| {
            session.record_instance(store, instance, module)?;
            session.objects.register_instance(store, instance)?;
            Ok(())
        })
    }
}

impl Objects {
    fn register_instance(&mut self, store: &mut StoreOpaque, instance: Instance) -> Result<()> {
        let id = instance.id;
        let mut functions = TryVec::new();
        for (index, _) in store[id].env_module().functions.iter().filter(|(i, f)| {
            !store[id].env_module().is_imported_function(*i) && !f.func_ref.is_reserved_value()
        }) {
            functions.push(index)?;
        }
        for index in functions {
            let store_id = store.id();
            let (inst, registry) = id.get_mut_and_module_registry(store);
            // SAFETY: this instance belongs to store.
            let func = unsafe { inst.get_exported_func(registry, store_id, index) };
            self.add_func(store, func)?;
        }
        let store_id = store.id();
        let (inst, registry) = id.get_mut_and_module_registry(store);
        // SAFETY: this instance belongs to store.
        if let Some(startup) = unsafe { inst.get_startup_func(registry, store_id) } {
            self.add_func(store, startup)?;
        }
        for i in 0..store[id].env_module().num_defined_memories() {
            // SAFETY: shared memories have been rejected before allocation.
            let memory =
                unsafe { Memory::from_raw(id, DefinedMemoryIndex::from_u32(u32::try_from(i)?)) };
            self.add_memory(store, memory)?;
        }
        for i in 0..store[id].env_module().num_defined_tables() {
            let table = crate::Table::from_raw(id, DefinedTableIndex::from_u32(u32::try_from(i)?));
            self.add_table(store, table)?;
        }
        for i in 0..store[id].env_module().num_defined_globals() {
            let global = Global::new_instance(
                store,
                id.instance(),
                DefinedGlobalIndex::from_u32(u32::try_from(i)?),
            );
            self.add_global(store, global)?;
        }
        Ok(())
    }
}
