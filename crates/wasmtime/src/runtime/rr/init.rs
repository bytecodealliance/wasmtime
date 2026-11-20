//! Core object construction. Modules are recorded as the checksums of their
//! Wasm bytecode, and host functions as their signatures.

use super::*;
use crate::{Global, Instance, Module};
use wasmtime_environ::packed_option::ReservedValue;
use wasmtime_environ::{DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, EntityIndex};

impl Session {
    fn import_func(&mut self, store: &StoreOpaque, func: Func) -> Result<usize> {
        if let Ok(id) = self.objects.find_func(func.vm_func_ref(store)) {
            return Ok(id);
        }
        let id = self.objects.add_func(store, func)?;
        let f = &self.objects.funcs[id];
        ensure!(f.host, "unregistered guest function in instance imports");
        let mut body = u32::try_from(id)?.to_le_bytes().to_vec();
        codec::blob(
            &mut body,
            &f.params.iter().map(|k| *k as u8).collect::<Vec<_>>(),
        )?;
        codec::blob(
            &mut body,
            &f.results.iter().map(|k| *k as u8).collect::<Vec<_>>(),
        )?;
        self.append(codec::HOST, &body)?;
        Ok(id)
    }

    fn import_global(&mut self, store: &mut StoreOpaque, global: Global) -> Result<usize> {
        if let Some(id) = self.objects.globals_by_key.get(&global.rr_key(store)) {
            return Ok(*id);
        }
        let ty = global._ty(store);
        let value = global._get(store);
        let id = self.objects.globals.len();
        let mut body = u32::try_from(id)?.to_le_bytes().to_vec();
        let mutable = u8::from(ty.mutability() == crate::Mutability::Var);
        if let crate::ValType::Ref(reference) = ty.content() {
            ensure!(
                reference.heap_type().is_func(),
                "record/replay requires an abstract function global"
            );
            let crate::Val::FuncRef(func) = value else {
                unreachable!()
            };
            let func = match func {
                None => 0,
                Some(func) => 1 + self.import_func(store, func)?,
            };
            body.extend_from_slice(&[codec::FUNCREF, mutable, u8::from(reference.is_nullable())]);
            body.extend_from_slice(&u32::try_from(func)?.to_le_bytes());
        } else {
            let kind = Kind::new(ty.content().clone())?;
            let raw = match value {
                crate::Val::I32(v) => ValRaw::i32(v),
                crate::Val::I64(v) => ValRaw::i64(v),
                crate::Val::F32(v) => ValRaw::f32(v),
                crate::Val::F64(v) => ValRaw::f64(v),
                crate::Val::V128(v) => ValRaw::v128(v.as_u128()),
                _ => unreachable!(),
            };
            body.extend_from_slice(&[kind as u8, mutable]);
            codec::reserve(&mut body, kind.size())?;
            // SAFETY: kind and raw both came from the global's checked type.
            unsafe {
                codec::values(&mut body, &[kind], &raw, |_| unreachable!())?;
            }
        }
        self.append(codec::GLOBAL, &body)?;
        self.objects
            .globals_by_key
            .insert(global.rr_key(store), id)?;
        self.objects.globals.push(global);
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
            .position(|m| Module::same(m, module))
        {
            Some(id) => id,
            None => {
                let id = self.objects.modules.len();
                let mut body = u32::try_from(id)?.to_le_bytes().to_vec();
                body.extend_from_slice(&**module.checksum());
                self.append(codec::MODULE, &body)?;
                self.objects.modules.push(module.clone());
                id
            }
        };
        let mut body = u32::try_from(module_id)?.to_le_bytes().to_vec();
        for initializer in &module.env_module().initializers {
            let wasmtime_environ::Initializer::Import { index: ty, .. } = *initializer;
            let object = match ty {
                EntityIndex::Function(index) => {
                    let store_id = store.id();
                    let (inst, registry) = instance.id.get_mut_and_module_registry(store);
                    // SAFETY: the new instance and its imports belong to store.
                    let func = unsafe { inst.get_exported_func(registry, store_id, index) };
                    self.import_func(store, func)?
                }
                EntityIndex::Memory(index) => {
                    let memory = store[instance.id]
                        .get_exported_memory(store.id(), index)
                        .unshared()
                        .ok_or_else(|| format_err!("shared memory import in recording"))?;
                    *self
                        .objects
                        .memories_by_key
                        .get(&memory.rr_key(store))
                        .ok_or_else(|| format_err!("unregistered imported memory"))?
                }
                EntityIndex::Table(index) => {
                    let table = store[instance.id].get_exported_table(store.id(), index);
                    *self
                        .objects
                        .tables_by_key
                        .get(&table.rr_key())
                        .ok_or_else(|| format_err!("unregistered imported table"))?
                }
                EntityIndex::Global(index) => {
                    let global = store[instance.id].get_exported_global(store.id(), index);
                    self.import_global(store, global)?
                }
                _ => bail!("unsupported record/replay import"),
            };
            body.extend_from_slice(&u32::try_from(object)?.to_le_bytes());
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
    pub(super) fn rr_register_func_reference(&mut self, raw: NonNull<VMFuncRef>) -> Result<()> {
        if self
            .rr
            .session
            .as_ref()
            .unwrap()
            .objects
            .find_func(raw)
            .is_ok()
        {
            return Ok(());
        }
        self.rr_register(|store, session| {
            // SAFETY: the boundary's typed slot contains a store-rooted reference.
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
            let id = session.objects.memories.len();
            let mut body = u32::try_from(id)?.to_le_bytes().to_vec();
            body.extend_from_slice(&[
                u8::from(ty.idx_type == wasmtime_environ::IndexType::I64),
                ty.page_size_log2,
            ]);
            body.extend_from_slice(&ty.limits.min.to_le_bytes());
            body.push(u8::from(ty.limits.max.is_some()));
            body.extend_from_slice(&ty.limits.max.unwrap_or_default().to_le_bytes());
            session.append(codec::MEMORY, &body)?;
            session
                .objects
                .memories_by_key
                .insert(memory.rr_key(store), id)?;
            session.objects.memories.push(memory);
            Ok(())
        })
    }

    pub(crate) fn rr_created_table(&mut self, table: crate::Table, init: crate::Ref) -> Result<()> {
        self.rr_register(|store, session| {
            let ty = table.ty_(store);
            ensure!(
                ty.element().heap_type().is_func(),
                "record/replay requires a function table"
            );
            let func = match init {
                crate::Ref::Func(None) => 0,
                crate::Ref::Func(Some(func)) => 1 + session.import_func(store, func)?,
                _ => bail!("unsupported initial table element"),
            };
            let id = session.objects.tables.len();
            let mut body = u32::try_from(id)?.to_le_bytes().to_vec();
            body.extend_from_slice(&[u8::from(ty.is_64()), u8::from(ty.element().is_nullable())]);
            body.extend_from_slice(&ty.minimum().to_le_bytes());
            body.push(u8::from(ty.maximum().is_some()));
            body.extend_from_slice(&ty.maximum().unwrap_or_default().to_le_bytes());
            body.extend_from_slice(&u32::try_from(func)?.to_le_bytes());
            session.append(codec::TABLE, &body)?;
            session.objects.tables_by_key.insert(table.rr_key(), id)?;
            session.objects.tables.push(table);
            Ok(())
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
        let functions = store[id]
            .env_module()
            .functions
            .iter()
            .filter(|(i, f)| {
                !store[id].env_module().is_imported_function(*i) && !f.func_ref.is_reserved_value()
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
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
            self.memories_by_key
                .insert(memory.rr_key(store), self.memories.len())?;
            self.memories.push(memory);
        }
        for i in 0..store[id].env_module().num_defined_tables() {
            let table = crate::Table::from_raw(id, DefinedTableIndex::from_u32(u32::try_from(i)?));
            self.tables_by_key
                .insert(table.rr_key(), self.tables.len())?;
            self.tables.push(table);
        }
        for i in 0..store[id].env_module().num_defined_globals() {
            let global = Global::new_instance(
                store,
                id.instance(),
                DefinedGlobalIndex::from_u32(u32::try_from(i)?),
            );
            self.globals_by_key
                .insert(global.rr_key(store), self.globals.len())?;
            self.globals.push(global);
        }
        Ok(())
    }
}
