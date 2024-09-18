//! Interface to compiling GC-related things.
//!
//! This module and its interface are implemented twice: once when the `gc`
//! cargo feature is enabled and once when the feature is disabled. The goal is
//! to have just a single `cfg(feature = "gc")` for the whole crate, which
//! selects between these two implementations.

use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{TypeIndex, WasmRefType, WasmResult};
use wasmtime_environ::GcTypeLayouts;

#[cfg(feature = "gc")]
mod enabled;
#[cfg(feature = "gc")]
use enabled as imp;

#[cfg(not(feature = "gc"))]
mod disabled;
#[cfg(not(feature = "gc"))]
use disabled as imp;

// Re-export the GC compilation interface from the implementation that we chose
// based on the compile-time features enabled.
pub use imp::*;

/// A trait for different collectors to emit any GC barriers they might require.
pub trait GcCompiler {
    /// Get the GC type layouts for this GC compiler.
    #[cfg_attr(not(feature = "gc"), allow(dead_code))]
    fn layouts(&self) -> &dyn GcTypeLayouts;

    /// Emit code to allocate a new struct.
    ///
    /// The struct should be of the given type and its fields initialized to the
    /// given values.
    #[cfg_attr(not(feature = "gc"), allow(dead_code))]
    fn alloc_struct(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        struct_type_index: TypeIndex,
        fields: &[ir::Value],
    ) -> WasmResult<ir::Value>;

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
