//! TODO: Move the contents of this file to other files, as "execute.rs" is
//! no longer a descriptive filename.

use action::ActionOutcome;
use code::Code;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::isa::TargetIsa;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, Global, GlobalInit, Memory, MemoryIndex, Table, TableElementType,
};
use export::{ExportValue, Resolver};
use instance::Instance;
use invoke::invoke_by_index;
use region::{protect, Protection};
use std::ptr::write_unaligned;
use std::string::String;
use std::vec::Vec;
use vmcontext::VMContext;
use wasmtime_environ::{
    compile_module, Compilation, MemoryPlan, MemoryStyle, Module, ModuleTranslation, Relocation,
    RelocationTarget, TablePlan, TableStyle,
};

/// Executes a module that has been translated with the `wasmtime-environ` environment
/// implementation.
pub fn compile_and_link_module<'data, 'module>(
    isa: &TargetIsa,
    translation: &ModuleTranslation<'data, 'module>,
    resolver: &mut Resolver,
) -> Result<Compilation, String> {
    let (mut compilation, relocations) = compile_module(&translation, isa)?;

    for (index, (ref module, ref field)) in translation.module.imported_funcs.iter() {
        match resolver.resolve(module, field) {
            Some(export_value) => match export_value {
                ExportValue::Function { address, signature } => {
                    let import_signature =
                        &translation.module.signatures[translation.module.functions[index]];
                    if signature != *import_signature {
                        return Err(format!(
                            "{}/{}: exported function with signature {} incompatible with function import with signature {}",
                            module, field,
                            signature, import_signature,
                        ));
                    }
                    compilation.resolved_func_imports.push(address);
                }
                ExportValue::Table { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Global { .. } => {
                    return Err(format!(
                        "{}/{}: export not compatible with function import",
                        module, field
                    ));
                }
            },
            None => return Err(format!("{}/{}: no provided import function", module, field)),
        }
    }
    for (index, (ref module, ref field)) in translation.module.imported_globals.iter() {
        match resolver.resolve(module, field) {
            Some(export_value) => match export_value {
                ExportValue::Global { address, global } => {
                    let imported_global = translation.module.globals[index];
                    if !is_global_compatible(&global, &imported_global) {
                        return Err(format!(
                            "{}/{}: exported global incompatible with global import",
                            module, field,
                        ));
                    }
                    compilation.resolved_global_imports.push(address as usize);
                }
                ExportValue::Table { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Function { .. } => {
                    return Err(format!(
                        "{}/{}: exported global incompatible with global import",
                        module, field
                    ));
                }
            },
            None => {
                return Err(format!(
                    "no provided import global for {}/{}",
                    module, field
                ))
            }
        }
    }
    for (index, (ref module, ref field)) in translation.module.imported_tables.iter() {
        match resolver.resolve(module, field) {
            Some(export_value) => match export_value {
                ExportValue::Table { address, table } => {
                    let import_table = &translation.module.table_plans[index];
                    if !is_table_compatible(&table, import_table) {
                        return Err(format!(
                            "{}/{}: exported table incompatible with table import",
                            module, field,
                        ));
                    }
                    compilation.resolved_table_imports.push(address as usize);
                }
                ExportValue::Global { .. }
                | ExportValue::Memory { .. }
                | ExportValue::Function { .. } => {
                    return Err(format!(
                        "{}/{}: export not compatible with table import",
                        module, field
                    ));
                }
            },
            None => return Err(format!("no provided import table for {}/{}", module, field)),
        }
    }
    for (index, (ref module, ref field)) in translation.module.imported_memories.iter() {
        match resolver.resolve(module, field) {
            Some(export_value) => match export_value {
                ExportValue::Memory { address, memory } => {
                    let import_memory = &translation.module.memory_plans[index];
                    if is_memory_compatible(&memory, import_memory) {
                        return Err(format!(
                            "{}/{}: exported memory incompatible with memory import",
                            module, field
                        ));
                    }
                    compilation.resolved_memory_imports.push(address as usize);
                }
                ExportValue::Table { .. }
                | ExportValue::Global { .. }
                | ExportValue::Function { .. } => {
                    return Err(format!(
                        "{}/{}: export not compatible with memory import",
                        module, field
                    ));
                }
            },
            None => {
                return Err(format!(
                    "no provided import memory for {}/{}",
                    module, field
                ))
            }
        }
    }

    // Apply relocations, now that we have virtual addresses for everything.
    relocate(&mut compilation, &relocations, &translation.module)?;

    Ok(compilation)
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

extern "C" {
    pub fn __rust_probestack();
}

/// Performs the relocations inside the function bytecode, provided the necessary metadata.
fn relocate(
    compilation: &mut Compilation,
    relocations: &PrimaryMap<DefinedFuncIndex, Vec<Relocation>>,
    module: &Module,
) -> Result<(), String> {
    // The relocations are relative to the relocation's address plus four bytes.
    for (i, function_relocs) in relocations.iter() {
        for r in function_relocs {
            let target_func_address: usize = match r.reloc_target {
                RelocationTarget::UserFunc(index) => match module.defined_func_index(index) {
                    Some(f) => compilation.functions[f].as_ptr() as usize,
                    None => compilation.resolved_func_imports[index],
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

            let body = &mut compilation.functions[i];
            match r.reloc {
                #[cfg(target_pointer_width = "64")]
                Reloc::Abs8 => unsafe {
                    let reloc_address = body.as_mut_ptr().add(r.offset as usize) as usize;
                    let reloc_addend = r.addend as isize;
                    let reloc_abs = (target_func_address as u64)
                        .checked_add(reloc_addend as u64)
                        .unwrap();
                    write_unaligned(reloc_address as *mut u64, reloc_abs);
                },
                #[cfg(target_pointer_width = "32")]
                Reloc::X86PCRel4 => unsafe {
                    let reloc_address = body.as_mut_ptr().add(r.offset as usize) as usize;
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
    Ok(())
}

extern "C" fn wasmtime_memory_grow(size: u32, memory_index: u32, vmctx: *mut VMContext) -> u32 {
    let instance = unsafe { (&mut *vmctx).instance() };
    let memory_index = MemoryIndex::new(memory_index as usize);

    instance
        .memory_grow(memory_index, size)
        .unwrap_or(u32::max_value())
}

extern "C" fn wasmtime_memory_size(memory_index: u32, vmctx: *mut VMContext) -> u32 {
    let instance = unsafe { (&mut *vmctx).instance() };
    let memory_index = MemoryIndex::new(memory_index as usize);

    instance.memory_size(memory_index)
}

/// prepares the execution context
pub fn finish_instantiation(
    code: &mut Code,
    isa: &TargetIsa,
    module: &Module,
    compilation: &Compilation,
    instance: &mut Instance,
) -> Result<(), String> {
    // TODO: Put all the function bodies into a page-aligned memory region, and
    // then make them ReadExecute rather than ReadWriteExecute.
    for code_buf in compilation.functions.values() {
        match unsafe {
            protect(
                code_buf.as_ptr(),
                code_buf.len(),
                Protection::ReadWriteExecute,
            )
        } {
            Ok(()) => (),
            Err(err) => {
                return Err(format!(
                    "failed to give executable permission to code: {}",
                    err
                ))
            }
        }
    }

    if let Some(start_index) = module.start_func {
        let vmctx = instance.vmctx();
        let result = invoke_by_index(code, isa, module, compilation, vmctx, start_index, &[])?;
        match result {
            ActionOutcome::Returned { values } => {
                assert!(values.is_empty());
            }
            ActionOutcome::Trapped { message } => {
                return Err(format!("start function trapped: {}", message));
            }
        }
    }

    Ok(())
}
