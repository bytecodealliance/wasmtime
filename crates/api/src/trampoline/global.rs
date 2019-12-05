use super::create_handle::create_handle;
use crate::{GlobalType, Mutability, Val};
use anyhow::Result;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};
use wasmtime_runtime::{InstanceHandle, VMGlobalDefinition};

#[allow(dead_code)]
pub struct GlobalState {
    definition: Box<VMGlobalDefinition>,
    handle: InstanceHandle,
}

pub fn create_global(gt: &GlobalType, val: Val) -> Result<(wasmtime_runtime::Export, GlobalState)> {
    let mut definition = Box::new(VMGlobalDefinition::new());
    unsafe {
        match val {
            Val::I32(i) => *definition.as_i32_mut() = i,
            Val::I64(i) => *definition.as_i64_mut() = i,
            Val::F32(f) => *definition.as_u32_mut() = f,
            Val::F64(f) => *definition.as_u64_mut() = f,
            _ => unimplemented!("create_global for {:?}", gt),
        }
    }

    let global = wasm::Global {
        ty: gt.content().get_wasmtime_type(),
        mutability: match gt.mutability() {
            Mutability::Const => false,
            Mutability::Var => true,
        },
        initializer: wasm::GlobalInit::Import, // TODO is it right?
    };
    let mut handle =
        create_handle(Module::new(), None, PrimaryMap::new(), Box::new(())).expect("handle");
    Ok((
        wasmtime_runtime::Export::Global {
            definition: definition.as_mut(),
            vmctx: handle.vmctx_mut_ptr(),
            global,
        },
        GlobalState { definition, handle },
    ))
}
