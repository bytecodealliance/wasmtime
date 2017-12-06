//! Cretonne code generation library.

#![deny(missing_docs)]

pub use context::Context;
pub use legalizer::legalize_function;
pub use verifier::verify_function;
pub use write::write_function;

/// Version number of the cretonne crate.
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[macro_use]
pub mod dbg;
#[macro_use]
pub mod entity;

pub mod bforest;
pub mod binemit;
pub mod bitset;
pub mod cursor;
pub mod dominator_tree;
pub mod flowgraph;
pub mod ir;
pub mod isa;
pub mod loop_analysis;
pub mod packed_option;
pub mod regalloc;
pub mod result;
pub mod settings;
pub mod timing;
pub mod verifier;

mod abi;
mod constant_hash;
mod context;
mod iterators;
mod legalizer;
mod licm;
mod partition_slice;
mod predicates;
mod ref_slice;
mod scoped_hash_map;
mod simple_gvn;
mod stack_layout;
mod topo_order;
mod unreachable_code;
mod write;
