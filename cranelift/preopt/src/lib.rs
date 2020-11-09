//! Performs early-stage optimizations on Cranelift IR.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
#![no_std]

mod constant_folding;

use cranelift_codegen::{isa::TargetIsa, settings::FlagsOrIsa, CodegenResult, Context};

/// Optimize the function with available optimizations.
///
/// Since this can be resource intensive (and code-size inflating),
/// it is separated from `Context::compile` to allow DCE to remove it
/// if it's not used.
pub fn optimize(ctx: &mut Context, isa: &dyn TargetIsa) -> CodegenResult<()> {
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
