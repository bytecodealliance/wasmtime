use crate::component::instance::lookup;
use crate::store::{StoreOpaque, Stored};
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, FuncTypeIndex, LiftedFunction, RuntimeInstanceIndex, StringEncoding,
};
use wasmtime_environ::PrimaryMap;
use wasmtime_runtime::{Export, ExportFunction, ExportMemory, VMTrampoline};

/// A WebAssembly component function.
//
// FIXME: write more docs here
#[derive(Copy, Clone, Debug)]
pub struct Func(Stored<FuncData>);

#[doc(hidden)]
#[allow(dead_code)] // FIXME: remove this when fields are actually used
pub struct FuncData {
    trampoline: VMTrampoline,
    export: ExportFunction,
    ty: FuncTypeIndex,
    types: Arc<ComponentTypes>,
    options: Options,
}

#[derive(Clone)]
#[allow(dead_code)] // FIXME: remove this when fields are actually used
pub(crate) struct Options {
    string_encoding: Option<StringEncoding>,
    intrinsics: Option<Intrinsics>,
}

#[derive(Clone)]
#[allow(dead_code)] // FIXME: remove this when fields are actually used
struct Intrinsics {
    memory: ExportMemory,
    realloc: ExportFunction,
    free: ExportFunction,
}

impl Func {
    pub(crate) fn from_lifted_func(
        store: &mut StoreOpaque,
        types: &Arc<ComponentTypes>,
        instances: &PrimaryMap<RuntimeInstanceIndex, crate::Instance>,
        func: &LiftedFunction,
    ) -> Func {
        let export = match lookup(store, instances, &func.func) {
            Export::Function(f) => f,
            _ => unreachable!(),
        };
        let trampoline = store.lookup_trampoline(unsafe { export.anyfunc.as_ref() });
        let intrinsics = func.options.intrinsics.as_ref().map(|i| {
            let memory = match lookup(store, instances, &i.memory) {
                Export::Memory(m) => m,
                _ => unreachable!(),
            };
            let realloc = match lookup(store, instances, &i.canonical_abi_realloc) {
                Export::Function(f) => f,
                _ => unreachable!(),
            };
            let free = match lookup(store, instances, &i.canonical_abi_free) {
                Export::Function(f) => f,
                _ => unreachable!(),
            };
            Intrinsics {
                memory,
                realloc,
                free,
            }
        });
        Func(store.store_data_mut().insert(FuncData {
            trampoline,
            export,
            options: Options {
                intrinsics,
                string_encoding: func.options.string_encoding,
            },
            ty: func.ty,
            types: types.clone(),
        }))
    }
}
