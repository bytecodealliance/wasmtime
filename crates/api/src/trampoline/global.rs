use super::create_handle::create_handle;
use crate::Store;
use crate::{GlobalType, Mutability, Val};
use anyhow::{bail, Result};
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};
use wasmtime_runtime::InstanceHandle;

pub fn create_global(store: &Store, gt: &GlobalType, val: Val) -> Result<InstanceHandle> {
    let global = wasm::Global {
        ty: match gt.content().get_wasmtime_type() {
            Some(t) => t,
            None => bail!("cannot support {:?} as a wasm global type", gt.content()),
        },
        mutability: match gt.mutability() {
            Mutability::Const => false,
            Mutability::Var => true,
        },
        initializer: match val {
            Val::I32(i) => wasm::GlobalInit::I32Const(i),
            Val::I64(i) => wasm::GlobalInit::I64Const(i),
            Val::F32(f) => wasm::GlobalInit::F32Const(f),
            Val::F64(f) => wasm::GlobalInit::F64Const(f),
            _ => unimplemented!("create_global for {:?}", gt),
        },
    };
    let mut module = Module::new();
    let global_id = module.local.globals.push(global);
    module.exports.insert(
        "global".to_string(),
        wasmtime_environ::Export::Global(global_id),
    );
    let handle = create_handle(
        module,
        store,
        PrimaryMap::new(),
        Default::default(),
        Box::new(()),
    )?;
    Ok(handle)
}
