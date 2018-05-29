//! Performs autodetection of the host for the purposes of running
//! Cretonne to generate code to run on the same machine.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate cretonne_codegen;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
extern crate raw_cpuid;

use cretonne_codegen::isa;
use cretonne_codegen::settings::{self, Configurable};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use raw_cpuid::CpuId;

/// Return `settings` and `isa` builders configured for the current host
/// machine, or `Err(())` if the host machine is not supported
/// in the current configuration.
pub fn builders() -> Result<(settings::Builder, isa::Builder), &'static str> {
    let mut flag_builder = settings::builder();

    if cfg!(any(unix, target_os = "nebulet")) {
        flag_builder.set("call_conv", "system_v").unwrap();
    } else if cfg!(windows) {
        flag_builder.set("call_conv", "windows_fastcall").unwrap();
    } else {
        return Err("unrecognized environment");
    }

    if cfg!(target_pointer_width = "64") {
        flag_builder.enable("is_64bit").unwrap();
    } else if !cfg!(target_pointer_width = "32") {
        return Err("unrecognized pointer size");
    }

    // TODO: Add RISC-V support once Rust supports it.
    let name = if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        "x86"
    } else if cfg!(target_arch = "arm") {
        "arm32"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err("unrecognized architecture");
    };

    let mut isa_builder = isa::lookup(name).map_err(|err| match err {
        isa::LookupError::Unknown => panic!(),
        isa::LookupError::Unsupported => "unsupported architecture",
    })?;

    if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        parse_x86_cpuid(&mut isa_builder)?;
    }

    Ok((flag_builder, isa_builder))
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn parse_x86_cpuid(isa_builder: &mut isa::Builder) -> Result<(), &'static str> {
    let cpuid = CpuId::new();

    if let Some(info) = cpuid.get_feature_info() {
        if !info.has_sse2() {
            return Err("x86 support requires SSE2");
        }
        if info.has_sse3() {
            isa_builder.enable("has_sse3").unwrap();
        }
        if info.has_sse41() {
            isa_builder.enable("has_sse41").unwrap();
        }
        if info.has_sse42() {
            isa_builder.enable("has_sse42").unwrap();
        }
        if info.has_popcnt() {
            isa_builder.enable("has_popcnt").unwrap();
        }
        if info.has_avx() {
            isa_builder.enable("has_avx").unwrap();
        }
    }
    if let Some(info) = cpuid.get_extended_feature_info() {
        if info.has_bmi1() {
            isa_builder.enable("has_bmi1").unwrap();
        }
        if info.has_bmi2() {
            isa_builder.enable("has_bmi2").unwrap();
        }
    }
    if let Some(info) = cpuid.get_extended_function_info() {
        if info.has_lzcnt() {
            isa_builder.enable("has_lzcnt").unwrap();
        }
    }
    Ok(())
}
