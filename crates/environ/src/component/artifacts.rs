//! Definitions of compilation artifacts of the component compilation process
//! which are serialized with `bincode` into output ELF files.

use crate::{
    CompiledFunctionsTable, CompiledModuleInfo, PrimaryMap, StaticModuleIndex, WasmChecksum,
    component::{Component, ComponentTypes, TypeComponentIndex},
};
use serde_derive::{Deserialize, Serialize};

/// Serializable state that's stored in a compilation artifact.
#[derive(Serialize, Deserialize)]
pub struct ComponentArtifacts {
    /// The type of this component.
    pub ty: TypeComponentIndex,
    /// Information all kept available at runtime as-is.
    pub info: CompiledComponentInfo,
    /// The index of every compiled function's location in the text section.
    pub table: CompiledFunctionsTable,
    /// Type information for this component and all contained modules.
    pub types: ComponentTypes,
    /// Serialized metadata about all included core wasm modules.
    pub static_modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
    /// A checksum of the source Wasm binary from which the component was compiled.
    pub checksum: WasmChecksum,
}

/// Runtime state that a component retains to support its operation.
#[derive(Serialize, Deserialize)]
pub struct CompiledComponentInfo {
    /// Type information calculated during translation about this component.
    pub component: Component,
}
