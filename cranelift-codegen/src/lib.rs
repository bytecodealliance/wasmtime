//! Cranelift code generation library.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature="cargo-clippy", allow(
// Produces only a false positive:
                clippy::while_let_loop,
// Produces many false positives, but did produce some valid lints, now fixed:
                clippy::needless_lifetimes,
// Generated code makes some style transgressions, but readability doesn't suffer much:
                clippy::many_single_char_names,
                clippy::identity_op,
                clippy::needless_borrow,
                clippy::cast_lossless,
                clippy::unreadable_literal,
                clippy::assign_op_pattern,
                clippy::empty_line_after_outer_attr,
// Hard to avoid in generated code:
                clippy::cyclomatic_complexity,
                clippy::too_many_arguments,
// Code generator doesn't have a way to collapse identical arms:
                clippy::match_same_arms,
// These are relatively minor style issues, but would be easy to fix:
                clippy::new_without_default,
                clippy::new_without_default_derive,
                clippy::should_implement_trait,
                clippy::len_without_is_empty))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
#![no_std]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use hashmap_core::{map as hash_map, HashMap, HashSet};
#[cfg(feature = "std")]
use std::collections::{hash_map, HashMap, HashSet};

pub use crate::context::Context;
pub use crate::legalizer::legalize_function;
pub use crate::value_label::{ValueLabelsRanges, ValueLocRange};
pub use crate::verifier::verify_function;
pub use crate::write::write_function;

pub use cranelift_bforest as bforest;
pub use cranelift_entity as entity;

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

pub use crate::entity::packed_option;

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
mod redundant_reload_remover;
mod ref_slice;
mod regalloc;
mod result;
mod scoped_hash_map;
mod simple_gvn;
mod simple_preopt;
mod stack_layout;
mod topo_order;
mod unreachable_code;
mod value_label;

pub use crate::result::{CodegenError, CodegenResult};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
