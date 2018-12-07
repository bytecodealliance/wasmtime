use cranelift_codegen::binemit::Reloc;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, Global, GlobalInit, Memory, MemoryIndex, Table, TableElementType,
};
use export::{ExportValue, Resolver};
use imports::Imports;
use std::ptr::write_unaligned;
use std::vec::Vec;
use vmcontext::VMContext;
use vmcontext::{VMGlobal, VMMemory, VMTable};
use wasmtime_environ::{
    MemoryPlan, MemoryStyle, Module, Relocation, RelocationTarget, Relocations, TablePlan,
    TableStyle,
};

/// A link error, such as incompatible or unmatched imports/exports.
#[derive(Fail, Debug)]
#[fail(display = "Link error: {}", _0)]
pub struct LinkError(String);

/// Links a module that has been compiled with `compiled_module` in `wasmtime-environ`.
pub fn link_module(
    module: &Module,
    allocated_functions: &PrimaryMap<DefinedFuncIndex, (*mut u8, usize)>,
    relocations: Relocations,
    resolver: &mut Resolver,
) -> Result<Imports, LinkError> {
    let mut imports = Imports::new();

    for (index, (ref module_name, ref field)) in module.imported_funcs.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                ExportValue::Function { address, signature } => {
                    let import_signature = &module.signatures[module.functions[index]];
                    if signature != *import_signature {
                        return Err(LinkError(
                            format!("{}/{}: exported function with signature {} incompatible with function import with signature {}",
                            module_name, field,
                            signature, import_signature)
                        ));
                    }
                    imports.functions.push(address);
                }
                ExportValue::Table { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Global { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: export not compatible with function import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "{}/{}: no provided import function",
                    module_name, field
                )))
            }
        }
    }

    for (index, (ref module_name, ref field)) in module.imported_globals.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                ExportValue::Global { address, global } => {
                    let imported_global = module.globals[index];
                    if !is_global_compatible(&global, &imported_global) {
                        return Err(LinkError(format!(
                            "{}/{}: exported global incompatible with global import",
                            module_name, field
                        )));
                    }
                    imports.globals.push(address as *mut VMGlobal);
                }
                ExportValue::Table { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: exported global incompatible with global import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "no provided import global for {}/{}",
                    module_name, field
                )))
            }
        }
    }

    for (index, (ref module_name, ref field)) in module.imported_tables.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                ExportValue::Table { address, table } => {
                    let import_table = &module.table_plans[index];
                    if !is_table_compatible(&table, import_table) {
                        return Err(LinkError(format!(
                            "{}/{}: exported table incompatible with table import",
                            module_name, field,
                        )));
                    }
                    imports.tables.push(address as *mut VMTable);
                }
                ExportValue::Global { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: export not compatible with table import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "no provided import table for {}/{}",
                    module_name, field
                )))
            }
        }
    }

    for (index, (ref module_name, ref field)) in module.imported_memories.iter() {
        match resolver.resolve(module_name, field) {
            Some(export_value) => match export_value {
                ExportValue::Memory { address, memory } => {
                    let import_memory = &module.memory_plans[index];
                    if is_memory_compatible(&memory, import_memory) {
                        return Err(LinkError(format!(
                            "{}/{}: exported memory incompatible with memory import",
                            module_name, field
                        )));
                    }
                    imports.memories.push(address as *mut VMMemory);
                }
                ExportValue::Table { .. }
                | ExportValue::Global { .. }
                | ExportValue::Function { .. } => {
                    return Err(LinkError(format!(
                        "{}/{}: export not compatible with memory import",
                        module_name, field
                    )));
                }
            },
            None => {
                return Err(LinkError(format!(
                    "no provided import memory for {}/{}",
                    module_name, field
                )))
            }
        }
    }

    // Apply relocations, now that we have virtual addresses for everything.
    relocate(&imports, allocated_functions, relocations, &module);

    Ok(imports)
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

fn is_table_style_compatible(exported_style: &TableStyle, imported_style: &TableStyle) -> bool {
    match exported_style {
        TableStyle::CallerChecksSignature => match imported_style {
            TableStyle::CallerChecksSignature => true,
        },
    }
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
        style: exported_style,
    } = exported;
    let TablePlan {
        table:
            Table {
                ty: imported_ty,
                minimum: imported_minimum,
                maximum: imported_maximum,
            },
        style: imported_style,
    } = imported;

    is_table_element_type_compatible(*exported_ty, *imported_ty)
        && imported_minimum >= exported_minimum
        && imported_maximum <= exported_maximum
        && is_table_style_compatible(imported_style, exported_style)
}

