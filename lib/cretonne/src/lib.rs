//! Cretonne code generation library.

#![deny(missing_docs,
        trivial_numeric_casts,
        unused_extern_crates)]

#![cfg_attr(feature="clippy",
            plugin(clippy(conf_file="../../clippy.toml")))]

#![cfg_attr(feature="cargo-clippy", allow(
// Rustfmt 0.9.0 is at odds with this lint:
                block_in_if_condition_stmt,
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
                redundant_field_names,
                useless_let_if_seq,
                len_without_is_empty))]

pub use context::Context;
pub use legalizer::legalize_function;
pub use verifier::verify_function;
pub use write::write_function;

/// Version number of the cretonne crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_use]
pub mod dbg;
#[macro_use]
pub mod entity;

pub mod bforest;
pub mod binemit;
pub mod cfg_printer;
pub mod cursor;
pub mod dominator_tree;
pub mod flowgraph;
pub mod ir;
pub mod isa;
pub mod loop_analysis;
pub mod packed_option;
pub mod print_errors;
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
