//! Performes early-stage optimizations on Cranelift IR.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(
    feature = "clippy",
    plugin(clippy(conf_file = "../../clippy.toml"))
)]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
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
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), alloc)]

#[cfg(not(feature = "std"))]
extern crate alloc;

extern crate cranelift_codegen;
// extern crate rustc_apfloat;

mod constant_folding;

use cranelift_codegen::{isa::TargetIsa, settings::FlagsOrIsa, CodegenResult, Context};

/// Optimize the function with available optimizations.
///
/// Since this can be resource intensive (and code-size inflating),
/// it is separated from `Context::compile` to allow DCE to remove it
/// if it's not used.
pub fn optimize(ctx: &mut Context, isa: &TargetIsa) -> CodegenResult<()> {
    ctx.verify_if(isa)?;
    fold_constants(ctx, isa)?;

    Ok(())
}

/// Fold constants
pub fn fold_constants<'a, FOI>(ctx: &mut Context, fisa: FOI) -> CodegenResult<()>
where
    FOI: Into<FlagsOrIsa<'a>>,
{
    constant_folding::fold_constants(&mut ctx.func);
    ctx.verify_if(fisa)?;
    Ok(())
}

/// This replaces `std` in builds with `core`.
#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::{boxed, slice, string, vec};
    pub use core::*;
}
