//! Linking for JIT-compiled code.

use crate::resolver::Resolver;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::JumpTableOffsets;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, Global, GlobalInit, Memory, Table, TableElementType};
use more_asserts::assert_ge;
use std::collections::HashSet;
use std::ptr::write_unaligned;
use wasmtime_environ::{
    MemoryPlan, MemoryStyle, Module, Relocation, RelocationTarget, Relocations, TablePlan,
};
use wasmtime_runtime::libcalls;
use wasmtime_runtime::{
    Export, Imports, InstanceHandle, LinkError, VMFunctionBody, VMFunctionImport, VMGlobalImport,
    VMMemoryImport, VMTableImport,
};

/// Links a module that has been compiled with `compiled_module` in `wasmtime-environ`.
pub fn link_module(
    module: &Module,
    allocated_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    jt_offsets: &PrimaryMap<DefinedFuncIndex, JumpTableOffsets>,
    relocations: Relocations,
    resolver: &mut dyn Resolver,
) -> Result<Imports, LinkError> {
    let mut dependencies = HashSet::new();

    let mut function_imports = PrimaryMap::with_capacity(module.imported_funcs.len());
    for (index, (ref module_name, ref field)) in module.imported_funcs.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                Export::Function {
                    address,
                    signature,
                    vmctx,
                } => {
                    let import_signature = &module.signatures[module.functions[index]];
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
    for (index, (ref module_name, ref field)) in module.imported_tables.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                Export::Table {
                    definition,
                    vmctx,
                    table,
                } => {
                    let import_table = &module.table_plans[index];
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
    for (index, (ref module_name, ref field)) in module.imported_memories.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                Export::Memory {
                    definition,
                    vmctx,
                    memory,
                } => {
                    let import_memory = &module.memory_plans[index];
                    if !is_memory_compatible(&memory, import_memory) {
                        return Err(LinkError(format!(
                            "{}/{}: incompatible import type: exported memory incompatible with \
                             memory import",
                            module_name, field
                        )));
                    }

                    // Sanity-check: Ensure that the imported memory has at least
                    // guard-page protections the importing module expects it to have.
                    match (memory.style, &import_memory.style) {
                        (
                            MemoryStyle::Static { bound },
                            MemoryStyle::Static {
                                bound: import_bound,
                            },
                        ) => {
                            assert_ge!(bound, *import_bound);
                        }
                        _ => (),
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
    for (index, (ref module_name, ref field)) in module.imported_globals.iter() {
        match resolver.resolve(module_name, field) {
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
                    let imported_global = module.globals[index];
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

    // Apply relocations, now that we have virtual addresses for everything.
    relocate(allocated_functions, jt_offsets, relocations, module);

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

/// Performs the relocations inside the function bytecode, provided the necessary metadata.
fn relocate(
    allocated_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    jt_offsets: &PrimaryMap<DefinedFuncIndex, JumpTableOffsets>,
    relocations: PrimaryMap<DefinedFuncIndex, Vec<Relocation>>,
    module: &Module,
) {
    for (i, function_relocs) in relocations.into_iter() {
        for r in function_relocs {
            use self::libcalls::*;
            let target_func_address: usize = match r.reloc_target {
                RelocationTarget::UserFunc(index) => match module.defined_func_index(index) {
                    Some(f) => {
                        let fatptr: *const [VMFunctionBody] = allocated_functions[f];
                        fatptr as *const VMFunctionBody as usize
                    }
                    None => panic!("direct call to import"),
                },
                RelocationTarget::Memory32Grow => wasmtime_memory32_grow as usize,
                RelocationTarget::Memory32Size => wasmtime_memory32_size as usize,
                RelocationTarget::ImportedMemory32Grow => wasmtime_imported_memory32_grow as usize,
                RelocationTarget::ImportedMemory32Size => wasmtime_imported_memory32_size as usize,
                RelocationTarget::LibCall(libcall) => {
                    use cranelift_codegen::ir::LibCall::*;
                    match libcall {
                        CeilF32 => wasmtime_f32_ceil as usize,
                        FloorF32 => wasmtime_f32_floor as usize,
                        TruncF32 => wasmtime_f32_trunc as usize,
                        NearestF32 => wasmtime_f32_nearest as usize,
                        CeilF64 => wasmtime_f64_ceil as usize,
                        FloorF64 => wasmtime_f64_floor as usize,
                        TruncF64 => wasmtime_f64_trunc as usize,
                        NearestF64 => wasmtime_f64_nearest as usize,
                        #[cfg(not(target_os = "windows"))]
                        Probestack => __rust_probestack as usize,
                        #[cfg(all(target_os = "windows", target_env = "gnu"))]
                        Probestack => ___chkstk as usize,
                        #[cfg(all(
                            target_os = "windows",
                            target_env = "msvc",
                            target_pointer_width = "64"
                        ))]
                        Probestack => __chkstk as usize,
                        other => panic!("unexpected libcall: {}", other),
                    }
                }
                RelocationTarget::JumpTable(func_index, jt) => {
                    match module.defined_func_index(func_index) {
                        Some(f) => {
                            let offset = *jt_offsets
                                .get(f)
                                .and_then(|ofs| ofs.get(jt))
                                .expect("func jump table");
                            let fatptr: *const [VMFunctionBody] = allocated_functions[f];
                            fatptr as *const VMFunctionBody as usize + offset as usize
                        }
                        None => panic!("func index of jump table"),
                    }
                }
            };

            let fatptr: *const [VMFunctionBody] = allocated_functions[i];
            let body = fatptr as *const VMFunctionBody;
            match r.reloc {
                #[cfg(target_pointer_width = "64")]
                Reloc::Abs8 => unsafe {
                    let reloc_address = body.add(r.offset as usize) as usize;
                    let reloc_addend = r.addend as isize;
                    let reloc_abs = (target_func_address as u64)
                        .checked_add(reloc_addend as u64)
                        .unwrap();
                    write_unaligned(reloc_address as *mut u64, reloc_abs);
                },
                #[cfg(target_pointer_width = "32")]
                Reloc::X86PCRel4 => unsafe {
                    let reloc_address = body.add(r.offset as usize) as usize;
                    let reloc_addend = r.addend as isize;
                    let reloc_delta_u32 = (target_func_address as u32)
                        .wrapping_sub(reloc_address as u32)
                        .checked_add(reloc_addend as u32)
                        .unwrap();
                    write_unaligned(reloc_address as *mut u32, reloc_delta_u32);
                },
                #[cfg(target_pointer_width = "32")]
                Reloc::X86CallPCRel4 => {
                    // ignore
                }
                Reloc::X86PCRelRodata4 => {
                    // ignore
                }
                _ => panic!("unsupported reloc kind"),
            }
        }
    }
}

/// A declaration for the stack probe function in Rust's standard library, for
/// catching callstack overflow.
extern "C" {
    #[cfg(not(target_os = "windows"))]
    pub fn __rust_probestack();
    #[cfg(all(
        target_os = "windows",
        target_env = "msvc",
        target_pointer_width = "64"
    ))]
    pub fn __chkstk();
    // ___chkstk (note the triple underscore) is implemented in compiler-builtins/src/x86_64.rs
    // by the Rust compiler for the MinGW target
    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    pub fn ___chkstk();
}
