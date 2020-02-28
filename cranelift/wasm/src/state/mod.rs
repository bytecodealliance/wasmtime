//! WebAssembly module and function translation state.
//!
//! The `ModuleTranslationState` struct defined in this module is used to keep track of data about
//! the whole WebAssembly module, such as the decoded type signatures.
//!
//! The `FuncTranslationState` struct defined in this module is used to keep track of the WebAssembly
//! value and control stacks during the translation of a single function.

pub(crate) mod func_state;
pub(crate) mod module_state;

// Re-export for convenience.
pub(crate) use func_state::*;
pub(crate) use module_state::*;
