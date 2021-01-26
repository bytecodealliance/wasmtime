//! Performs autodetection of the host for the purposes of running
//! Cranelift to generate code to run on the same machine.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
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

use cranelift_codegen::isa;
use target_lexicon::Triple;

/// Return an `isa` builder configured for the current host
/// machine, or `Err(())` if the host machine is not supported
/// in the current configuration.
pub fn builder() -> Result<isa::Builder, &'static str> {
    builder_with_options(isa::BackendVariant::Any, true)
}

/// Return an `isa` builder configured for the current host
/// machine, or `Err(())` if the host machine is not supported
/// in the current configuration.
///
/// Selects the given backend variant specifically; this is
/// useful when more than oen backend exists for a given target
/// (e.g., on x86-64).
pub fn builder_with_options(
    variant: isa::BackendVariant,
    infer_native_flags: bool,
) -> Result<isa::Builder, &'static str> {
    let mut isa_builder =
        isa::lookup_variant(Triple::host(), variant).map_err(|err| match err {
            isa::LookupError::SupportDisabled => {
                "support for architecture disabled at compile time"
            }
            isa::LookupError::Unsupported => "unsupported architecture",
        })?;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use cranelift_codegen::settings::Configurable;

        if !std::is_x86_feature_detected!("sse2") {
            return Err("x86 support requires SSE2");
        }

        if !infer_native_flags {
            return Ok(isa_builder);
        }

        if std::is_x86_feature_detected!("sse3") {
            isa_builder.enable("has_sse3").unwrap();
        }
        if std::is_x86_feature_detected!("ssse3") {
            isa_builder.enable("has_ssse3").unwrap();
        }
        if std::is_x86_feature_detected!("sse4.1") {
            isa_builder.enable("has_sse41").unwrap();
        }
        if std::is_x86_feature_detected!("sse4.2") {
            isa_builder.enable("has_sse42").unwrap();
        }
        if std::is_x86_feature_detected!("popcnt") {
            isa_builder.enable("has_popcnt").unwrap();
        }
        if std::is_x86_feature_detected!("avx") {
            isa_builder.enable("has_avx").unwrap();
        }
        if std::is_x86_feature_detected!("avx2") {
            isa_builder.enable("has_avx2").unwrap();
        }
        if std::is_x86_feature_detected!("bmi1") {
            isa_builder.enable("has_bmi1").unwrap();
        }
        if std::is_x86_feature_detected!("bmi2") {
            isa_builder.enable("has_bmi2").unwrap();
        }
        if std::is_x86_feature_detected!("avx512dq") {
            isa_builder.enable("has_avx512dq").unwrap();
        }
        if std::is_x86_feature_detected!("avx512vl") {
            isa_builder.enable("has_avx512vl").unwrap();
        }
        if std::is_x86_feature_detected!("avx512f") {
            isa_builder.enable("has_avx512f").unwrap();
        }
        if std::is_x86_feature_detected!("lzcnt") {
            isa_builder.enable("has_lzcnt").unwrap();
        }
    }

    // squelch warnings about unused mut/variables on some platforms.
    drop(&mut isa_builder);
    drop(infer_native_flags);

    Ok(isa_builder)
}

#[cfg(test)]
mod tests {
    use super::builder;
    use cranelift_codegen::isa::CallConv;
    use cranelift_codegen::settings;

    #[test]
    fn test() {
        if let Ok(isa_builder) = builder() {
            let flag_builder = settings::builder();
            let isa = isa_builder.finish(settings::Flags::new(flag_builder));
            if cfg!(any(unix, target_os = "nebulet")) {
                assert_eq!(isa.default_call_conv(), CallConv::SystemV);
            } else if cfg!(windows) {
                assert_eq!(isa.default_call_conv(), CallConv::WindowsFastcall);
            }
            if cfg!(target_pointer_width = "64") {
                assert_eq!(isa.pointer_bits(), 64);
            }
            if cfg!(target_pointer_width = "32") {
                assert_eq!(isa.pointer_bits(), 32);
            }
            if cfg!(target_pointer_width = "16") {
                assert_eq!(isa.pointer_bits(), 16);
            }
        }
    }
}

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
