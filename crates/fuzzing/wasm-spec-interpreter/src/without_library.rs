//! Panic when interpreting WebAssembly modules; see the rationale for this in
//! `lib.rs`.
//!
//! ```should_panic
//! # use wasm_spec_interpreter::interpret;
//! let _ = interpret(&[], vec![]);
//! ```

use crate::Value;

#[allow(dead_code)]
pub fn interpret(_module: &[u8], _parameters: Vec<Value>) -> Result<Vec<Value>, String> {
    panic!(
        "wasm-spec-interpreter was built without its Rust-to-OCaml shim \
        library; re-compile with the dependencies listed in its README.md."
    );
}
