//! TODO: Move the contents of this file to other files, as "execute.rs" is
//! no longer a descriptive filename.

use code::Code;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::isa::TargetIsa;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{DefinedFuncIndex, MemoryIndex};
use instance::Instance;
use invoke::{invoke_by_index, InvokeOutcome};
use region::protect;
use region::Protection;
use std::ptr::write_unaligned;
use std::string::String;
use std::vec::Vec;
use vmcontext::VMContext;
use wasmtime_environ::{
    compile_module, Compilation, Module, ModuleTranslation, Relocation, RelocationTarget,
};

/// Executes a module that has been translated with the `wasmtime-environ` environment
/// implementation.
pub fn compile_and_link_module<'data, 'module, F>(
    isa: &TargetIsa,
    translation: &ModuleTranslation<'data, 'module>,
    imports: F,
) -> Result<Compilation, String>
where
    F: Fn(&str, &str) -> Option<usize>,
{
    let (mut compilation, relocations) = compile_module(&translation, isa)?;

    // Apply relocations, now that we have virtual addresses for everything.
    relocate(&mut compilation, &relocations, &translation.module, imports);

    Ok(compilation)
}

/// Performs the relocations inside the function bytecode, provided the necessary metadata
fn relocate<F>(
    compilation: &mut Compilation,
    relocations: &PrimaryMap<DefinedFuncIndex, Vec<Relocation>>,
    module: &Module,
    imports: F,
) where
    F: Fn(&str, &str) -> Option<usize>,
{
    // The relocations are relative to the relocation's address plus four bytes
    // TODO: Support architectures other than x64, and other reloc kinds.
    for (i, function_relocs) in relocations.iter() {
        for r in function_relocs {
            let target_func_address: usize = match r.reloc_target {
                RelocationTarget::UserFunc(index) => match module.defined_func_index(index) {
                    Some(f) => compilation.functions[f].as_ptr() as usize,
                    None => {
                        let func = &module.imported_funcs[index];
                        match imports(&func.0, &func.1) {
                            Some(ptr) => ptr,
                            None => {
                                panic!("no provided import function for {}/{}", &func.0, &func.1)
                            }
                        }
                    }
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
            InvokeOutcome::Returned { values } => {
                assert!(values.is_empty());
            }
            InvokeOutcome::Trapped { message } => {
                return Err(format!("start function trapped: {}", message));
            }
        }
    }

    Ok(())
}
