//! Support for a calling of an imported function.

use crate::runtime::Store;
use anyhow::Result;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{
    Imports, InstanceHandle, VMFunctionBody, VMSharedSignatureIndex, VMTrampoline,
};

pub(crate) fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
    state: Box<dyn Any>,
) -> Result<InstanceHandle> {
    let imports = Imports::new(
        HashSet::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
    );
    let data_initializers = Vec::new();

    // Compute indices into the shared signature table.
    let signatures = module
        .local
        .signatures
        .values()
        .map(|sig| store.compiler().signatures().register(sig))
        .collect::<PrimaryMap<_, _>>();

    unsafe {
        Ok(InstanceHandle::new(
            Arc::new(module),
            store.compiler().trap_registry().register_traps(Vec::new()),
            finished_functions.into_boxed_slice(),
            trampolines,
            imports,
            store.memory_creator(),
            &data_initializers,
            signatures.into_boxed_slice(),
            None,
            store
                .engine()
                .config()
                .validating_config
                .operator_config
                .enable_bulk_memory,
            state,
        )?)
    }
}
