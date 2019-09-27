//! Support for a calling of an imported function.

use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
//use target_lexicon::HOST;
use failure::Error;
use wasmtime_environ::Module;
use wasmtime_runtime::{Imports, InstanceHandle, VMFunctionBody};

use crate::{HashMap, HashSet};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;
use core::cell::{RefCell, RefMut};

use crate::runtime::Store;

pub(crate) fn create_handle(
    module: Module,
    signature_registry: Option<RefMut<Store>>,
    finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody>,
    state: Box<dyn Any>,
) -> Result<InstanceHandle, Error> {
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
        .and_then(|mut signature_registry| {
            Some(
                module
                    .signatures
                    .values()
                    .map(|sig| signature_registry.register_cranelift_signature(sig))
                    .collect::<PrimaryMap<_, _>>(),
            )
        })
        .unwrap_or_else(|| PrimaryMap::new());

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
