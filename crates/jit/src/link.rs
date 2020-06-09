//! Linking for JIT-compiled code.

use object::read::{Object, ObjectSection, Relocation, RelocationTarget};
use object::{elf, File, RelocationEncoding, RelocationKind};
use std::ptr::{read_unaligned, write_unaligned};
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::wasm::{DefinedFuncIndex, FuncIndex};
use wasmtime_environ::Module;
use wasmtime_runtime::libcalls;
use wasmtime_runtime::VMFunctionBody;

/// Links a module that has been compiled with `compiled_module` in `wasmtime-environ`.
///
/// Performs all required relocations inside the function code, provided the necessary metadata.
pub fn link_module(
    obj: &File,
    module: &Module,
    code_range: &mut [u8],
    finished_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
) {
    let text_section = obj.section_by_name(".text").unwrap();
    let body = code_range.as_ptr() as *const VMFunctionBody;

    for (offset, r) in text_section.relocations() {
        apply_reloc(module, obj, finished_functions, body, offset, r);
    }
}

fn apply_reloc(
    module: &Module,
    obj: &File,
    finished_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    body: *const VMFunctionBody,
    offset: u64,
    r: Relocation,
) {
    let target_func_address: usize = match r.target() {
        RelocationTarget::Symbol(i) => {
            let sym = obj.symbol_by_index(i).unwrap();
            match sym.name() {
                Some(name) => {
                    if name.starts_with("_wasm_function_") {
                        let index = name["_wasm_function_".len()..].parse::<usize>().unwrap();
                        match module.local.defined_func_index(FuncIndex::new(index)) {
                            Some(f) => {
                                let fatptr: *const [VMFunctionBody] = finished_functions[f];
                                fatptr as *const VMFunctionBody as usize
                            }
                            None => panic!("direct call to import"),
                        }
                    } else if name.starts_with("wasmtime_") {
                        to_libcall_address(name)
                    } else {
                        panic!("unknown function to link: {}", name);
                    }
                }
                None => panic!("unexpected relocation target: not a symbol"),
            }
        }
        _ => panic!("unexpected relocation target"),
    };

    match (r.kind(), r.encoding(), r.size()) {
        #[cfg(target_pointer_width = "64")]
        (RelocationKind::Absolute, RelocationEncoding::Generic, 64) => unsafe {
            let reloc_address = body.add(offset as usize) as usize;
            let reloc_addend = r.addend() as isize;
            let reloc_abs = (target_func_address as u64)
                .checked_add(reloc_addend as u64)
                .unwrap();
            write_unaligned(reloc_address as *mut u64, reloc_abs);
        },
        #[cfg(target_pointer_width = "32")]
        (RelocationKind::Relative, RelocationEncoding::Generic, 32) => unsafe {
            let reloc_address = body.add(offset as usize) as usize;
            let reloc_addend = r.addend() as isize;
            let reloc_delta_u32 = (target_func_address as u32)
                .wrapping_sub(reloc_address as u32)
                .checked_add(reloc_addend as u32)
                .unwrap();
            write_unaligned(reloc_address as *mut u32, reloc_delta_u32);
        },
        #[cfg(target_pointer_width = "32")]
        (RelocationKind::Relative, RelocationEncoding::X86Branch, 32) => unsafe {
            let reloc_address = body.add(offset as usize) as usize;
            let reloc_addend = r.addend() as isize;
            let reloc_delta_u32 = (target_func_address as u32)
                .wrapping_sub(reloc_address as u32)
                .wrapping_add(reloc_addend as u32);
            write_unaligned(reloc_address as *mut u32, reloc_delta_u32);
        },
        #[cfg(target_pointer_width = "64")]
        (RelocationKind::Relative, RelocationEncoding::X86Branch, 32) => unsafe {
            let reloc_address = body.add(offset as usize) as usize;
            let reloc_addend = r.addend() as isize;
            let reloc_delta_u64 = (target_func_address as u64)
                .wrapping_sub(reloc_address as u64)
                .wrapping_add(reloc_addend as u64);
            assert!(
                reloc_delta_u64 as isize <= i32::max_value() as isize,
                "relocation too large to fit in i32"
            );
            write_unaligned(reloc_address as *mut u32, reloc_delta_u64 as u32);
        },
        (RelocationKind::Elf(elf::R_AARCH64_CALL26), RelocationEncoding::Generic, 32) => unsafe {
            let reloc_address = body.add(offset as usize) as usize;
            let reloc_addend = r.addend() as isize;
            let reloc_delta = (target_func_address as u64).wrapping_sub(reloc_address as u64);
            // TODO: come up with a PLT-like solution for longer calls. We can't extend the
            // code segment at this point, but we could conservatively allocate space at the
            // end of the function during codegen, a fixed amount per call, to allow for
            // potential branch islands.
            assert!((reloc_delta as i64) < (1 << 27));
            assert!((reloc_delta as i64) >= -(1 << 27));
            let reloc_delta = reloc_delta as u32;
            let reloc_delta = reloc_delta.wrapping_add(reloc_addend as u32);
            let delta_bits = reloc_delta >> 2;
            let insn = read_unaligned(reloc_address as *const u32);
            let new_insn = (insn & 0xfc00_0000) | (delta_bits & 0x03ff_ffff);
            write_unaligned(reloc_address as *mut u32, new_insn);
        },
        other => panic!("unsupported reloc kind: {:?}", other),
    }
}

fn to_libcall_address(name: &str) -> usize {
    use self::libcalls::*;
    match name {
        "wasmtime_i64_udiv" => wasmtime_i64_udiv as usize,
        "wasmtime_i64_sdiv" => wasmtime_i64_sdiv as usize,
        "wasmtime_i64_urem" => wasmtime_i64_urem as usize,
        "wasmtime_i64_srem" => wasmtime_i64_srem as usize,
        "wasmtime_i64_ishl" => wasmtime_i64_ishl as usize,
        "wasmtime_i64_ushr" => wasmtime_i64_ushr as usize,
        "wasmtime_i64_sshr" => wasmtime_i64_sshr as usize,
        "wasmtime_f32_ceil" => wasmtime_f32_ceil as usize,
        "wasmtime_f32_floor" => wasmtime_f32_floor as usize,
        "wasmtime_f32_trunc" => wasmtime_f32_trunc as usize,
        "wasmtime_f32_nearest" => wasmtime_f32_nearest as usize,
        "wasmtime_f64_ceil" => wasmtime_f64_ceil as usize,
        "wasmtime_f64_floor" => wasmtime_f64_floor as usize,
        "wasmtime_f64_trunc" => wasmtime_f64_trunc as usize,
        "wasmtime_f64_nearest" => wasmtime_f64_nearest as usize,
        other => panic!("unexpected libcall: {}", other),
    }
}
