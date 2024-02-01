//! Definitions of compilation artifacts of the component compilation process
//! which are serialized with `bincode` into output ELF files.

use crate::{
    component::{AllCallFunc, Component, ComponentTypes, TrampolineIndex, TypeComponentIndex},
    CompiledModuleInfo, FunctionLoc, PrimaryMap, StaticModuleIndex,
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

    /// Where lowered function trampolines are located within the `text`
    /// section of `code_memory`.
    ///
    /// These are the
    ///
    /// 1. Wasm-call,
    /// 2. array-call, and
    /// 3. native-call
    ///
    /// function pointers that end up in a `VMFuncRef` for each
    /// lowering.
    pub trampolines: PrimaryMap<TrampolineIndex, AllCallFunc<FunctionLoc>>,

    /// The location of the wasm-to-native trampoline for the `resource.drop`
    /// intrinsic.
    pub resource_drop_wasm_to_native_trampoline: Option<FunctionLoc>,
}
