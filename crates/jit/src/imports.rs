//! Module imports resolving logic.

use crate::resolver::Resolver;
use more_asserts::assert_ge;
use std::collections::HashSet;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{Global, GlobalInit, Memory, Table, TableElementType};
use wasmtime_environ::{MemoryPlan, MemoryStyle, Module, TablePlan};
use wasmtime_runtime::{
    Export, Imports, InstanceHandle, LinkError, VMFunctionImport, VMGlobalImport, VMMemoryImport,
    VMTableImport,
};

/// This function allows to match all imports of a `Module` with concrete definitions provided by
/// a `Resolver`.
///
/// If all imports are satisfied returns an `Imports` instance required for a module instantiation.
pub fn resolve_imports(module: &Module, resolver: &mut dyn Resolver) -> Result<Imports, LinkError> {
    let mut dependencies = HashSet::new();

    let mut function_imports = PrimaryMap::with_capacity(module.imported_funcs.len());
    for (index, (module_name, field, import_idx)) in module.imported_funcs.iter() {
        match resolver.resolve(*import_idx, module_name, field) {
            Some(export_value) => match export_value {
                Export::Function {
                    address,
                    signature,
                    vmctx,
                } => {
                    let import_signature = &module.local.signatures[module.local.functions[index]];
                    if signature != *import_signature {
                        // TODO: If the difference is in the calling convention,
                        // we could emit a wrapper function to fix it up.
                        return Err(LinkError(format!(
                            "{}/{}: incompatible import type: exported function with signature {} \
                             incompatible with function import with signature {}",
                            module_name, field, signature, import_signature
                        )));
                    }
                    dependencies.insert(unsafe { InstanceHandle::from_vmctx(vmctx) });
                    function_imports.push(VMFunctionImport {
                        body: address,
                        vmctx,
                    });
                }
                Export::Table { .. } | Export::Memory { .. } | Export::Global { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: export incompatible with function import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "{}/{}: unknown import function: function not provided",
                    module_name, field
                )));
            }
        }
    }

    let mut table_imports = PrimaryMap::with_capacity(module.imported_tables.len());
    for (index, (module_name, field, import_idx)) in module.imported_tables.iter() {
        match resolver.resolve(*import_idx, module_name, field) {
            Some(export_value) => match export_value {
                Export::Table {
                    definition,
                    vmctx,
                    table,
                } => {
                    let import_table = &module.local.table_plans[index];
                    if !is_table_compatible(&table, import_table) {
                        return Err(LinkError(format!(
                            "{}/{}: incompatible import type: exported table incompatible with \
                             table import",
                            module_name, field,
                        )));
                    }
                    dependencies.insert(unsafe { InstanceHandle::from_vmctx(vmctx) });
                    table_imports.push(VMTableImport {
                        from: definition,
                        vmctx,
                    });
                }
                Export::Global { .. } | Export::Memory { .. } | Export::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: export incompatible with table import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "unknown import: no provided import table for {}/{}",
                    module_name, field
                )));
            }
        }
    }

    let mut memory_imports = PrimaryMap::with_capacity(module.imported_memories.len());
    for (index, (module_name, field, import_idx)) in module.imported_memories.iter() {
        match resolver.resolve(*import_idx, module_name, field) {
            Some(export_value) => match export_value {
                Export::Memory {
                    definition,
                    vmctx,
                    memory,
                } => {
                    let import_memory = &module.local.memory_plans[index];
                    if !is_memory_compatible(&memory, import_memory) {
                        return Err(LinkError(format!(
                            "{}/{}: incompatible import type: exported memory incompatible with \
                             memory import",
                            module_name, field
                        )));
                    }

                    // Sanity-check: Ensure that the imported memory has at least
                    // guard-page protections the importing module expects it to have.
                    if let (
                        MemoryStyle::Static { bound },
                        MemoryStyle::Static {
                            bound: import_bound,
                        },
                    ) = (memory.style, &import_memory.style)
                    {
                        assert_ge!(bound, *import_bound);
                    }
                    assert_ge!(memory.offset_guard_size, import_memory.offset_guard_size);

                    dependencies.insert(unsafe { InstanceHandle::from_vmctx(vmctx) });
                    memory_imports.push(VMMemoryImport {
                        from: definition,
                        vmctx,
                    });
                }
                Export::Table { .. } | Export::Global { .. } | Export::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: export incompatible with memory import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "unknown import: no provided import memory for {}/{}",
                    module_name, field
                )));
            }
        }
    }

    let mut global_imports = PrimaryMap::with_capacity(module.imported_globals.len());
    for (index, (module_name, field, import_idx)) in module.imported_globals.iter() {
        match resolver.resolve(*import_idx, module_name, field) {
            Some(export_value) => match export_value {
                Export::Table { .. } | Export::Memory { .. } | Export::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: incompatible import type: exported global incompatible with \
                         global import",
                        module_name, field
                    )));
                }
                Export::Global {
                    definition,
                    vmctx,
                    global,
                } => {
                    let imported_global = module.local.globals[index];
                    if !is_global_compatible(&global, &imported_global) {
                        return Err(LinkError(format!(
                            "{}/{}: incompatible import type: exported global incompatible with \
                             global import",
                            module_name, field
                        )));
                    }
                    dependencies.insert(unsafe { InstanceHandle::from_vmctx(vmctx) });
                    global_imports.push(VMGlobalImport { from: definition });
                }
            },
            None => {
                return Err(LinkError(format!(
                    "unknown import: no provided import global for {}/{}",
                    module_name, field
                )));
            }
        }
    }

    Ok(Imports::new(
        dependencies,
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
        ty: exported_ty,
        mutability: exported_mutability,
        initializer: _exported_initializer,
    } = exported;
    let Global {
        ty: imported_ty,
        mutability: imported_mutability,
        initializer: _imported_initializer,
    } = imported;
    exported_ty == imported_ty && imported_mutability == exported_mutability
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
                ty: exported_ty,
                minimum: exported_minimum,
                maximum: exported_maximum,
            },
        style: _exported_style,
    } = exported;
    let TablePlan {
        table:
            Table {
                ty: imported_ty,
                minimum: imported_minimum,
                maximum: imported_maximum,
            },
        style: _imported_style,
    } = imported;

    is_table_element_type_compatible(*exported_ty, *imported_ty)
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
