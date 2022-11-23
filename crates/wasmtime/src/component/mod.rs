//! In-progress implementation of the WebAssembly component model
//!
//! This module is a work-in-progress and currently represents an incomplete and
//! probably buggy implementation of the component model.

mod component;
mod func;
mod instance;
mod linker;
mod matching;
mod storage;
mod store;
pub mod types;
mod values;
pub use self::component::Component;
pub use self::func::{
    ComponentNamedList, ComponentType, Func, Lift, Lower, TypedFunc, WasmList, WasmStr,
};
pub use self::instance::{ExportInstance, Exports, Instance, InstancePre};
pub use self::linker::{Linker, LinkerInstance};
pub use self::types::Type;
pub use self::values::Val;
pub use wasmtime_component_macro::{flags, ComponentType, Lift, Lower};

// These items are expected to be used by an eventual
// `#[derive(ComponentType)]`, they are not part of Wasmtime's API stability
// guarantees
#[doc(hidden)]
pub mod __internal {
    pub use super::func::{
        format_flags, lower_payload, typecheck_enum, typecheck_flags, typecheck_record,
        typecheck_union, typecheck_variant, ComponentVariant, MaybeUninitExt, Memory, MemoryMut,
        Options,
    };
    pub use crate::map_maybe_uninit;
    pub use crate::store::StoreOpaque;
    pub use anyhow;
    pub use wasmtime_environ;
    pub use wasmtime_environ::component::{CanonicalAbiInfo, ComponentTypes, InterfaceType};
}

pub(crate) use self::store::ComponentStoreData;
