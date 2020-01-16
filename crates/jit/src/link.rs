//! Linking for JIT-compiled code.

use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::JumpTableOffsets;
use std::ptr::write_unaligned;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::{Module, RelocationTarget, Relocations};
use wasmtime_runtime::libcalls;
use wasmtime_runtime::VMFunctionBody;

/// Links a module that has been compiled with `compiled_module` in `wasmtime-environ`.
///
/// Performs all required relocations inside the function code, provided the necessary metadata.
pub fn link_module(
    module: &Module,
    allocated_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    jt_offsets: &PrimaryMap<DefinedFuncIndex, JumpTableOffsets>,
    relocations: Relocations,
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
