//! Definitions of compilation artifacts of the component compilation process
//! which are serialized with `bincode` into output ELF files.

use crate::{
    CompiledModuleInfo, FunctionLoc, PrimaryMap, StaticModuleIndex,
    component::{Component, ComponentTypes, TrampolineIndex, TypeComponentIndex},
};
use serde_derive::{Deserialize, Serialize};

/// Serializable state that's stored in a compilation artifact.
#[derive(Serialize, Deserialize)]
pub struct ComponentArtifacts {
    /// The type of this component.
    pub ty: TypeComponentIndex,
    /// Information all kept available at runtime as-is.
    pub info: CompiledComponentInfo,
    /// Type information for this component and all contained modules.
    pub types: ComponentTypes,
    /// Serialized metadata about all included core wasm modules.
    pub static_modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
}

/// Runtime state that a component retains to support its operation.
#[derive(Serialize, Deserialize)]
pub struct CompiledComponentInfo {
    /// Type information calculated during translation about this component.
    pub component: Component,

    /// Where lowered function wasm-call trampolines that end up in a
    /// `VMFuncRef` are located within the `text` section of `code_memory`.
    pub wasm_call_trampolines: PrimaryMap<TrampolineIndex, FunctionLoc>,

    /// Where lowered function array-call trampolines that end up in a
    /// `VMFuncRef` are located within the `text` section of `code_memory`.
    pub array_call_trampolines: PrimaryMap<TrampolineIndex, FunctionLoc>,

    /// The location of the wasm-to-array trampoline for the `resource.drop`
    /// intrinsic.
    pub resource_drop_wasm_to_array_trampoline: Option<FunctionLoc>,
}
