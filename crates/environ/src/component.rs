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

mod compiler;
pub mod dfg;
mod info;
mod translate;
mod types;
mod vmcomponent_offsets;
pub use self::compiler::*;
pub use self::info::*;
pub use self::translate::*;
pub use self::types::*;
pub use self::vmcomponent_offsets::*;

/// Helper macro to iterate over the transcoders that the host will provide
/// adapter modules through libcalls.
#[macro_export]
macro_rules! foreach_transcoder {
    ($mac:ident) => {
        $mac! {
            utf8_to_utf8(src: ptr_u8, len: size, dst: ptr_u8);
            utf16_to_utf16(src: ptr_u16, len: size, dst: ptr_u16);
            latin1_to_latin1(src: ptr_u8, len: size, dst: ptr_u8);
            latin1_to_utf16(src: ptr_u8, len: size, dst: ptr_u16);
            utf8_to_utf16(src: ptr_u8, len: size, dst: ptr_u16) -> size;
            utf16_to_utf8(src: ptr_u16, src_len: size, dst: ptr_u8, dst_len: size) -> size_pair;
            latin1_to_utf8(src: ptr_u8, src_len: size, dst: ptr_u8, dst_len: size) -> size_pair;
            utf16_to_compact_probably_utf16(src: ptr_u16, len: size, dst: ptr_u16) -> size;
            utf8_to_latin1(src: ptr_u8, len: size, dst: ptr_u8) -> size_pair;
            utf16_to_latin1(src: ptr_u16, len: size, dst: ptr_u8) -> size_pair;
            utf8_to_compact_utf16(src: ptr_u8, src_len: size, dst: ptr_u16, dst_len: size, bytes_so_far: size) -> size;
            utf16_to_compact_utf16(src: ptr_u16, src_len: size, dst: ptr_u16, dst_len: size, bytes_so_far: size) -> size;
        }
    };
}
