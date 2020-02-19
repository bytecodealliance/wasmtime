//! Runtime library calls. Note that wasm compilers may sometimes perform these
//! inline rather than calling them, particularly when CPUs have special
//! instructions which compute them directly.

use crate::table::Table;
use crate::traphandlers::raise_lib_trap;
use crate::vmcontext::VMContext;
use wasmtime_environ::ir;
use wasmtime_environ::wasm::{
    DefinedMemoryIndex, DefinedTableIndex, MemoryIndex, PassiveElemIndex, TableIndex,
};

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
    vmctx: *mut VMContext,
    delta: u32,
    memory_index: u32,
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
    vmctx: *mut VMContext,
    delta: u32,
    memory_index: u32,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = MemoryIndex::from_u32(memory_index);

    instance
        .imported_memory_grow(memory_index, delta)
        .unwrap_or(u32::max_value())
}

/// Implementation of memory.size for locally-defined 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_memory32_size(vmctx: *mut VMContext, memory_index: u32) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);

    instance.memory_size(memory_index)
}

/// Implementation of memory.size for imported 32-bit memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_imported_memory32_size(
    vmctx: *mut VMContext,
    memory_index: u32,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = MemoryIndex::from_u32(memory_index);

    instance.imported_memory_size(memory_index)
}

/// Implementation of `table.copy` when both tables are locally defined.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_table_copy_defined_defined(
    vmctx: *mut VMContext,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let dst_table_index = DefinedTableIndex::from_u32(dst_table_index);
    let src_table_index = DefinedTableIndex::from_u32(src_table_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    let dst_table = instance.get_defined_table(dst_table_index);
    let src_table = instance.get_defined_table(src_table_index);
    if let Err(trap) = Table::copy(dst_table, src_table, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `table.copy` when the destination table is locally defined
/// and the source table is imported.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_table_copy_defined_imported(
    vmctx: *mut VMContext,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let dst_table_index = DefinedTableIndex::from_u32(dst_table_index);
    let src_table_index = TableIndex::from_u32(src_table_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    let dst_table = instance.get_defined_table(dst_table_index);
    let src_table = instance.get_foreign_table(src_table_index);
    if let Err(trap) = Table::copy(dst_table, src_table, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `table.copy` when the destination table is imported
/// and the source table is locally defined.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_table_copy_imported_defined(
    vmctx: *mut VMContext,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let dst_table_index = TableIndex::from_u32(dst_table_index);
    let src_table_index = DefinedTableIndex::from_u32(src_table_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    let dst_table = instance.get_foreign_table(dst_table_index);
    let src_table = instance.get_defined_table(src_table_index);
    if let Err(trap) = Table::copy(dst_table, src_table, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `table.copy` when both tables are imported.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_table_copy_imported_imported(
    vmctx: *mut VMContext,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let dst_table_index = TableIndex::from_u32(dst_table_index);
    let src_table_index = TableIndex::from_u32(src_table_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    let dst_table = instance.get_foreign_table(dst_table_index);
    let src_table = instance.get_foreign_table(src_table_index);
    if let Err(trap) = Table::copy(dst_table, src_table, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `table.init`.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_table_init(
    vmctx: *mut VMContext,
    table_index: u32,
    elem_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let table_index = TableIndex::from_u32(table_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let elem_index = PassiveElemIndex::from_u32(elem_index);

    let instance = (&mut *vmctx).instance();

    if let Err(trap) = instance.table_init(table_index, elem_index, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `elem.drop`.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_elem_drop(vmctx: *mut VMContext, elem_index: u32) {
    let elem_index = PassiveElemIndex::from_u32(elem_index);
    let instance = (&mut *vmctx).instance();
    instance.elem_drop(elem_index);
}

/// Implementation of `memory.copy` for locally defined memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_memory_copy(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    if let Err(trap) = instance.defined_memory_copy(memory_index, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}

/// Implementation of `memory.copy` for imported memories.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_imported_memory_copy(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    src: u32,
    len: u32,
    source_loc: u32,
) {
    let memory_index = MemoryIndex::from_u32(memory_index);
    let source_loc = ir::SourceLoc::new(source_loc);
    let instance = (&mut *vmctx).instance();
    if let Err(trap) = instance.imported_memory_copy(memory_index, dst, src, len, source_loc) {
        raise_lib_trap(trap);
    }
}
