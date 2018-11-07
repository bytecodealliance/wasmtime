//! Cranelift code generation library.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(
    feature = "clippy",
    plugin(clippy(conf_file = "../../clippy.toml"))
)]
#![cfg_attr(feature="cargo-clippy", allow(
// Produces only a false positive:
                while_let_loop,
// Produces many false positives, but did produce some valid lints, now fixed:
                needless_lifetimes,
// Generated code makes some style transgressions, but readability doesn't suffer much:
                many_single_char_names,
                identity_op,
                needless_borrow,
                cast_lossless,
                unreadable_literal,
                assign_op_pattern,
                empty_line_after_outer_attr,
// Hard to avoid in generated code:
                cyclomatic_complexity,
                too_many_arguments,
// Code generator doesn't have a way to collapse identical arms:
                match_same_arms,
// These are relatively minor style issues, but would be easy to fix:
                new_without_default,
                new_without_default_derive,
                should_implement_trait,
                len_without_is_empty))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        print_stdout,
        unicode_not_nfc,
        use_self
    )
)]
// Turns on no_std and alloc features if std is not available.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]
// TODO: Remove this workaround once https://github.com/rust-lang/rust/issues/27747 is done.
#![cfg_attr(not(feature = "std"), feature(slice_concat_ext))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[cfg_attr(test, macro_use)]
extern crate target_lexicon;

#[macro_use]
extern crate log;

pub use context::Context;
pub use legalizer::legalize_function;
pub use verifier::verify_function;
pub use write::write_function;

/// Version number of the cranelift-codegen crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_use]
pub extern crate cranelift_entity as entity;
pub extern crate cranelift_bforest as bforest;

pub mod binemit;
pub mod cfg_printer;
pub mod cursor;
pub mod dbg;
pub mod dominator_tree;
pub mod flowgraph;
pub mod ir;
pub mod isa;
pub mod loop_analysis;
pub mod print_errors;
pub mod settings;
pub mod timing;
pub mod verifier;
pub mod write;

pub use entity::packed_option;

mod abi;
mod bitset;
mod constant_hash;
mod context;
mod dce;
mod divconst_magic_numbers;
mod fx;
mod iterators;
mod legalizer;
mod licm;
mod nan_canonicalization;
mod partition_slice;
mod postopt;
mod predicates;
mod ref_slice;
mod regalloc;
mod result;
mod scoped_hash_map;
mod simple_gvn;
mod simple_preopt;
mod stack_layout;
mod topo_order;
mod unreachable_code;

pub use result::{CodegenError, CodegenResult};

/// This replaces `std` in builds with `core`.
#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{boxed, slice, string, vec};
    pub use core::*;
    pub mod collections {
        #[allow(unused_extern_crates)]
        extern crate hashmap_core;

        pub use self::hashmap_core::map as hash_map;
        pub use self::hashmap_core::{HashMap, HashSet};
        pub use alloc::collections::BTreeSet;
    }
}
