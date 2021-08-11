//! Utilities for working with object files that operate as Wasmtime's
//! serialization and intermediate format for compiled modules.

use cranelift_entity::EntityRef;
use cranelift_wasm::{FuncIndex, SignatureIndex};

const FUNCTION_PREFIX: &str = "_wasm_function_";
const TRAMPOLINE_PREFIX: &str = "_trampoline_";

/// Returns the symbol name in an object file for the corresponding wasm
/// function index in a module.
pub fn func_symbol_name(index: FuncIndex) -> String {
    format!("{}{}", FUNCTION_PREFIX, index.index())
}

/// Attempts to extract the corresponding function index from a symbol possibly produced by
/// `func_symbol_name`.
pub fn try_parse_func_name(name: &str) -> Option<FuncIndex> {
    let n = name.strip_prefix(FUNCTION_PREFIX)?.parse().ok()?;
    Some(FuncIndex::new(n))
}

/// Returns the symbol name in an object file for the corresponding trampoline
/// for the given signature in a module.
pub fn trampoline_symbol_name(index: SignatureIndex) -> String {
    format!("{}{}", TRAMPOLINE_PREFIX, index.index())
}

/// Attempts to extract the corresponding signature index from a symbol
/// possibly produced by `trampoline_symbol_name`.
pub fn try_parse_trampoline_name(name: &str) -> Option<SignatureIndex> {
    let n = name.strip_prefix(TRAMPOLINE_PREFIX)?.parse().ok()?;
    Some(SignatureIndex::new(n))
}
