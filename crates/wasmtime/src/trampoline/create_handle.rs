//! Support for a calling of an imported function.

use crate::trampoline::StoreInstanceHandle;
use crate::Store;
use anyhow::Result;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::{
    Imports, InstanceHandle, StackMapRegistry, VMExternRefActivationsTable, VMFunctionBody,
    VMFunctionImport, VMSharedSignatureIndex, VMTrampoline,
};

pub(crate) fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
    state: Box<dyn Any>,
    func_imports: &[VMFunctionImport],
) -> Result<StoreInstanceHandle> {
    let mut imports = Imports::default();
    imports.functions = func_imports;

    // Compute indices into the shared signature table.
    let signatures = module
        .signatures
        .values()
        .map(|(wasm, native)| store.register_signature(wasm.clone(), native.clone()))
        .collect::<PrimaryMap<_, _>>();

    unsafe {
        let handle = InstanceHandle::new(
            Arc::new(module),
            Arc::new(()),
            &finished_functions,
            trampolines,
            imports,
            store.memory_creator(),
            signatures.into_boxed_slice(),
            state,
            store.interrupts(),
            store.externref_activations_table() as *const VMExternRefActivationsTable as *mut _,
            store.stack_map_registry() as *const StackMapRegistry as *mut _,
        )?;
        Ok(store.add_instance(handle))
    }
}
