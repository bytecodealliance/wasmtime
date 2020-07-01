//! Runtime library calls.
//!
//! Note that Wasm compilers may sometimes perform these inline rather than
//! calling them, particularly when CPUs have special instructions which compute
//! them directly.
//!
//! These functions are called by compiled Wasm code, and therefore must take
//! certain care about some things:
//!
//! * They must always be `pub extern "C"` and should only contain basic, raw
//!   i32/i64/f32/f64/pointer parameters that are safe to pass across the system
//!   ABI!
//!
//! * If any nested function propagates an `Err(trap)` out to the library
//!   function frame, we need to raise it. This involves some nasty and quite
//!   unsafe code under the covers! Notable, after raising the trap, drops
//!   **will not** be run for local variables! This can lead to things like
//!   leaking `InstanceHandle`s which leads to never deallocating JIT code,
//!   instances, and modules! Therefore, always use nested blocks to ensure
//!   drops run before raising a trap:
//!
//!   ```ignore
//!   pub extern "C" fn my_lib_function(...) {
//!       let result = {
//!           // Do everything in here so drops run at the end of the block.
//!           ...
//!       };
//!       if let Err(trap) = result {
//!           // Now we can safely raise the trap without leaking!
//!           raise_lib_trap(trap);
//!       }
//!   }
//!   ```
//!
//! * When receiving a raw `*mut u8` that is actually a `VMExternRef` reference,
//!   convert it into a proper `VMExternRef` with `VMExternRef::clone_from_raw`
//!   as soon as apossible. Any GC before raw pointer is converted into a
//!   reference can potentially collect the referenced object, which could lead
//!   to use after free. Avoid this by eagerly converting into a proper
//!   `VMExternRef`!
//!
//!   ```ignore
//!   pub unsafe extern "C" my_lib_takes_ref(raw_extern_ref: *mut u8) {
//!       // Before `clone_from_raw`, `raw_extern_ref` is potentially unrooted,
//!       // and doing GC here could lead to use after free!
//!
//!       let my_extern_ref = if raw_extern_ref.is_null() {
//!           None
//!       } else {
//!           Some(VMExternRef::clone_from_raw(raw_extern_ref))
//!       };
//!
//!       // Now that we did `clone_from_raw`, it is safe to do a GC (or do
//!       // anything else that might transitively GC, like call back into
//!       // Wasm!)
//!   }
//!   ```

use crate::externref::VMExternRef;
use crate::table::Table;
use crate::traphandlers::raise_lib_trap;
use crate::vmcontext::{VMCallerCheckedAnyfunc, VMContext};
use wasmtime_environ::wasm::{
    DataIndex, DefinedMemoryIndex, ElemIndex, MemoryIndex, TableElementType, TableIndex,
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

/// Implementation of i64.udiv
pub extern "C" fn wasmtime_i64_udiv(x: u64, y: u64) -> u64 {
    x / y
}

/// Implementation of i64.sdiv
pub extern "C" fn wasmtime_i64_sdiv(x: i64, y: i64) -> i64 {
    x / y
}

/// Implementation of i64.urem
pub extern "C" fn wasmtime_i64_urem(x: u64, y: u64) -> u64 {
    x % y
}

/// Implementation of i64.srem
pub extern "C" fn wasmtime_i64_srem(x: i64, y: i64) -> i64 {
    x % y
}

/// Implementation of i64.ishl
pub extern "C" fn wasmtime_i64_ishl(x: i64, y: i64) -> i64 {
    x << y
}

/// Implementation of i64.ushr
pub extern "C" fn wasmtime_i64_ushr(x: u64, y: i64) -> u64 {
    x >> y
}

/// Implementation of i64.sshr
pub extern "C" fn wasmtime_i64_sshr(x: i64, y: i64) -> i64 {
    x >> y
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
pub unsafe extern "C" fn wasmtime_memory32_size(vmctx: *mut VMContext, memory_index: u32) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);

    instance.memory_size(memory_index)
}

/// Implementation of memory.size for imported 32-bit memories.
pub unsafe extern "C" fn wasmtime_imported_memory32_size(
    vmctx: *mut VMContext,
    memory_index: u32,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    let memory_index = MemoryIndex::from_u32(memory_index);

    instance.imported_memory_size(memory_index)
}

