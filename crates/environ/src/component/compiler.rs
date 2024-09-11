use crate::component::{AllCallFunc, ComponentTranslation, ComponentTypesBuilder, TrampolineIndex};
use crate::prelude::*;
use crate::Tunables;
use anyhow::Result;
use std::any::Any;

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
        trampoline: TrampolineIndex,
        tunables: &Tunables,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>>;
}