fn is_memory_style_compatible(exported_style: &MemoryStyle, imported_style: &MemoryStyle) -> bool {
    match exported_style {
        MemoryStyle::Dynamic => match imported_style {
            MemoryStyle::Dynamic => true,
            _ => false,
        },
        MemoryStyle::Static {
            bound: imported_bound,
        } => match imported_style {
            MemoryStyle::Static {
                bound: exported_bound,
            } => exported_bound >= imported_bound,
            _ => false,
        },
    }
}

fn is_memory_compatible(exported: &MemoryPlan, imported: &MemoryPlan) -> bool {
    let MemoryPlan {
        memory:
            Memory {
                minimum: exported_minimum,
                maximum: exported_maximum,
                shared: exported_shared,
            },
        style: exported_style,
        offset_guard_size: exported_offset_guard_size,
    } = exported;
    let MemoryPlan {
        memory:
            Memory {
                minimum: imported_minimum,
                maximum: imported_maximum,
                shared: imported_shared,
            },
        style: imported_style,
        offset_guard_size: imported_offset_guard_size,
    } = imported;

    imported_minimum >= exported_minimum
        && imported_maximum <= exported_maximum
        && exported_shared == imported_shared
        && is_memory_style_compatible(exported_style, imported_style)
        && exported_offset_guard_size >= imported_offset_guard_size
}

/// Performs the relocations inside the function bytecode, provided the necessary metadata.
fn relocate(
    imports: &Imports,
    allocated_functions: &PrimaryMap<DefinedFuncIndex, (*mut u8, usize)>,
    relocations: PrimaryMap<DefinedFuncIndex, Vec<Relocation>>,
    module: &Module,
) {
    for (i, function_relocs) in relocations.into_iter() {
        for r in function_relocs {
            let target_func_address: usize = match r.reloc_target {
                RelocationTarget::UserFunc(index) => match module.defined_func_index(index) {
                    Some(f) => allocated_functions[f].0 as usize,
                    None => imports.functions[index] as usize,
                },
                RelocationTarget::MemoryGrow => wasmtime_memory_grow as usize,
                RelocationTarget::MemorySize => wasmtime_memory_size as usize,
                RelocationTarget::LibCall(libcall) => {
                    use cranelift_codegen::ir::LibCall::*;
                    use libcalls::*;
                    match libcall {
                        CeilF32 => wasmtime_f32_ceil as usize,
                        FloorF32 => wasmtime_f32_floor as usize,
                        TruncF32 => wasmtime_f32_trunc as usize,
                        NearestF32 => wasmtime_f32_nearest as usize,
                        CeilF64 => wasmtime_f64_ceil as usize,
                        FloorF64 => wasmtime_f64_floor as usize,
                        TruncF64 => wasmtime_f64_trunc as usize,
                        NearestF64 => wasmtime_f64_nearest as usize,
                        Probestack => __rust_probestack as usize,
                        other => panic!("unexpected libcall: {}", other),
                    }
                }
            };

            let body = allocated_functions[i].0;
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
                _ => panic!("unsupported reloc kind"),
            }
        }
    }
}

/// A declaration for the stack probe function in Rust's standard library, for
/// catching callstack overflow.
extern "C" {
    pub fn __rust_probestack();
}

/// The implementation of memory.grow.
extern "C" fn wasmtime_memory_grow(size: u32, memory_index: u32, vmctx: *mut VMContext) -> u32 {
    let instance = unsafe { (&mut *vmctx).instance() };
    let memory_index = MemoryIndex::new(memory_index as usize);

    instance
        .memory_grow(memory_index, size)
        .unwrap_or(u32::max_value())
}

/// The implementation of memory.size.
extern "C" fn wasmtime_memory_size(memory_index: u32, vmctx: *mut VMContext) -> u32 {
    let instance = unsafe { (&mut *vmctx).instance() };
    let memory_index = MemoryIndex::new(memory_index as usize);

    instance.memory_size(memory_index)
}
