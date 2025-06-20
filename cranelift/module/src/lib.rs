//! Top-level lib.rs for `cranelift_module`.

#![deny(missing_docs)]
#![no_std]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use hashbrown::{HashMap, hash_map};
use std::borrow::ToOwned;
use std::boxed::Box;
#[cfg(feature = "std")]
use std::collections::{HashMap, hash_map};
use std::string::String;

use cranelift_codegen::ir;

mod data_context;
mod module;
mod traps;

pub use crate::data_context::{DataDescription, Init};
pub use crate::module::{
    DataDeclaration, DataId, FuncId, FuncOrDataId, FunctionDeclaration, Linkage, Module,
    ModuleDeclarations, ModuleError, ModuleReloc, ModuleRelocTarget, ModuleResult,
};
pub use crate::traps::TrapSite;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default names for [ir::LibCall]s. A function by this name is imported into the object as
/// part of the translation of a [ir::ExternalName::LibCall] variant.
pub fn default_libcall_names() -> Box<dyn Fn(ir::LibCall) -> String + Send + Sync> {
    Box::new(move |libcall| match libcall {
        ir::LibCall::Probestack => "__cranelift_probestack".to_owned(),
        ir::LibCall::CeilF32 => "ceilf".to_owned(),
        ir::LibCall::CeilF64 => "ceil".to_owned(),
        ir::LibCall::FloorF32 => "floorf".to_owned(),
        ir::LibCall::FloorF64 => "floor".to_owned(),
        ir::LibCall::TruncF32 => "truncf".to_owned(),
        ir::LibCall::TruncF64 => "trunc".to_owned(),
        ir::LibCall::NearestF32 => "nearbyintf".to_owned(),
        ir::LibCall::NearestF64 => "nearbyint".to_owned(),
        ir::LibCall::FmaF32 => "fmaf".to_owned(),
        ir::LibCall::FmaF64 => "fma".to_owned(),
        ir::LibCall::Memcpy => "memcpy".to_owned(),
        ir::LibCall::Memset => "memset".to_owned(),
        ir::LibCall::Memmove => "memmove".to_owned(),
        ir::LibCall::Memcmp => "memcmp".to_owned(),

        ir::LibCall::ElfTlsGetAddr => "__tls_get_addr".to_owned(),
        ir::LibCall::ElfTlsGetOffset => "__tls_get_offset".to_owned(),
        ir::LibCall::X86Pshufb => "__cranelift_x86_pshufb".to_owned(),
    })
}
