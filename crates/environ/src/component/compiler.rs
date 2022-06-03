use crate::component::{Component, ComponentTypes, LowerImport, LoweredIndex};
use crate::PrimaryMap;
use anyhow::Result;
use object::write::Object;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Description of where a trampoline is located in the text section of a
/// compiled image.
#[derive(Serialize, Deserialize)]
pub struct TrampolineInfo {
    /// The byte offset from the start of the text section where this trampoline
    /// starts.
    pub start: u32,
    /// The byte length of this trampoline's function body.
    pub length: u32,
}

/// Compilation support necessary for components.
pub trait ComponentCompiler: Send + Sync {
    /// Creates a trampoline for a `canon.lower`'d host function.
    ///
    /// This function will create a suitable trampoline which can be called from
    /// WebAssembly code and which will then call into host code. The signature
    /// of this generated trampoline should have the appropriate wasm ABI for
    /// the `lowering.canonical_abi` type signature (e.g. System-V).
    ///
    /// The generated trampoline will interpret its first argument as a
    /// `*mut VMComponentContext` and use the `VMComponentOffsets` for
    /// `component` to read necessary data (as specified by `lowering.options`)
    /// and call the host function pointer. Notably the host function pointer
    /// has the signature `VMLoweringCallee` where many of the arguments are
    /// loaded from known offsets (for this particular generated trampoline)
    /// from the `VMComponentContext`.
    ///
    /// Returns a compiler-specific `Box<dyn Any>` which can be passed later to
    /// `emit_obj` to crate an elf object.
    fn compile_lowered_trampoline(
        &self,
        component: &Component,
        lowering: &LowerImport,
        types: &ComponentTypes,
    ) -> Result<Box<dyn Any + Send>>;

    /// Emits the `trampolines` specified into the in-progress ELF object
    /// specified by `obj`.
    ///
    /// Returns a map of trampoline information for where to find them all in
    /// the text section.
    ///
    /// Note that this will also prepare unwinding information for all the
    /// trampolines as necessary.
    fn emit_obj(
        &self,
        trampolines: PrimaryMap<LoweredIndex, Box<dyn Any + Send>>,
        obj: &mut Object<'static>,
    ) -> Result<PrimaryMap<LoweredIndex, TrampolineInfo>>;
}
