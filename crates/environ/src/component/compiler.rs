use crate::component::{
    ComponentTranslation, ComponentTypesBuilder, RuntimeComponentInstanceIndex,
    TypeResourceTableIndex, UnsafeIntrinsic,
};
use crate::error::Result;
use crate::{Abi, CompiledFunctionBody, FactInlineIntrinsic, FuncKey, Tunables};

/// The component-model-specific `FactInlineIntrinsic`s.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FactComponentIntrinsic {
    /// `resource.transfer-borrow-to-owner`: lazily lend a `borrow` to its
    /// resource's owning instance, or fall back to the libcall.
    ResourceTransferBorrowToOwner {
        /// The resource type of the handle being lent.
        ty: TypeResourceTableIndex,
        /// The component instance owning `ty`'s handle table.
        instance: RuntimeComponentInstanceIndex,
    },
}

impl From<FactComponentIntrinsic> for FactInlineIntrinsic {
    #[inline]
    fn from(intrinsic: FactComponentIntrinsic) -> Self {
        Self::Component(intrinsic)
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
    fn compile_component_trampoline(
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
