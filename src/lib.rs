//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]

use once_cell::sync::Lazy;
use wasmtime_cli_flags::{SUPPORTED_WASI_MODULES, SUPPORTED_WASM_FEATURES};

static FLAG_EXPLANATIONS: Lazy<String> = Lazy::new(|| {
    use std::fmt::Write;

    let mut s = String::new();

    // Explain --wasm-features.
    writeln!(&mut s, "Supported values for `--wasm-features`:").unwrap();
    writeln!(&mut s).unwrap();
    let max = SUPPORTED_WASM_FEATURES
        .iter()
        .max_by_key(|(name, _)| name.len())
        .unwrap();
    for (name, desc) in SUPPORTED_WASM_FEATURES.iter() {
        writeln!(&mut s, "{:width$} {}", name, desc, width = max.0.len() + 2).unwrap();
    }
    writeln!(&mut s).unwrap();

    // Explain --wasi-modules.
    writeln!(&mut s, "Supported values for `--wasi-modules`:").unwrap();
    writeln!(&mut s).unwrap();
    let max = SUPPORTED_WASI_MODULES
        .iter()
        .max_by_key(|(name, _)| name.len())
        .unwrap();
    for (name, desc) in SUPPORTED_WASI_MODULES.iter() {
        writeln!(&mut s, "{:width$} {}", name, desc, width = max.0.len() + 2).unwrap();
    }

    writeln!(&mut s).unwrap();
    writeln!(&mut s, "Features prefixed with '-' will be disabled.").unwrap();

    s
});

pub mod commands;
