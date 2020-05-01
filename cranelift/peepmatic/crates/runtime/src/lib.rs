//! Runtime support for `peepmatic`'s peephole optimizers.
//!
//! This crate contains everything required to use a `peepmatic`-generated
//! peephole optimizer.
//!
//! ## Why is this a different crate from `peepmatic`?
//!
//! In short: build times and code size.
//!
//! If you are just using a peephole optimizer, you shouldn't need the functions
//! to construct it from scratch from the DSL (and the implied code size and
//! compilation time), let alone even build it at all. You should just
//! deserialize an already-built peephole optimizer, and then use it.
//!
//! That's all that is contained here in this crate.

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

pub mod cc;
pub mod error;
pub mod instruction_set;
pub mod integer_interner;
pub mod linear;
pub mod operator;
pub mod optimizations;
pub mod optimizer;
pub mod part;
pub mod paths;
pub mod r#type;

pub use error::{Error, Result};
pub use optimizations::PeepholeOptimizations;
pub use optimizer::PeepholeOptimizer;
