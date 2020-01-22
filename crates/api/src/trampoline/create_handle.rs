//! Support for a calling of an imported function.

use crate::runtime::Store;
use anyhow::Result;
use std::any::Any;
use std::collections::HashSet;
use std::rc::Rc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{Imports, InstanceHandle, VMFunctionBody};

pub(crate) fn create_handle(
    module: Module,
    store: Option<&Store>,
    finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody>,
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
    let signatures = store
        .map(|store| {
            module
                .signatures
                .values()
                .map(|sig| store.compiler().signatures().register(sig))
                .collect::<PrimaryMap<_, _>>()
        })
        .unwrap_or_else(PrimaryMap::new);

    Ok(InstanceHandle::new(
        Rc::new(module),
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        state,
    )
    .expect("instance"))
}
