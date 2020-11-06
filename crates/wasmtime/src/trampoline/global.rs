use super::create_handle::create_handle;
use crate::trampoline::StoreInstanceHandle;
use crate::{GlobalType, Mutability, Store, Val};
use anyhow::Result;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};
use wasmtime_runtime::VMFunctionImport;

pub fn create_global(store: &Store, gt: &GlobalType, val: Val) -> Result<StoreInstanceHandle> {
    let mut module = Module::new();
    let mut func_imports = Vec::new();
    let mut externref_init = None;

    let global = wasm::Global {
        wasm_ty: gt.content().to_wasm_type(),
        ty: gt.content().get_wasmtime_type(),
        mutability: match gt.mutability() {
            Mutability::Const => false,
            Mutability::Var => true,
        },
        initializer: match val {
            Val::I32(i) => wasm::GlobalInit::I32Const(i),
            Val::I64(i) => wasm::GlobalInit::I64Const(i),
            Val::F32(f) => wasm::GlobalInit::F32Const(f),
            Val::F64(f) => wasm::GlobalInit::F64Const(f),
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
                let signatures = store.signatures().borrow();
                let shared_sig_index = f.sig_index();
                let (wasm, _) = signatures
                    .lookup_shared(shared_sig_index)
                    .expect("signature not registered");
                let local_sig_index = module.signatures.push(wasm.clone());
                let func_index = module.functions.push(local_sig_index);
                module.num_imported_funcs = 1;
                module.imports.push((
                    "".into(),
                    "".into(),
                    wasm::EntityIndex::Function(func_index),
                ));

                let f = f.caller_checked_anyfunc();
                let f = unsafe { f.as_ref() };
                func_imports.push(VMFunctionImport {
                    body: f.func_ptr,
                    vmctx: f.vmctx,
                });

                wasm::GlobalInit::RefFunc(func_index)
            }
            _ => unimplemented!("create_global for {:?}", gt),
        },
    };

    let global_id = module.globals.push(global);
    module
        .exports
        .insert(String::new(), wasm::EntityIndex::Global(global_id));
    let handle = create_handle(
        module,
        store,
        PrimaryMap::new(),
        Box::new(()),
        &func_imports,
    )?;

    if let Some(x) = externref_init {
        match handle.lookup("").unwrap() {
            wasmtime_runtime::Export::Global(g) => unsafe {
                *(*g.definition).as_externref_mut() = Some(x.inner);
            },
            _ => unreachable!(),
        }
    }

    Ok(handle)
}
