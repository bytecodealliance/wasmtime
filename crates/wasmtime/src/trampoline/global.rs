use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::{GlobalType, Mutability, Val};
use anyhow::Result;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{
    wasm::{self, SignatureIndex},
    Module, ModuleType,
};
use wasmtime_runtime::VMFunctionImport;

pub fn create_global(store: &mut StoreOpaque<'_>, gt: &GlobalType, val: Val) -> Result<InstanceId> {
    let mut module = Module::new();
    let mut func_imports = Vec::new();
    let mut externref_init = None;
    let mut shared_signature_id = None;

    let global = wasm::Global {
        wasm_ty: gt.content().to_wasm_type(),
        mutability: match gt.mutability() {
            Mutability::Const => false,
            Mutability::Var => true,
        },
        initializer: match val {
            Val::I32(i) => wasm::GlobalInit::I32Const(i),
            Val::I64(i) => wasm::GlobalInit::I64Const(i),
            Val::F32(f) => wasm::GlobalInit::F32Const(f),
            Val::F64(f) => wasm::GlobalInit::F64Const(f),
            Val::V128(i) => wasm::GlobalInit::V128Const(i.into()),
            Val::ExternRef(None) | Val::FuncRef(None) => wasm::GlobalInit::RefNullConst,
            Val::ExternRef(Some(x)) => {
                // There is no `GlobalInit` variant for using an existing
                // `externref` that isn't an import (because Wasm can't create
                // an `externref` by itself). Therefore, initialize the global
                // as null, and then monkey patch it after instantiation below.
                externref_init = Some(x);
                wasm::GlobalInit::RefNullConst
            }
            Val::FuncRef(Some(f)) => {
                // Add a function import to the stub module, and then initialize
                // our global with a `ref.func` to grab that imported function.
                let f = f.caller_checked_anyfunc(store);
                let f = unsafe { f.as_ref() };
                shared_signature_id = Some(f.type_index);
                let sig_id = SignatureIndex::from_u32(u32::max_value() - 1);
                module.types.push(ModuleType::Function(sig_id));
                let func_index = module.functions.push(sig_id);
                module.num_imported_funcs = 1;
                module
                    .initializers
                    .push(wasmtime_environ::Initializer::Import {
                        name: "".into(),
                        field: None,
                        index: wasm::EntityIndex::Function(func_index),
                    });

                func_imports.push(VMFunctionImport {
                    body: f.func_ptr,
                    vmctx: f.vmctx,
                });

                wasm::GlobalInit::RefFunc(func_index)
            }
        },
    };

    let global_id = module.globals.push(global);
    module
        .exports
        .insert(String::new(), wasm::EntityIndex::Global(global_id));
    let id = create_handle(
        module,
        store,
        PrimaryMap::new(),
        Box::new(()),
        &func_imports,
        shared_signature_id,
    )?;

    if let Some(x) = externref_init {
        let instance = store.instance(id);
        match instance.lookup_by_declaration(&wasm::EntityIndex::Global(global_id)) {
            wasmtime_runtime::Export::Global(g) => unsafe {
                *(*g.definition).as_externref_mut() = Some(x.inner);
            },
            _ => unreachable!(),
        }
    }

    Ok(id)
}
