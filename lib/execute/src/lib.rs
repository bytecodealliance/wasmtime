//! JIT-style runtime for WebAssembly using Cretonne.

#![deny(missing_docs)]

extern crate cretonne;
extern crate cton_wasm;
extern crate region;
extern crate wasmstandalone_runtime;

use cretonne::isa::TargetIsa;
use std::mem::transmute;
use region::Protection;
use region::protect;
use std::ptr::write_unaligned;
use wasmstandalone_runtime::Compilation;

/// Executes a module that has been translated with the `standalone::Runtime` runtime implementation.
pub fn compile_module<'data, 'module>(
    isa: &TargetIsa,
    translation: &wasmstandalone_runtime::ModuleTranslation<'data, 'module>,
) -> Result<wasmstandalone_runtime::Compilation<'module>, String> {
    debug_assert!(
        translation.module.start_func.is_none() ||
            translation.module.start_func.unwrap() >= translation.module.imported_funcs.len(),
        "imported start functions not supported yet"
    );

    let (mut compilation, relocations) = translation.compile(isa)?;

    // Apply relocations, now that we have virtual addresses for everything.
    relocate(&mut compilation, &relocations);

    Ok(compilation)
}

/// Performs the relocations inside the function bytecode, provided the necessary metadata
fn relocate(compilation: &mut Compilation, relocations: &wasmstandalone_runtime::Relocations) {
    // The relocations are relative to the relocation's address plus four bytes
    // TODO: Support architectures other than x64, and other reloc kinds.
    for (i, function_relocs) in relocations.iter().enumerate() {
        for &(_reloc, func_index, offset) in function_relocs {
            let target_func_address: isize = compilation.functions[func_index].as_ptr() as isize;
            let body = &mut compilation.functions[i];
            unsafe {
                let reloc_address: isize = body.as_mut_ptr().offset(offset as isize + 4) as isize;
                let reloc_delta_i32: i32 = (target_func_address - reloc_address) as i32;
                write_unaligned(reloc_address as *mut i32, reloc_delta_i32);
            }
        }
    }
}

/// Jumps to the code region of memory and execute the start function of the module.
pub fn execute(
    compilation: &wasmstandalone_runtime::Compilation,
    _instance: &wasmstandalone_runtime::Instance,
) -> Result<(), String> {
    let start_index = compilation.module.start_func.ok_or_else(|| {
        String::from("No start function defined, aborting execution")
    })?;
    let code_buf = &compilation.functions[start_index];
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
                err.description()
            ))
        }
    }
    // Rather than writing inline assembly to jump to the code region, we use the fact that
    // the Rust ABI for calling a function with no arguments and no return matches the one of
    // the generated code.Thanks to this, we can transmute the code region into a first-class
    // Rust function and call it.
    unsafe {
        let start_func = transmute::<_, fn()>(code_buf.as_ptr());
        start_func();
    }
    Ok(())
}
