//! Support for the component model in Wasmtime.
//!
//! This module contains all of the internal type definitions used by Wasmtime
//! to process the component model. Despite everything being `pub` here this is
//! not the public interface of Wasmtime to the component model. Instead this is
//! the internal support to mirror the core wasm support that Wasmtime already
//! implements.
//!
//! Some main items contained within here are:
//!
//! * Type hierarchy information for the component model
//! * Translation of a component into Wasmtime's representation
//! * Type information about a component used at runtime
//!
//! This module also contains a lot of Serialize/Deserialize types which are
//! encoded in the final compiled image for a component.
//!
//! Note that this entire module is gated behind the `component-model` Cargo
//! feature.
//!
//! ## Warning: In-progress
//!
//! As-of the time of this writing this module is incomplete and under
//! development. It will be added to incrementally over time as more features
//! are implemented. Current design decisions are also susceptible to change at
//! any time. Some comments may reflect historical rather than current state as
//! well (sorry).

/// Canonical ABI-defined constant for the maximum number of "flat" parameters
/// to a wasm function, or the maximum number of parameters a core wasm function
/// will take for just the parameters used. Over this number the heap is used
/// for transferring parameters.
pub const MAX_FLAT_PARAMS: usize = 16;

/// Canonical ABI-defined constant for the maximum number of "flat" results.
/// This number of results are returned directly from wasm and otherwise results
/// are transferred through memory.
pub const MAX_FLAT_RESULTS: usize = 1;

mod artifacts;
mod info;
mod names;
mod types;
mod vmcomponent_offsets;
pub use self::artifacts::*;
pub use self::info::*;
pub use self::names::*;
pub use self::types::*;
pub use self::vmcomponent_offsets::*;

#[cfg(feature = "compile")]
mod compiler;
#[cfg(feature = "compile")]
pub mod dfg;
#[cfg(feature = "compile")]
mod translate;
#[cfg(feature = "compile")]
mod types_builder;
#[cfg(feature = "compile")]
pub use self::compiler::*;
#[cfg(feature = "compile")]
pub use self::translate::*;
#[cfg(feature = "compile")]
pub use self::types_builder::*;

