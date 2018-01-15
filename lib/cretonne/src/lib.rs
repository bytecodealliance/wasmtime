//! Cretonne code generation library.
#![cfg_attr(feature = "no_std", no_std)]
#![deny(missing_docs)]

// Turns on alloc feature if no_std
#![cfg_attr(feature = "no_std", feature(alloc))]

// Include the `hashmap_core` crate if no_std
#[cfg(feature = "no_std")]
extern crate hashmap_core;
#[cfg(feature = "no_std")]
extern crate error_core;
#[cfg(feature = "no_std")]
#[macro_use]
extern crate alloc;

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
mod iterators;
mod legalizer;
mod licm;
mod partition_slice;
mod predicates;
mod ref_slice;
mod regalloc;
mod scoped_hash_map;
mod simple_gvn;
mod stack_layout;
mod topo_order;
mod unreachable_code;
mod write;

/// This replaces `std` in builds with no_std.
#[cfg(feature = "no_std")]
mod std {
    pub use core::*;
    #[macro_use]
    pub use alloc::{boxed, vec, string};
    pub mod prelude {
        pub use core::prelude as v1;
    }
    pub mod collections {
        pub use hashmap_core::{HashMap, HashSet};
        pub use hashmap_core::map as hash_map;
        pub use alloc::BTreeSet;
    }
    pub mod error {
        pub use error_core::Error;
    }
}