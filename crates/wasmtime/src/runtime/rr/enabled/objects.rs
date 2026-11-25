//! Automatic, store-local identities assigned during recorded construction.
//!
//! IDs are assigned by runtime object order, never by pointers or export names.
//! Pointers are used only for reverse lookup in the current process. In
//! particular an imported alias has the same ID as its underlying definition.
//!
//! These are distinct from the store's own indices, such as `InstanceId`, which
//! depend on implementation details such as the dummy instances that own host
//! memories and tables, while IDs in a trace must mean the same to other
//! versions of Wasmtime.

use super::*;
use crate::runtime::externals::{GlobalHashKey, TableHashKey};
use crate::runtime::memory::MemoryHashKey;
use crate::runtime::vm::VMFuncRefHashKey;
use wasmtime_environ::collections::TryPrimaryMap;

macro_rules! define_ids {
    ($($(#[$attr:meta])* $name:ident;)*) => {
        $(
            $(#[$attr])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub(super) struct $name(u32);
            wasmtime_environ::entity_impl!($name);
        )*
    };
}

define_ids! {
    /// The ID of a function in a trace.
    RecordedFunc;
    /// The ID of a memory in a trace.
    RecordedMemory;
    /// The ID of a global in a trace.
    RecordedGlobal;
    /// The ID of a table in a trace.
    RecordedTable;
    /// The ID of a module in a trace.
    RecordedModule;
    /// The ID of a call across the host/Wasm boundary in a trace.
    RecordedCall;
}

pub(super) struct FuncInfo {
    pub host: bool,
    pub params: TryVec<Kind>,
    pub results: TryVec<Kind>,
}

#[derive(Default)]
pub(super) struct Objects {
    pub funcs: TryPrimaryMap<RecordedFunc, FuncInfo>,
    funcs_by_key: TryHashMap<VMFuncRefHashKey, RecordedFunc>,
    pub memories: TryPrimaryMap<RecordedMemory, Memory>,
    memories_by_key: TryHashMap<MemoryHashKey, RecordedMemory>,
    globals: TryPrimaryMap<RecordedGlobal, crate::Global>,
    globals_by_key: TryHashMap<GlobalHashKey, RecordedGlobal>,
    tables: TryPrimaryMap<RecordedTable, crate::Table>,
    tables_by_key: TryHashMap<TableHashKey, RecordedTable>,
    pub modules: TryPrimaryMap<RecordedModule, crate::Module>,
}

impl Objects {
    /// Encodes a function reference: zero if it is null, and otherwise one more
    /// than the function's ID.
    ///
    /// # Safety
    ///
    /// `ptr` must be null or a valid pointer to a `VMFuncRef`.
    pub unsafe fn encode_ref(&self, ptr: *mut core::ffi::c_void) -> Result<u32> {
        let Some(ptr) = NonNull::new(ptr.cast::<VMFuncRef>()) else {
            return Ok(0);
        };
        // SAFETY: inherited from this function's contract.
        let func = unsafe { ptr.as_ref() };
        // Cannot overflow: `u32::MAX` is not a valid ID.
        Ok(1 + self.find_func_ref(func)?.as_u32())
    }

    pub fn add_func(&mut self, store: &StoreOpaque, func: Func) -> Result<RecordedFunc> {
        if let Ok(id) = self.find_func(store, func) {
            return Ok(id);
        }
        // SAFETY: the store owns the function and its context.
        let (key, magic) = unsafe {
            let raw = func.vm_func_ref(store);
            let raw = raw.as_ref();
            (raw.hash_key(), raw.vmctx.as_non_null().as_ref().magic)
        };
        let host = match magic {
            wasmtime_environ::VMCONTEXT_MAGIC => false,
            wasmtime_environ::VM_ARRAY_CALL_HOST_FUNC_MAGIC => true,
            _ => bail!("unsupported function context in record/replay"),
        };
        let ty = func.load_ty(store);
        let mut params = TryVec::with_capacity(ty.params().len())?;
        for param in ty.params() {
            params.push(Kind::new(&param))?;
        }
        let mut results = TryVec::with_capacity(ty.results().len())?;
        for result in ty.results() {
            results.push(Kind::new(&result))?;
        }
        // Replay reconstructs host functions from their signatures.
        if host
            && params
                .iter()
                .chain(&results)
                .any(|k| *k == Kind::Unsupported)
        {
            bail!(codec::UNSUPPORTED);
        }
        let id = self.funcs.push(FuncInfo {
            host,
            params,
            results,
        })?;
        self.funcs_by_key.insert(key, id)?;
        Ok(id)
    }

    pub fn find_func(&self, store: &StoreOpaque, func: Func) -> Result<RecordedFunc> {
        // SAFETY: the store owns the function.
        self.find_func_ref(unsafe { func.vm_func_ref(store).as_ref() })
    }

    pub fn find_func_ref(&self, func: &VMFuncRef) -> Result<RecordedFunc> {
        self.funcs_by_key
            .get(&func.hash_key())
            .copied()
            .ok_or_else(|| format_err!("unregistered function crossed the record/replay boundary"))
    }

    pub fn add_memory(&mut self, store: &StoreOpaque, memory: Memory) -> Result<RecordedMemory> {
        let id = self.memories.push(memory)?;
        self.memories_by_key.insert(memory.hash_key(store), id)?;
        Ok(id)
    }

    pub fn find_memory(&self, store: &StoreOpaque, memory: Memory) -> Option<RecordedMemory> {
        self.memories_by_key.get(&memory.hash_key(store)).copied()
    }

    pub fn add_global(
        &mut self,
        store: &StoreOpaque,
        global: crate::Global,
    ) -> Result<RecordedGlobal> {
        let id = self.globals.push(global)?;
        self.globals_by_key.insert(global.hash_key(store), id)?;
        Ok(id)
    }

    pub fn find_global(
        &self,
        store: &StoreOpaque,
        global: crate::Global,
    ) -> Option<RecordedGlobal> {
        self.globals_by_key.get(&global.hash_key(store)).copied()
    }

    pub fn add_table(&mut self, store: &StoreOpaque, table: crate::Table) -> Result<RecordedTable> {
        let id = self.tables.push(table)?;
        self.tables_by_key.insert(table.hash_key(store), id)?;
        Ok(id)
    }

    pub fn find_table(&self, store: &StoreOpaque, table: crate::Table) -> Option<RecordedTable> {
        self.tables_by_key.get(&table.hash_key(store)).copied()
    }
}
