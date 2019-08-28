//! Support for a calling of an imported function.

use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
//use target_lexicon::HOST;
use failure::Error;
use wasmtime_environ::Module;
use wasmtime_runtime::{Imports, InstanceHandle, VMFunctionBody};

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub fn create_handle(
    module: Module,
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
    let signatures = PrimaryMap::new();

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
