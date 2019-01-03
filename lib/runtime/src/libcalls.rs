//! Runtime library calls. Note that wasm compilers may sometimes perform these
//! inline rather than calling them, particularly when CPUs have special
//! instructions which compute them directly.

use cranelift_wasm::{DefinedMemoryIndex, MemoryIndex};
use vmcontext::VMContext;

/// Implementation of f32.ceil
pub extern "C" fn wasmtime_f32_ceil(x: f32) -> f32 {
    x.ceil()
}

/// Implementation of f32.floor
pub extern "C" fn wasmtime_f32_floor(x: f32) -> f32 {
    x.floor()
}

/// Implementation of f32.trunc
pub extern "C" fn wasmtime_f32_trunc(x: f32) -> f32 {
    x.trunc()
}

/// Implementation of f32.nearest
#[allow(clippy::float_arithmetic, clippy::float_cmp)]
pub extern "C" fn wasmtime_f32_nearest(x: f32) -> f32 {
    // Rust doesn't have a nearest function, so do it manually.
    if x == 0.0 {
        // Preserve the sign of zero.
        x
    } else {
        // Nearest is either ceil or floor depending on which is nearest or even.
        let u = x.ceil();
        let d = x.floor();
        let um = (x - u).abs();
        let dm = (x - d).abs();
        if um < dm
            || (um == dm && {
                let h = u / 2.;
                h.floor() == h
            })
        {
            u
        } else {
            d
        }
    }
}

/// Implementation of f64.ceil
pub extern "C" fn wasmtime_f64_ceil(x: f64) -> f64 {
    x.ceil()
}

/// Implementation of f64.floor
pub extern "C" fn wasmtime_f64_floor(x: f64) -> f64 {
    x.floor()
}

/// Implementation of f64.trunc
pub extern "C" fn wasmtime_f64_trunc(x: f64) -> f64 {
    x.trunc()
}

/// Implementation of f64.nearest
#[allow(clippy::float_arithmetic, clippy::float_cmp)]
pub extern "C" fn wasmtime_f64_nearest(x: f64) -> f64 {
    // Rust doesn't have a nearest function, so do it manually.
    if x == 0.0 {
        // Preserve the sign of zero.
        x
    } else {
        // Nearest is either ceil or floor depending on which is nearest or even.
        let u = x.ceil();
        let d = x.floor();
        let um = (x - u).abs();
        let dm = (x - d).abs();
        if um < dm
            || (um == dm && {
                let h = u / 2.;
                h.floor() == h
            })
        {
            u
        } else {
            d
        }
    }
}

/// Implementation of memory.grow for locally-defined 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_memory32_grow(
    delta: u32,
    memory_index: u32,
    vmctx: *mut VMContext,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);

    instance
        .memory_grow(memory_index, delta)
        .unwrap_or(u32::max_value())
}

/// Implementation of memory.grow for imported 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_imported_memory32_grow(
    delta: u32,
    memory_index: u32,
    vmctx: *mut VMContext,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    assert!(
        (memory_index as usize) < instance.num_imported_memories(),
        "imported memory index for memory.grow out of bounds"
    );

    let memory_index = MemoryIndex::from_u32(memory_index);
    let import = instance.vmctx().imported_memory(memory_index);
    let foreign_instance = (&mut *import.vmctx).instance();
    let foreign_memory = &mut *import.from;
    let foreign_index = foreign_instance.vmctx().memory_index(foreign_memory);

    foreign_instance
        .memory_grow(foreign_index, delta)
        .unwrap_or(u32::max_value())
}

/// Implementation of memory.size for locally-defined 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_memory32_size(memory_index: u32, vmctx: *mut VMContext) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);

    instance.memory_size(memory_index)
}

/// Implementation of memory.size for imported 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_imported_memory32_size(
    memory_index: u32,
    vmctx: *mut VMContext,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    assert!(
        (memory_index as usize) < instance.num_imported_memories(),
        "imported memory index for memory.grow out of bounds"
    );

    let memory_index = MemoryIndex::from_u32(memory_index);
    let import = instance.vmctx().imported_memory(memory_index);
    let foreign_instance = (&mut *import.vmctx).instance();
    let foreign_memory = &mut *import.from;
    let foreign_index = foreign_instance.vmctx().memory_index(foreign_memory);

    foreign_instance.memory_size(foreign_index)
}
