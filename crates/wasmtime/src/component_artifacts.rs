#![cfg(feature = "component-model")]

use serde_derive::{Deserialize, Serialize};
use wasmtime_environ::{
    component::{AllCallFunc, ComponentTypes, TrampolineIndex},
    CompiledModuleInfo, FunctionLoc, PrimaryMap, StaticModuleIndex,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentArtifacts {
    pub(crate) info: CompiledComponentInfo,
    pub(crate) types: ComponentTypes,
    pub(crate) static_modules: PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CompiledComponentInfo {
    /// Type information calculated during translation about this component.
    pub(crate) component: wasmtime_environ::component::Component,

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
    pub(crate) trampolines: PrimaryMap<TrampolineIndex, AllCallFunc<FunctionLoc>>,

    /// The location of the wasm-to-native trampoline for the `resource.drop`
    /// intrinsic.
    pub(crate) resource_drop_wasm_to_native_trampoline: Option<FunctionLoc>,
}
