//! Support for a calling of an imported function.

use crate::trampoline::StoreInstanceHandle;
use crate::Store;
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{
    Imports, InstanceHandle, StackMapRegistry, VMExternRefActivationsTable, VMFunctionBody,
    VMFunctionImport, VMSharedSignatureIndex,
};

pub(crate) fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    state: Box<dyn Any>,
    func_imports: &[VMFunctionImport],
    shared_signature_id: Option<VMSharedSignatureIndex>,
) -> Result<StoreInstanceHandle> {
    let mut imports = Imports::default();
    imports.functions = func_imports;
    let module = Arc::new(module);

    unsafe {
        let handle = InstanceHandle::new(
            module,
            &finished_functions,
            imports,
            store.memory_creator(),
            &|_| shared_signature_id.unwrap(),
            state,
            store.interrupts(),
            store.externref_activations_table() as *const VMExternRefActivationsTable as *mut _,
            store.stack_map_registry() as *const StackMapRegistry as *mut _,
        )?;
        Ok(store.add_instance(handle))
    }
}
