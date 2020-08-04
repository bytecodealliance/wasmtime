//! Module imports resolving logic.

use crate::resolver::Resolver;
use more_asserts::assert_ge;
use std::convert::TryInto;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{Global, GlobalInit, Memory, Table, TableElementType};
use wasmtime_environ::{EntityIndex, MemoryPlan, MemoryStyle, Module, TablePlan};
use wasmtime_runtime::{
    Export, Imports, LinkError, SignatureRegistry, VMFunctionImport, VMGlobalImport,
    VMMemoryImport, VMTableImport,
};

/// This function allows to match all imports of a `Module` with concrete definitions provided by
/// a `Resolver`.
///
/// If all imports are satisfied returns an `Imports` instance required for a module instantiation.
pub fn resolve_imports(
    module: &Module,
    signatures: &SignatureRegistry,
    resolver: &mut dyn Resolver,
) -> Result<Imports, LinkError> {
    let mut function_imports = PrimaryMap::with_capacity(module.num_imported_funcs);
    let mut table_imports = PrimaryMap::with_capacity(module.num_imported_tables);
    let mut memory_imports = PrimaryMap::with_capacity(module.num_imported_memories);
    let mut global_imports = PrimaryMap::with_capacity(module.num_imported_globals);

    for (import_idx, (module_name, field_name, import)) in module.imports.iter().enumerate() {
        let import_idx = import_idx.try_into().unwrap();
        let export = resolver.resolve(import_idx, module_name, field_name);

        match (import, &export) {
            (EntityIndex::Function(func_index), Some(Export::Function(f))) => {
                let import_signature = module.native_func_signature(*func_index);
                let signature = signatures
                    .lookup_native(unsafe { f.anyfunc.as_ref().type_index })
                    .unwrap();
                if signature != *import_signature {
                    // TODO: If the difference is in the calling convention,
                    // we could emit a wrapper function to fix it up.
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: exported function with signature {} \
                         incompatible with function import with signature {}",
                        module_name, field_name, signature, import_signature
                    )));
                }
                function_imports.push(VMFunctionImport {
                    body: unsafe { f.anyfunc.as_ref().func_ptr },
                    vmctx: unsafe { f.anyfunc.as_ref().vmctx },
                });
            }
            (EntityIndex::Function(_), Some(_)) => {
                return Err(LinkError(format!(
                    "{}/{}: incompatible import type: export incompatible with function import",
                    module_name, field_name
                )));
            }
            (EntityIndex::Function(_), None) => {
                return Err(LinkError(format!(
                    "{}/{}: unknown import function: function not provided",
                    module_name, field_name
                )));
            }

            (EntityIndex::Table(table_index), Some(Export::Table(t))) => {
                let import_table = &module.table_plans[*table_index];
                if !is_table_compatible(&t.table, import_table) {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: exported table incompatible with \
                             table import",
                        module_name, field_name,
                    )));
                }
                table_imports.push(VMTableImport {
                    from: t.definition,
                    vmctx: t.vmctx,
                });
            }
            (EntityIndex::Table(_), Some(_)) => {
                return Err(LinkError(format!(
                    "{}/{}: incompatible import type: export incompatible with table import",
                    module_name, field_name
                )));
            }
            (EntityIndex::Table(_), None) => {
                return Err(LinkError(format!(
                    "{}/{}: unknown import table: table not provided",
                    module_name, field_name
                )));
            }

            (EntityIndex::Memory(memory_index), Some(Export::Memory(m))) => {
                let import_memory = &module.memory_plans[*memory_index];
                if !is_memory_compatible(&m.memory, import_memory) {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: exported memory incompatible with \
                             memory import",
                        module_name, field_name
                    )));
                }

                // Sanity-check: Ensure that the imported memory has at least
                // guard-page protections the importing module expects it to have.
                if let (
                    MemoryStyle::Static { bound },
                    MemoryStyle::Static {
                        bound: import_bound,
                    },
                ) = (&m.memory.style, &import_memory.style)
                {
                    assert_ge!(*bound, *import_bound);
                }
                assert_ge!(m.memory.offset_guard_size, import_memory.offset_guard_size);

                memory_imports.push(VMMemoryImport {
                    from: m.definition,
                    vmctx: m.vmctx,
                });
            }
            (EntityIndex::Memory(_), Some(_)) => {
                return Err(LinkError(format!(
                    "{}/{}: incompatible import type: export incompatible with memory import",
                    module_name, field_name
                )));
            }
            (EntityIndex::Memory(_), None) => {
                return Err(LinkError(format!(
                    "{}/{}: unknown import memory: memory not provided",
                    module_name, field_name
                )));
            }

            (EntityIndex::Global(global_index), Some(Export::Global(g))) => {
                let imported_global = module.globals[*global_index];
                if !is_global_compatible(&g.global, &imported_global) {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: exported global incompatible with \
                             global import",
                        module_name, field_name
                    )));
                }
                global_imports.push(VMGlobalImport { from: g.definition });
            }
            (EntityIndex::Global(_), Some(_)) => {
                return Err(LinkError(format!(
                    "{}/{}: incompatible import type: export incompatible with global import",
                    module_name, field_name
                )));
            }
            (EntityIndex::Global(_), None) => {
                return Err(LinkError(format!(
                    "{}/{}: unknown import global: global not provided",
                    module_name, field_name
                )));
            }
        }
    }

    Ok(Imports::new(
        function_imports,
        table_imports,
        memory_imports,
        global_imports,
    ))
}

