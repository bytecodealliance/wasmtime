//! Utilities for working with object files that operate as Wasmtime's
//! serialization and intermediate format for compiled modules.

use crate::{EntityRef, FuncIndex, SignatureIndex};

const FUNCTION_PREFIX: &str = "_wasm_function_";
const TRAMPOLINE_PREFIX: &str = "_trampoline_";

/// Returns the symbol name in an object file for the corresponding wasm
/// function index in a module.
pub fn func_symbol_name(index: FuncIndex) -> String {
    format!("{}{}", FUNCTION_PREFIX, index.index())
}

/// Returns the symbol name in an object file for the corresponding trampoline
/// for the given signature in a module.
pub fn trampoline_symbol_name(index: SignatureIndex) -> String {
    format!("{}{}", TRAMPOLINE_PREFIX, index.index())
}
