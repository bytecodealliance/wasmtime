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
    Imports, InstanceAllocationRequest, InstanceAllocator, StackMapRegistry,
    VMExternRefActivationsTable, VMFunctionBody, VMFunctionImport, VMSharedSignatureIndex,
};

pub(crate) fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    host_state: Box<dyn Any>,
    func_imports: &[VMFunctionImport],
    shared_signature_id: Option<VMSharedSignatureIndex>,
) -> Result<StoreInstanceHandle> {
    let mut imports = Imports::default();
    imports.functions = func_imports;
    let module = Arc::new(module);

    unsafe {
        // Use the default allocator when creating handles associated with host objects
        // The configured instance allocator should only be used when creating module instances
        // as we don't want host objects to count towards instance limits.
        let handle = store
            .engine()
            .config()
            .default_instance_allocator
            .allocate(InstanceAllocationRequest {
                module: module.clone(),
                finished_functions: &finished_functions,
                imports,
                shared_signatures: shared_signature_id.into(),
                host_state,
                interrupts: store.interrupts(),
                externref_activations_table: store.externref_activations_table()
                    as *const VMExternRefActivationsTable
                    as *mut _,
                stack_map_registry: store.stack_map_registry() as *const StackMapRegistry as *mut _,
            })?;

        Ok(store.add_instance(handle, true))
    }
}
