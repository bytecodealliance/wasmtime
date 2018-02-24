//! Cretonne code generation library.

#![deny(missing_docs,
        trivial_numeric_casts,
        unused_extern_crates)]

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
pub mod cursor;
pub mod dominator_tree;
pub mod flowgraph;
pub mod ir;
pub mod isa;
pub mod loop_analysis;
pub mod packed_option;
pub mod result;
pub mod settings;
pub mod timing;
pub mod verifier;

mod abi;
mod bitset;
mod constant_hash;
mod context;
mod divconst_magic_numbers;
mod iterators;
mod legalizer;
mod licm;
mod partition_slice;
mod predicates;
mod preopt;
mod ref_slice;
mod regalloc;
mod scoped_hash_map;
mod simple_gvn;
mod stack_layout;
mod topo_order;
mod unreachable_code;
mod write;
