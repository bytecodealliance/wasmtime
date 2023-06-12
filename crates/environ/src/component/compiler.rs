use crate::component::{Component, ComponentTypes, LowerImport, Transcoder};
use crate::WasmFuncType;
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
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>>;

    /// Creates a function which will always trap that has the `ty` specified.
    ///
    /// This will create a small trampoline whose only purpose is to generate a
    /// trap at runtime. This is used to implement the degenerate case of a
    /// `canon lift`'d function immediately being `canon lower`'d.
    fn compile_always_trap(&self, ty: &WasmFuncType) -> Result<AllCallFunc<Box<dyn Any + Send>>>;

    /// Compiles a trampoline to implement string transcoding from adapter
    /// modules.
    ///
    /// The generated trampoline will invoke the `transcoder.op` libcall with
    /// the various memory configuration provided in `transcoder`. This is used
    /// to pass raw pointers to host functions to avoid the host having to deal
    /// with base pointers, offsets, memory32-vs-64, etc.
    ///
    /// Note that all bounds checks for memories are present in adapters
    /// themselves, and the host libcalls simply assume that the pointers are
    /// valid.
    fn compile_transcoder(
        &self,
        component: &Component,
        transcoder: &Transcoder,
        types: &ComponentTypes,
    ) -> Result<AllCallFunc<Box<dyn Any + Send>>>;
}
