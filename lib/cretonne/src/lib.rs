//! Cretonne code generation library.

#![deny(missing_docs)]

pub use verifier::verify_function;
pub use write::write_function;
pub use legalizer::legalize_function;

/// Version number of the cretonne crate.
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub mod ir;
pub mod isa;
pub mod cfg;
pub mod dominator_tree;
pub mod entity_map;
pub mod entity_list;
pub mod sparse_map;
pub mod settings;
pub mod verifier;
pub mod regalloc;

mod write;
mod constant_hash;
mod predicates;
mod legalizer;
mod ref_slice;
mod partition_slice;
mod packed_option;
