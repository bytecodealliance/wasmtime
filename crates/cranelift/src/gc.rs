//! Interface to compiling GC-related things.
//!
//! This module and its interface are implemented twice: once when the `gc`
//! cargo feature is enabled and once when the feature is disabled. The goal is
//! to have just a single `cfg(feature = "gc")` for the whole crate, which
//! selects between these two implementations.

use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{WasmHeapType, WasmRefType, WasmResult, WasmValType};

#[cfg(feature = "gc")]
mod enabled;
#[cfg(feature = "gc")]
use enabled as imp;

#[cfg(not(feature = "gc"))]
mod disabled;
#[cfg(not(feature = "gc"))]
use disabled as imp;

/// Get the GC compiler configured for the given function environment.
pub fn gc_compiler(func_env: &FuncEnvironment<'_>) -> Box<dyn GcCompiler> {
    imp::gc_compiler(func_env)
}

/// Load a `*mut VMGcRef` into a virtual register, without any GC barriers.
///
/// The resulting value is an instance of the function environment's type for
/// GC-managed references, aka `i32`. Note that a `VMGcRef` is always 4-bytes
/// large, even when targeting 64-bit architectures.
pub fn unbarriered_load_gc_ref(
    func_env: &FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    ty: WasmHeapType,
    src: ir::Value,
    flags: ir::MemFlags,
) -> WasmResult<ir::Value> {
    imp::unbarriered_load_gc_ref(func_env, builder, ty, src, flags)
}

/// Store `*dst = gc_ref`, without any GC barriers.
///
/// `dst` is a `*mut VMGcRef`.
///
/// `gc_ref` is an instance of the function environment's type for GC-managed
/// references, aka `i32`. Note that a `VMGcRef` is always 4-bytes large, even
/// when targeting 64-bit architectures.
pub fn unbarriered_store_gc_ref(
    func_env: &FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    ty: WasmHeapType,
    dst: ir::Value,
    gc_ref: ir::Value,
    flags: ir::MemFlags,
) -> WasmResult<()> {
    imp::unbarriered_store_gc_ref(func_env, builder, ty, dst, gc_ref, flags)
}

/// Get the index and signature of the built-in function for doing `table.grow`
/// on GC reference tables.
pub fn gc_ref_table_grow_builtin(
    ty: WasmHeapType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    imp::gc_ref_table_grow_builtin(ty, func_env, func)
}

/// Get the index and signature of the built-in function for doing `table.fill`
/// on GC reference tables.
pub fn gc_ref_table_fill_builtin(
    ty: WasmHeapType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    imp::gc_ref_table_fill_builtin(ty, func_env, func)
}

/// Get the index and signature of the built-in function for doing `global.get`
/// on a GC reference global.
pub fn gc_ref_global_get_builtin(
    ty: WasmValType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    imp::gc_ref_global_get_builtin(ty, func_env, func)
}

/// Get the index and signature of the built-in function for doing `global.set`
/// on a GC reference global.
pub fn gc_ref_global_set_builtin(
    ty: WasmValType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    imp::gc_ref_global_set_builtin(ty, func_env, func)
}

/// A trait for different collectors to emit any GC barriers they might require.
pub trait GcCompiler {
    /// Emit a read barrier for when we are cloning a GC reference onto the Wasm
    /// stack.
    ///
    /// This is used, for example, when reading from a global or a table
    /// element.
    ///
    /// In pseudocode, this is the following operation:
    ///
    /// ```ignore
    /// x = *src;
    /// ```
    ///
    /// Parameters:
    ///
    /// * `func_env`: The function environment that this GC compiler is
    ///   operating within.
    ///
    /// * `builder`: Function builder. Currently at the position where the read
    ///   should be inserted. Upon return, should be positioned where control
    ///   continues just after the read completes. Any intermediate blocks
    ///   created in the process of emitting the read barrier should be added to
    ///   the layout and sealed.
    ///
    /// * `ty`: The Wasm reference type that is being read.
    ///
    /// * `src`: A pointer to the GC reference that should be read; this is an
    ///   instance of a `*mut Option<VMGcRef>`.
    ///
    /// * `flags`: The memory flags that should be used when accessing `src`.
    ///
    /// This method should return the cloned GC reference (an instance of
    /// `VMGcRef`) of type `i32`.
    fn translate_read_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        src: ir::Value,
        flags: ir::MemFlags,
    ) -> WasmResult<ir::Value>;

    /// Emit a write barrier for when we are writing a GC reference over another
    /// GC reference.
    ///
    /// This is used, for example, when writing to a global or a table element.
    ///
    /// In pseudocode, this is the following operation:
    ///
    /// ```ignore
    /// *dst = new_val;
    /// ```
    ///
    /// Parameters:
    ///
    /// * `func_env`: The function environment that this GC compiler is
    ///   operating within.
    ///
    /// * `builder`: Function builder. Currently at the position where the write
    ///   should be inserted. Upon return, should be positioned where control
    ///   continues just after the write completes. Any intermediate blocks
    ///   created in the process of emitting the read barrier should be added to
    ///   the layout and sealed.
    ///
    /// * `ty`: The Wasm reference type that is being written.
    ///
    /// * `dst`: A pointer to the GC reference that will be overwritten; note
    ///   that is this is an instance of a `*mut VMGcRef`, *not* a `VMGcRef`
    ///   itself or a `*mut VMGcHeader`!
    ///
    /// * `new_val`: The new value that should be written into `dst`. This is a
    ///   `VMGcRef` of Cranelift type `i32`; not a `*mut VMGcRef`.
    ///
    /// * `flags`: The memory flags that should be used when accessing `dst`.
    fn translate_write_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        dst: ir::Value,
        new_val: ir::Value,
        flags: ir::MemFlags,
    ) -> WasmResult<()>;
}
