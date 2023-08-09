use crate::component::{ComponentTranslation, ComponentTypes, TrampolineIndex};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A triple of related functions/trampolines variants with differing calling
/// conventions: `{wasm,array,native}_call`.
///
/// Generic so we can use this with either the `Box<dyn Any + Send>`s that
/// implementations of the compiler trait return or with `FunctionLoc`s inside
/// an object file, for example.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct AllCallFunc<T> {
    /// The function exposing the Wasm calling convention.
    pub wasm_call: T,
    /// The function exposing the array calling convention.
    pub array_call: T,
    /// The function exposing the native calling convention.
    pub native_call: T,
}

impl<T> AllCallFunc<T> {
    /// Map an `AllCallFunc<T>` into an `AllCallFunc<U>`.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> AllCallFunc<U> {
        AllCallFunc {
            wasm_call: f(self.wasm_call),
            array_call: f(self.array_call),
            native_call: f(self.native_call),
        }
    }
}

/// Compilation support necessary for components.
pub trait ComponentCompiler: Send + Sync {
    /// Compiles the pieces necessary to create a `VMFuncRef` for the
    /// `trampoline` specified.
    ///
    /// Each trampoline is a member of the `Trampoline` enumeration and has a
    /// unique purpose and is translated differently. See the implementation of
    /// this trait for Cranelift for more information.
    fn compile_trampoline(
        &self,
        component: &ComponentTranslation,
        types: &ComponentTypes,
        trampoline: TrampolineIndex,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>>;
}