/// Helper macro, like `foreach_transcoder`, to iterate over builtins for
/// components unrelated to transcoding.
#[macro_export]
macro_rules! foreach_builtin_component_function {
    ($mac:ident) => {
        $mac! {
            resource_new32(vmctx: vmctx, resource: u32, rep: u32) -> u64;
            resource_rep32(vmctx: vmctx, resource: u32, idx: u32) -> u64;

            // Returns an `Option<u32>` where `None` is "no destructor needed"
            // and `Some(val)` is "run the destructor on this rep". The option
            // is encoded as a 64-bit integer where the low bit is Some/None
            // and bits 1-33 are the payload.
            resource_drop(vmctx: vmctx, resource: u32, idx: u32) -> u64;

            resource_transfer_own(vmctx: vmctx, src_idx: u32, src_table: u32, dst_table: u32) -> u64;
            resource_transfer_borrow(vmctx: vmctx, src_idx: u32, src_table: u32, dst_table: u32) -> u64;
            resource_enter_call(vmctx: vmctx);
            resource_exit_call(vmctx: vmctx) -> bool;

            #[cfg(feature = "component-model-async")]
            task_backpressure(vmctx: vmctx, caller_instance: u32, enabled: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            task_return(vmctx: vmctx, ty: u32, storage: ptr_u8, storage_len: size) -> bool;
            #[cfg(feature = "component-model-async")]
            task_wait(vmctx: vmctx, caller_instance: u32, async_: u8, memory: ptr_u8, payload: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            task_poll(vmctx: vmctx, caller_instance: u32, async_: u8, memory: ptr_u8, payload: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            task_yield(vmctx: vmctx, async_: u8) -> bool;
            #[cfg(feature = "component-model-async")]
            subtask_drop(vmctx: vmctx, caller_instance: u32, task_id: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            sync_enter(vmctx: vmctx, start: ptr_u8, return_: ptr_u8, caller_instance: u32, task_return_type: u32, result_count: u32, storage: ptr_u8, storage_len: size) -> bool;
            #[cfg(feature = "component-model-async")]
            sync_exit(vmctx: vmctx, callback: ptr_u8, caller_instance: u32, callee: ptr_u8, callee_instance: u32, param_count: u32, storage: ptr_u8, storage_len: size) -> bool;
            #[cfg(feature = "component-model-async")]
            async_enter(vmctx: vmctx, start: ptr_u8, return_: ptr_u8, caller_instance: u32, task_return_type: u32, params: u32, results: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            async_exit(vmctx: vmctx, callback: ptr_u8, post_return: ptr_u8, caller_instance: u32, callee: ptr_u8, callee_instance: u32, param_count: u32, result_count: u32, flags: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_new(vmctx: vmctx, ty: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_write(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, future: u32, address: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_read(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, future: u32, address: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_cancel_write(vmctx: vmctx, ty: u32, async_: u8, writer: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_cancel_read(vmctx: vmctx, ty: u32, async_: u8, reader: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            future_close_writable(vmctx: vmctx, ty: u32, writer: u32, error: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            future_close_readable(vmctx: vmctx, ty: u32, reader: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            stream_new(vmctx: vmctx, ty: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_write(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, stream: u32, address: u32, count: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_read(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, stream: u32, address: u32, count: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_cancel_write(vmctx: vmctx, ty: u32, async_: u8, writer: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_cancel_read(vmctx: vmctx, ty: u32, async_: u8, reader: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_close_writable(vmctx: vmctx, ty: u32, writer: u32, error: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            stream_close_readable(vmctx: vmctx, ty: u32, reader: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            flat_stream_write(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, ty: u32, payload_size: u32, payload_align: u32, stream: u32, address: u32, count: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            flat_stream_read(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, ty: u32, payload_size: u32, payload_align: u32, stream: u32, address: u32, count: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            error_context_new(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, debug_msg_address: u32, debug_msg_len: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            error_context_debug_message(vmctx: vmctx, memory: ptr_u8, realloc: ptr_u8, string_encoding: u8, ty: u32, err_ctx_handle: u32, debug_msg_address: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            error_context_drop(vmctx: vmctx, ty: u32, err_ctx_handle: u32) -> bool;
            #[cfg(feature = "component-model-async")]
            future_transfer(vmctx: vmctx, src_idx: u32, src_table: u32, dst_table: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            stream_transfer(vmctx: vmctx, src_idx: u32, src_table: u32, dst_table: u32) -> u64;
            #[cfg(feature = "component-model-async")]
            error_context_transfer(vmctx: vmctx, src_idx: u32, src_table: u32, dst_table: u32) -> u64;

            trap(vmctx: vmctx, code: u8);

            utf8_to_utf8(src: ptr_u8, len: size, dst: ptr_u8) -> bool;
            utf16_to_utf16(src: ptr_u16, len: size, dst: ptr_u16) -> bool;
            latin1_to_latin1(src: ptr_u8, len: size, dst: ptr_u8) -> bool;
            latin1_to_utf16(src: ptr_u8, len: size, dst: ptr_u16) -> bool;
            utf8_to_utf16(src: ptr_u8, len: size, dst: ptr_u16) -> size;
            utf16_to_utf8(src: ptr_u16, src_len: size, dst: ptr_u8, dst_len: size, ret2: ptr_size) -> size;
            latin1_to_utf8(src: ptr_u8, src_len: size, dst: ptr_u8, dst_len: size, ret2: ptr_size) -> size;
            utf16_to_compact_probably_utf16(src: ptr_u16, len: size, dst: ptr_u16) -> size;
            utf8_to_latin1(src: ptr_u8, len: size, dst: ptr_u8, ret2: ptr_size) -> size;
            utf16_to_latin1(src: ptr_u16, len: size, dst: ptr_u8, ret2: ptr_size) -> size;
            utf8_to_compact_utf16(src: ptr_u8, src_len: size, dst: ptr_u16, dst_len: size, bytes_so_far: size) -> size;
            utf16_to_compact_utf16(src: ptr_u16, src_len: size, dst: ptr_u16, dst_len: size, bytes_so_far: size) -> size;
        }
    };
}

// Define `struct ComponentBuiltinFunctionIndex`
declare_builtin_index!(
    ComponentBuiltinFunctionIndex,
    foreach_builtin_component_function
);