/// Implementation of `table.grow`.
pub unsafe extern "C" fn wasmtime_table_grow(
    vmctx: *mut VMContext,
    table_index: u32,
    delta: u32,
    // NB: we don't know whether this is a pointer to a `VMCallerCheckedAnyfunc`
    // or is a `VMExternRef` until we look at the table type.
    init_value: *mut u8,
) -> u32 {
    let instance = (&mut *vmctx).instance();
    let table_index = TableIndex::from_u32(table_index);
    match instance.table_element_type(table_index) {
        TableElementType::Func => {
            let func = init_value as *mut VMCallerCheckedAnyfunc;
            instance
                .table_grow(table_index, delta, func.into())
                .unwrap_or(-1_i32 as u32)
        }
        TableElementType::Val(ty) => {
            debug_assert_eq!(ty, crate::ref_type());

            let init_value = if init_value.is_null() {
                None
            } else {
                Some(VMExternRef::clone_from_raw(init_value))
            };

            instance
                .table_grow(table_index, delta, init_value.into())
                .unwrap_or(-1_i32 as u32)
        }
    }
}

/// Implementation of `table.copy`.
pub unsafe extern "C" fn wasmtime_table_copy(
    vmctx: *mut VMContext,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) {
    let result = {
        let dst_table_index = TableIndex::from_u32(dst_table_index);
        let src_table_index = TableIndex::from_u32(src_table_index);
        let instance = (&mut *vmctx).instance();
        let dst_table = instance.get_table(dst_table_index);
        let src_table = instance.get_table(src_table_index);
        Table::copy(dst_table, src_table, dst, src, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `table.init`.
pub unsafe extern "C" fn wasmtime_table_init(
    vmctx: *mut VMContext,
    table_index: u32,
    elem_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) {
    let result = {
        let table_index = TableIndex::from_u32(table_index);
        let elem_index = ElemIndex::from_u32(elem_index);
        let instance = (&mut *vmctx).instance();
        instance.table_init(table_index, elem_index, dst, src, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `elem.drop`.
pub unsafe extern "C" fn wasmtime_elem_drop(vmctx: *mut VMContext, elem_index: u32) {
    let elem_index = ElemIndex::from_u32(elem_index);
    let instance = (&mut *vmctx).instance();
    instance.elem_drop(elem_index);
}

/// Implementation of `memory.copy` for locally defined memories.
pub unsafe extern "C" fn wasmtime_defined_memory_copy(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) {
    let result = {
        let memory_index = DefinedMemoryIndex::from_u32(memory_index);
        let instance = (&mut *vmctx).instance();
        instance.defined_memory_copy(memory_index, dst, src, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `memory.copy` for imported memories.
pub unsafe extern "C" fn wasmtime_imported_memory_copy(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) {
    let result = {
        let memory_index = MemoryIndex::from_u32(memory_index);
        let instance = (&mut *vmctx).instance();
        instance.imported_memory_copy(memory_index, dst, src, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `memory.fill` for locally defined memories.
pub unsafe extern "C" fn wasmtime_memory_fill(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    val: u32,
    len: u32,
) {
    let result = {
        let memory_index = DefinedMemoryIndex::from_u32(memory_index);
        let instance = (&mut *vmctx).instance();
        instance.defined_memory_fill(memory_index, dst, val, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `memory.fill` for imported memories.
pub unsafe extern "C" fn wasmtime_imported_memory_fill(
    vmctx: *mut VMContext,
    memory_index: u32,
    dst: u32,
    val: u32,
    len: u32,
) {
    let result = {
        let memory_index = MemoryIndex::from_u32(memory_index);
        let instance = (&mut *vmctx).instance();
        instance.imported_memory_fill(memory_index, dst, val, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `memory.init`.
pub unsafe extern "C" fn wasmtime_memory_init(
    vmctx: *mut VMContext,
    memory_index: u32,
    data_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) {
    let result = {
        let memory_index = MemoryIndex::from_u32(memory_index);
        let data_index = DataIndex::from_u32(data_index);
        let instance = (&mut *vmctx).instance();
        instance.memory_init(memory_index, data_index, dst, src, len)
    };
    if let Err(trap) = result {
        raise_lib_trap(trap);
    }
}

/// Implementation of `data.drop`.
pub unsafe extern "C" fn wasmtime_data_drop(vmctx: *mut VMContext, data_index: u32) {
    let data_index = DataIndex::from_u32(data_index);
    let instance = (&mut *vmctx).instance();
    instance.data_drop(data_index)
}
