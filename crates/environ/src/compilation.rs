//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::{
    DefinedFuncIndex, FunctionAddressMap, FunctionBodyData, ModuleTranslation, PrimaryMap,
    StackMap, Tunables, TypeTables, WasmError, WasmFuncType,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Information about a function, such as trap information, address map,
/// and stack maps.
#[derive(Serialize, Deserialize, Clone, Default)]
#[allow(missing_docs)]
pub struct FunctionInfo {
    pub traps: Vec<TrapInformation>,
    pub address_map: FunctionAddressMap,
    pub stack_maps: Vec<StackMapInformation>,
}

/// Information about trap.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code. It is relative to the beginning of the function.
    pub code_offset: u32,
    /// Code of the trap.
    pub trap_code: TrapCode,
}

/// A trap code describing the reason for a trap.
///
/// All trap instructions have an explicit trap code.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum TrapCode {
    /// The current stack space was exhausted.
    StackOverflow,

    /// A `heap_addr` instruction detected an out-of-bounds error.
    ///
    /// Note that not all out-of-bounds heap accesses are reported this way;
    /// some are detected by a segmentation fault on the heap unmapped or
    /// offset-guard pages.
    HeapOutOfBounds,

    /// A wasm atomic operation was presented with a not-naturally-aligned linear-memory address.
    HeapMisaligned,

    /// A `table_addr` instruction detected an out-of-bounds error.
    TableOutOfBounds,

    /// Indirect call to a null table entry.
    IndirectCallToNull,

    /// Signature mismatch on indirect call.
    BadSignature,

    /// An integer arithmetic operation caused an overflow.
    IntegerOverflow,

    /// An integer division by zero.
    IntegerDivisionByZero,

    /// Failed float-to-int conversion.
    BadConversionToInteger,

    /// Code that was supposed to have been unreachable was reached.
    UnreachableCodeReached,

    /// Execution has potentially run too long and may be interrupted.
    /// This trap is resumable.
    Interrupt,
}

/// The offset within a function of a GC safepoint, and its associated stack
/// map.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StackMapInformation {
    /// The offset of the GC safepoint within the function's native code. It is
    /// relative to the beginning of the function.
    pub code_offset: u32,

    /// The stack map for identifying live GC refs at the GC safepoint.
    pub stack_map: StackMap,
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
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Collects the results of compilation and emits an in-memory ELF object
    /// which is the serialized representation of all compiler artifacts.
    ///
    /// Note that ELF is used regardless of the target architecture.
    fn emit_obj(
        &self,
        module: &ModuleTranslation,
        types: &TypeTables,
        funcs: PrimaryMap<DefinedFuncIndex, Box<dyn Any + Send>>,
        emit_dwarf: bool,
    ) -> Result<(Vec<u8>, PrimaryMap<DefinedFuncIndex, FunctionInfo>)>;

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