fn is_global_compatible(exported: &Global, imported: &Global) -> bool {
    match imported.initializer {
        GlobalInit::Import => (),
        _ => panic!("imported Global should have an Imported initializer"),
    }

    let Global {
        wasm_ty: exported_wasm_ty,
        ty: exported_ty,
        mutability: exported_mutability,
        initializer: _exported_initializer,
    } = exported;
    let Global {
        wasm_ty: imported_wasm_ty,
        ty: imported_ty,
        mutability: imported_mutability,
        initializer: _imported_initializer,
    } = imported;
    exported_wasm_ty == imported_wasm_ty
        && exported_ty == imported_ty
        && imported_mutability == exported_mutability
}

fn is_table_element_type_compatible(
    exported_type: TableElementType,
    imported_type: TableElementType,
) -> bool {
    match exported_type {
        TableElementType::Func => match imported_type {
            TableElementType::Func => true,
            _ => false,
        },
        TableElementType::Val(exported_val_ty) => match imported_type {
            TableElementType::Val(imported_val_ty) => exported_val_ty == imported_val_ty,
            _ => false,
        },
    }
}

fn is_table_compatible(exported: &TablePlan, imported: &TablePlan) -> bool {
    let TablePlan {
        table:
            Table {
                wasm_ty: exported_wasm_ty,
                ty: exported_ty,
                minimum: exported_minimum,
                maximum: exported_maximum,
            },
        style: _exported_style,
    } = exported;
    let TablePlan {
        table:
            Table {
                wasm_ty: imported_wasm_ty,
                ty: imported_ty,
                minimum: imported_minimum,
                maximum: imported_maximum,
            },
        style: _imported_style,
    } = imported;

    exported_wasm_ty == imported_wasm_ty
        && is_table_element_type_compatible(*exported_ty, *imported_ty)
        && imported_minimum <= exported_minimum
        && (imported_maximum.is_none()
            || (!exported_maximum.is_none()
                && imported_maximum.unwrap() >= exported_maximum.unwrap()))
}

fn is_memory_compatible(exported: &MemoryPlan, imported: &MemoryPlan) -> bool {
    let MemoryPlan {
        memory:
            Memory {
                minimum: exported_minimum,
                maximum: exported_maximum,
                shared: exported_shared,
            },
        style: _exported_style,
        offset_guard_size: _exported_offset_guard_size,
    } = exported;
    let MemoryPlan {
        memory:
            Memory {
                minimum: imported_minimum,
                maximum: imported_maximum,
                shared: imported_shared,
            },
        style: _imported_style,
        offset_guard_size: _imported_offset_guard_size,
    } = imported;

    imported_minimum <= exported_minimum
        && (imported_maximum.is_none()
            || (!exported_maximum.is_none()
                && imported_maximum.unwrap() >= exported_maximum.unwrap()))
        && exported_shared == imported_shared
}
