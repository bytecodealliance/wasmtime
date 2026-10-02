//! Automatic, store-local identities assigned during recorded construction.
//!
//! IDs are assigned by runtime object order, never by pointers or export names.
//! Pointers are used only for reverse lookup in the current process. In
//! particular an imported alias has the same ID as its underlying definition.

use super::*;

#[derive(Clone)]
pub(super) struct RecordedFunc {
    pub host: bool,
    pub params: Vec<Kind>,
    pub results: Vec<Kind>,
}

#[derive(Default)]
pub(super) struct Objects {
    pub funcs: Vec<RecordedFunc>,
    pub(super) functions_by_key: TryHashMap<(usize, usize), usize>,
    pub memories: Vec<Memory>,
    pub memories_by_key: TryHashMap<usize, usize>,
    pub globals: Vec<crate::Global>,
    pub globals_by_key: TryHashMap<usize, usize>,
    pub tables: Vec<crate::Table>,
    pub tables_by_key: TryHashMap<(u32, u32), usize>,
    pub modules: Vec<crate::Module>,
}

impl Objects {
    pub(super) fn encode_ref(&self, ptr: *mut core::ffi::c_void) -> Result<u32> {
        match NonNull::new(ptr.cast::<VMFuncRef>()) {
            None => Ok(0),
            Some(ptr) => Ok(u32::try_from(1 + self.find_func(ptr)?)?),
        }
    }

    pub(super) fn add_func(&mut self, store: &StoreOpaque, func: Func) -> Result<usize> {
        let raw = func.vm_func_ref(store);
        if let Some(id) = self.functions_by_key.get(&func_key(raw)) {
            return Ok(*id);
        }
        // SAFETY: the function is rooted in store, which owns its context.
        let magic = unsafe { raw.as_ref().vmctx.as_non_null().as_ref().magic };
        let host = match magic {
            wasmtime_environ::VMCONTEXT_MAGIC => false,
            wasmtime_environ::VM_ARRAY_CALL_HOST_FUNC_MAGIC => true,
            _ => bail!("unsupported function context in record/replay"),
        };
        let ty = func.load_ty(store);
        let params = ty.params().map(Kind::new).collect::<Result<Vec<_>>>()?;
        let results = ty.results().map(Kind::new).collect::<Result<Vec<_>>>()?;
        // Replay reconstructs host functions from their signatures.
        ensure!(
            !host
                || !params
                    .iter()
                    .chain(&results)
                    .any(|k| *k == Kind::Unsupported),
            "record/replay does not support GC or typed reference boundaries"
        );
        let id = self.funcs.len();
        self.functions_by_key.insert(func_key(raw), id)?;
        self.funcs.push(RecordedFunc {
            host,
            params,
            results,
        });
        Ok(id)
    }

    pub fn find_func(&self, func: NonNull<VMFuncRef>) -> Result<usize> {
        self.functions_by_key
            .get(&func_key(func))
            .copied()
            .ok_or_else(|| format_err!("unregistered function crossed the record/replay boundary"))
    }
}
