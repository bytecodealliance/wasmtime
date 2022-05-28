//! In-progress implementation of the WebAssembly component model
//!
//! This module is a work-in-progress and currently represents an incomplete and
//! probably buggy implementation of the component model.

mod component;
mod func;
mod instance;
mod store;
pub use self::component::Component;
pub use self::func::{ComponentParams, ComponentValue, Func, Op, TypedFunc, WasmList, WasmStr};
pub use self::instance::Instance;

// These items are expected to be used by an eventual
// `#[derive(ComponentValue)]`, they are not part of Wasmtime's API stability
// guarantees
#[doc(hidden)]
pub use {self::func::Memory, wasmtime_environ};

pub(crate) use self::store::ComponentStoreData;
