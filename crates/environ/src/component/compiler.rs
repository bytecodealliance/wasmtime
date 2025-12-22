use crate::component::{ComponentTranslation, ComponentTypesBuilder, UnsafeIntrinsic};
use crate::error::Result;
use crate::{Abi, CompiledFunctionBody, FuncKey, Tunables};

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
        types: &ComponentTypesBuilder,
        key: FuncKey,
        abi: Abi,
        tunables: &Tunables,
        symbol: &str,
    ) -> Result<CompiledFunctionBody>;

    /// Compile the given Wasmtime intrinsic.
    fn compile_intrinsic(
        &self,
        tunables: &Tunables,
        component: &ComponentTranslation,
        types: &ComponentTypesBuilder,
        intrinsic: UnsafeIntrinsic,
        abi: Abi,
        symbol: &str,
    ) -> Result<CompiledFunctionBody>;
}
