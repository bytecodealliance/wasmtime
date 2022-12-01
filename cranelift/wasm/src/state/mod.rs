//! WebAssembly module and function translation state.
//!
//! The `FuncTranslationState` struct defined in this module is used to keep track of the WebAssembly
//! value and control stacks during the translation of a single function.

pub(crate) mod func_state;

// Re-export for convenience.
pub(crate) use func_state::*;
