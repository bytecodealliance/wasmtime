//! Support for a calling of an imported function.

use crate::runtime::Store;
use anyhow::Result;
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{Imports, InstanceHandle, VMFunctionBody};

pub(crate) fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
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
        .signatures
        .values()
        .map(|sig| store.compiler().signatures().register(sig))
        .collect::<PrimaryMap<_, _>>();

    unsafe {
        Ok(InstanceHandle::new(
            Arc::new(module),
            store.compiler().trap_registry().register_traps(Vec::new()),
            finished_functions.into_boxed_slice(),
            imports,
            &data_initializers,
            signatures.into_boxed_slice(),
            None,
            state,
        )?)
    }
}
