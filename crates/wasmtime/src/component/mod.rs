//! In-progress implementation of the WebAssembly component model
//!
//! This module is a work-in-progress and currently represents an incomplete and
//! probably buggy implementation of the component model.

mod component;
mod func;
mod instance;
mod linker;
mod matching;
mod store;
pub use self::component::Component;
pub use self::func::{
    ComponentParams, ComponentType, Func, IntoComponentFunc, Lift, Lower, TypedFunc, WasmList,
    WasmStr,
};
pub use self::instance::{Instance, InstancePre};
pub use self::linker::{Linker, LinkerInstance};
pub use wasmtime_component_macro::{ComponentType, Lift, Lower};

// These items are expected to be used by an eventual
// `#[derive(ComponentType)]`, they are not part of Wasmtime's API stability
// guarantees
#[doc(hidden)]
pub mod __internal {
    pub use super::func::{
        align_to, next_field, typecheck_record, MaybeUninitExt, Memory, MemoryMut, Options,
    };
    pub use crate::map_maybe_uninit;
    pub use crate::store::StoreOpaque;
    pub use anyhow;
    pub use wasmtime_environ;
    pub use wasmtime_environ::component::{ComponentTypes, InterfaceType};
}

pub(crate) use self::store::ComponentStoreData;
