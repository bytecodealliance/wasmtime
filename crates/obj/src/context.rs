#![allow(clippy::cast_ptr_alignment)]

use more_asserts::assert_le;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ptr;
use wasmtime_environ::entity::EntityRef;
use wasmtime_environ::isa::TargetFrontendConfig;
use wasmtime_environ::wasm::GlobalInit;
use wasmtime_environ::{Module, TargetSharedSignatureIndex, VMOffsets};

pub struct TableRelocation {
    pub index: usize,
    pub offset: usize,
}

pub fn layout_vmcontext(
    module: &Module,
    target_config: &TargetFrontendConfig,
) -> (Box<[u8]>, Box<[TableRelocation]>) {
    let ofs = VMOffsets::new(target_config.pointer_bytes(), &module);
    let out_len = ofs.size_of_vmctx() as usize;
    let mut out = vec![0; out_len];

    // Assign unique indicies to unique signatures.
    let mut signature_registry = HashMap::new();
    let mut signature_registry_len = signature_registry.len();
    for (index, sig) in module.signatures.iter() {
        let offset = ofs.vmctx_vmshared_signature_id(index) as usize;
        let target_index = match signature_registry.entry(sig) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                assert_le!(signature_registry_len, std::u32::MAX as usize);
                let id = TargetSharedSignatureIndex::new(signature_registry_len as u32);
                signature_registry_len += 1;
                *v.insert(id)
            }
        };
        unsafe {
            let to = out.as_mut_ptr().add(offset) as *mut TargetSharedSignatureIndex;
            ptr::write(to, target_index);
        }
    }

    let num_tables_imports = module.imported_tables.len();
    let mut table_relocs = Vec::with_capacity(module.table_plans.len() - num_tables_imports);
    for (index, table) in module.table_plans.iter().skip(num_tables_imports) {
        let def_index = module.defined_table_index(index).unwrap();
        let offset = ofs.vmctx_vmtable_definition(def_index) as usize;
        let current_elements = table.table.minimum;
        unsafe {
            assert_eq!(
                ::std::mem::size_of::<u32>() as u8,
                ofs.size_of_vmtable_definition_current_elements(),
                "vmtable_definition_current_elements expected to be u32"
            );
            let to = out
                .as_mut_ptr()
                .add(offset)
                .add(ofs.vmtable_definition_current_elements() as usize);
            ptr::write(to as *mut u32, current_elements);
        }
        table_relocs.push(TableRelocation {
            index: def_index.index(),
            offset,
        });
    }

    let num_globals_imports = module.imported_globals.len();
    for (index, global) in module.globals.iter().skip(num_globals_imports) {
        let def_index = module.defined_global_index(index).unwrap();
        let offset = ofs.vmctx_vmglobal_definition(def_index) as usize;
        let to = unsafe { out.as_mut_ptr().add(offset) };
        match global.initializer {
            GlobalInit::I32Const(x) => unsafe {
                ptr::write(to as *mut i32, x);
            },
            GlobalInit::I64Const(x) => unsafe {
                ptr::write(to as *mut i64, x);
            },
            GlobalInit::F32Const(x) => unsafe {
                ptr::write(to as *mut u32, x);
            },
            GlobalInit::F64Const(x) => unsafe {
                ptr::write(to as *mut u64, x);
            },
            _ => panic!("unsupported global type"),
        }
    }

    (out.into_boxed_slice(), table_relocs.into_boxed_slice())
}
