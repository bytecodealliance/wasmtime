//! Support for a calling of an imported function.

use crate::runtime::Store;
use anyhow::Result;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{Imports, InstanceHandle, VMFunctionBody};

pub(crate) fn create_handle(
    module: Module,
    signature_registry: Option<&Store>,
    finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody>,
    state: Box<dyn Any>,
) -> Result<InstanceHandle> {
    let global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>> =
        Rc::new(RefCell::new(HashMap::new()));

    let imports = Imports::new(
        HashSet::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
        PrimaryMap::new(),
    );
    let data_initializers = Vec::new();

    // Compute indices into the shared signature table.
    let signatures = signature_registry
        .map(|signature_registry| {
            module
                .signatures
                .values()
                .map(|sig| signature_registry.register_wasmtime_signature(sig))
                .collect::<PrimaryMap<_, _>>()
        })
        .unwrap_or_else(PrimaryMap::new);

    Ok(InstanceHandle::new(
        Rc::new(module),
        global_exports,
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        state,
    )
    .expect("instance"))
}
