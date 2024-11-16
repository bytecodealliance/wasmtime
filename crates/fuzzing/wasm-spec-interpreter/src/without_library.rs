//! Panic when interpreting WebAssembly modules; see the rationale for this in
//! `lib.rs`.
//!
//! ```should_panic
//! # use wasm_spec_interpreter::instantiate;
//! let _ = instantiate(&[]);
//! ```

use crate::{SpecExport, SpecInstance, SpecValue};

pub fn instantiate(_module: &[u8]) -> Result<SpecInstance, String> {
    fail_at_runtime()
}

pub fn interpret(
    _instance: &SpecInstance,
    _name: &str,
    _parameters: Option<Vec<SpecValue>>,
) -> Result<Vec<SpecValue>, String> {
    fail_at_runtime()
}

pub fn interpret_legacy(
    _module: &[u8],
    _parameters: Option<Vec<SpecValue>>,
) -> Result<Vec<SpecValue>, String> {
    fail_at_runtime()
}

pub fn export(_instance: &SpecInstance, _name: &str) -> Result<SpecExport, String> {
    fail_at_runtime()
}

fn fail_at_runtime() -> ! {
    panic!(
        "wasm-spec-interpreter was built without its Rust-to-OCaml shim \
        library; re-compile with the dependencies listed in its README.md."
    );
}

pub fn setup_ocaml_runtime() {}
