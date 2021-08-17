//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::{FunctionAddressMap, FunctionBodyData, ModuleTranslation, Tunables, TypeTables};
use anyhow::Result;
use cranelift_codegen::{binemit, ir, isa::unwind::UnwindInfo};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, WasmError, WasmFuncType};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

#[allow(missing_docs)]
pub type CompiledFunctions = PrimaryMap<DefinedFuncIndex, CompiledFunction>;

/// Compiled function: machine code body, jump table offsets, and unwind information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub struct CompiledFunction {
    /// The machine code for this function.
    pub body: Vec<u8>,

    /// The jump tables offsets (in the body).
    pub jt_offsets: ir::JumpTableOffsets,

    /// The unwind information.
    pub unwind_info: Option<UnwindInfo>,

    pub relocations: Vec<Relocation>,
    pub address_map: FunctionAddressMap,
    pub value_labels_ranges: cranelift_codegen::ValueLabelsRanges,
    pub stack_slots: ir::StackSlots,
    pub traps: Vec<TrapInformation>,
    pub stack_maps: Vec<StackMapInformation>,
}

/// A record of a relocation to perform.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// Relocation target.
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

/// Destination function. Can be either user function or some special one, like `memory.grow`.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelocationTarget {
    /// The user function index.
    UserFunc(FuncIndex),
    /// A compiler-generated libcall.
    LibCall(ir::LibCall),
    /// Jump table index.
    JumpTable(FuncIndex, ir::JumpTable),
}

/// Information about trap.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code. It is relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,
    /// Code of the trap.
    pub trap_code: ir::TrapCode,
}

/// The offset within a function of a GC safepoint, and its associated stack
/// map.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StackMapInformation {
    /// The offset of the GC safepoint within the function's native code. It is
    /// relative to the beginning of the function.
    pub code_offset: binemit::CodeOffset,

    /// The stack map for identifying live GC refs at the GC safepoint.
    pub stack_map: binemit::StackMap,
}

/// An error while compiling WebAssembly to machine code.
#[derive(Error, Debug)]
pub enum CompileError {
    /// A wasm translation error occured.
    #[error("WebAssembly translation error")]
    Wasm(#[from] WasmError),

    /// A compilation error occured.
    #[error("Compilation error: {0}")]
    Codegen(String),

    /// A compilation error occured.
    #[error("Debug info is not supported with this configuration")]
    DebugInfoNotSupported,
}

/// Abstract trait representing the ability to create a `Compiler` below.
///
/// This is used in Wasmtime to separate compiler implementations, currently
/// mostly used to separate Cranelift from Wasmtime itself.
pub trait CompilerBuilder: Send + Sync + fmt::Debug {
    /// Like the `Clone` trait, but for the boxed trait object.
    fn clone(&self) -> Box<dyn CompilerBuilder>;

    /// Sets the target of compilation to the target specified.
    fn target(&mut self, target: target_lexicon::Triple) -> Result<()>;

    /// Returns the currently configured target triple that compilation will
    /// produce artifacts for.
    fn triple(&self) -> &target_lexicon::Triple;

    /// Compiler-specific method to configure various settings in the compiler
    /// itself.
    ///
    /// This is expected to be defined per-compiler. Compilers should return
    /// errors for unknown names/values.
    fn set(&mut self, name: &str, val: &str) -> Result<()>;

    /// Compiler-specific method for configuring settings.
    ///
    /// Same as [`CompilerBuilder::set`] except for enabling boolean flags.
    /// Currently cranelift uses this to sometimes enable a family of settings.
    fn enable(&mut self, name: &str) -> Result<()>;

    /// Returns a list of all possible settings that can be configured with
    /// [`CompilerBuilder::set`] and [`CompilerBuilder::enable`].
    fn settings(&self) -> Vec<Setting>;

    /// Builds a new [`Compiler`] object from this configuration.
    fn build(&self) -> Box<dyn Compiler>;
}

/// Description of compiler settings returned by [`CompilerBuilder::settings`].
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    /// The name of the setting.
    pub name: &'static str,
    /// The description of the setting.
    pub description: &'static str,
    /// The kind of the setting.
    pub kind: SettingKind,
    /// The supported values of the setting (for enum values).
    pub values: Option<&'static [&'static str]>,
}

/// Different kinds of [`Setting`] values that can be configured in a
/// [`CompilerBuilder`]
#[derive(Clone, Copy, Debug)]
pub enum SettingKind {
    /// The setting is an enumeration, meaning it's one of a set of values.
    Enum,
    /// The setting is a number.
    Num,
    /// The setting is a boolean.
    Bool,
    /// The setting is a preset.
    Preset,
}

/// An implementation of a compiler which can compile WebAssembly functions to
/// machine code and perform other miscellaneous tasks needed by the JIT runtime.
pub trait Compiler: Send + Sync {
    /// Compiles the function `index` within `translation`.
    ///
    /// The body of the function is available in `data` and configuration
    /// values are also passed in via `tunables`. Type information in
    /// `translation` is all relative to `types`.
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        tunables: &Tunables,
        types: &TypeTables,
    ) -> Result<CompiledFunction, CompileError>;

    /// Collects the results of compilation and emits an in-memory ELF object
    /// which is the serialized representation of all compiler artifacts.
    ///
    /// Note that ELF is used regardless of the target architecture.
    fn emit_obj(
        &self,
        module: &ModuleTranslation,
        types: &TypeTables,
        funcs: &CompiledFunctions,
        emit_dwarf: bool,
    ) -> Result<Vec<u8>>;

    /// Emits a small ELF object file in-memory which has two functions for the
    /// host-to-wasm and wasm-to-host trampolines for the wasm type given.
    fn emit_trampoline_obj(&self, ty: &WasmFuncType, host_fn: usize) -> Result<Vec<u8>>;

    /// Returns the target triple that this compiler is compiling for.
    fn triple(&self) -> &target_lexicon::Triple;

    /// Returns a list of configured settings for this compiler.
    fn flags(&self) -> HashMap<String, FlagValue>;

    /// Same as [`Compiler::flags`], but ISA-specific (a cranelift-ism)
    fn isa_flags(&self) -> HashMap<String, FlagValue>;
}

/// Value of a configured setting for a [`Compiler`]
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum FlagValue {
    /// Name of the value that has been configured for this setting.
    Enum(Cow<'static, str>),
    /// The numerical value of the configured settings.
    Num(u8),
    /// Whether the setting is on or off.
    Bool(bool),
}

impl fmt::Display for FlagValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Enum(v) => v.fmt(f),
            Self::Num(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
        }
    }
}
