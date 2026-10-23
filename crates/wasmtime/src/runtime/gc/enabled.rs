//! The actual implementation of garbage collection, for when the `gc` Cargo
//! feature is enabled.

mod anyref;
mod arrayref;
mod eqref;
mod exnref;
mod externref;
mod i31;
mod rooting;
mod structref;

pub use anyref::*;
pub use arrayref::*;
pub use eqref::*;
pub use exnref::*;
pub use externref::*;
pub use i31::*;
pub use rooting::*;
pub use structref::*;

use crate::{Engine, bail_bug, prelude::*};
use alloc::sync::Arc;
use wasmtime_environ::{GcArrayLayout, GcLayout, GcStructLayout, VMSharedTypeIndex};

/// Get the GC layout registered for `type_index`.
///
/// Type indices generally come out of the untrusted GC heap, so `type_index`
/// may not name a registered type at all.
fn gc_layout(engine: &Engine, type_index: VMSharedTypeIndex) -> Result<GcLayout> {
    match engine.signatures().layout(type_index) {
        Some(layout) => Ok(layout),
        None => bail_bug!("no GC layout for {type_index:?}"),
    }
}

/// Get the struct layout registered for `type_index`.
///
/// See `gc_layout`.
fn gc_struct_layout(engine: &Engine, type_index: VMSharedTypeIndex) -> Result<Arc<GcStructLayout>> {
    match gc_layout(engine, type_index)? {
        GcLayout::Struct(s) => Ok(s),
        GcLayout::Array(_) => bail_bug!("expected a struct GC layout for {type_index:?}"),
    }
}

/// Get the array layout registered for `type_index`.
///
/// See `gc_layout`.
fn gc_array_layout(engine: &Engine, type_index: VMSharedTypeIndex) -> Result<GcArrayLayout> {
    match gc_layout(engine, type_index)? {
        GcLayout::Array(a) => Ok(a),
        GcLayout::Struct(_) => bail_bug!("expected an array GC layout for {type_index:?}"),
    }
}
