//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::{
    DefinedFuncIndex, FilePos, FunctionBodyData, ModuleTranslation, PrimaryMap, SignatureIndex,
    StackMap, Tunables, TypeTables, WasmError, WasmFuncType,
};
use anyhow::Result;
use object::write::Object;
use object::{Architecture, BinaryFormat};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

/// Information about a function, such as trap information, address map,
/// and stack maps.
#[derive(Serialize, Deserialize, Clone, Default)]
#[allow(missing_docs)]
pub struct FunctionInfo {
    pub start_srcloc: FilePos,
    pub stack_maps: Vec<StackMapInformation>,

    /// Offset in the text section of where this function starts.
    pub start: u64,
    /// The size of the compiled function, in bytes.
    pub length: u32,
}

/// Information about a compiled trampoline which the host can call to enter
/// wasm.
#[derive(Serialize, Deserialize, Clone)]
#[allow(missing_docs)]
pub struct Trampoline {
    /// The signature this trampoline is for
    pub signature: SignatureIndex,

    /// Offset in the text section of where this function starts.
    pub start: u64,
    /// The size of the compiled function, in bytes.
    pub length: u32,
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

    /// Collects the results of compilation into an in-memory object.
    ///
    /// This function will receive the same `Box<dyn Ayn>` produced as part of
    /// `compile_function`, as well as the general compilation environment with
    /// the translation/types. This method is expected to populate information
    /// in the object file such as:
    ///
    /// * Compiled code in a `.text` section
    /// * Unwind information in Wasmtime-specific sections
    /// * DWARF debugging information for the host, if `emit_dwarf` is `true`
    ///   and the compiler supports it.
    /// * Relocations, if necessary, for the text section
    ///
    /// The final result of compilation will contain more sections inserted by
    /// the compiler-agnostic runtime.
    fn emit_obj(
        &self,
        module: &ModuleTranslation,
        types: &TypeTables,
        funcs: PrimaryMap<DefinedFuncIndex, Box<dyn Any + Send>>,
        emit_dwarf: bool,
        obj: &mut Object<'static>,
    ) -> Result<(PrimaryMap<DefinedFuncIndex, FunctionInfo>, Vec<Trampoline>)>;

    /// Inserts two functions for host-to-wasm and wasm-to-host trampolines into
    /// the `obj` provided.
    ///
    /// This will configure the same sections as `emit_obj`, but will likely be
    /// much smaller. The two returned `Trampoline` structures describe where to
    /// find the host-to-wasm and wasm-to-host trampolines in the text section,
    /// respectively.
    fn emit_trampoline_obj(
        &self,
        ty: &WasmFuncType,
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(Trampoline, Trampoline)>;

    /// Creates a new `Object` file which is used to build the results of a
    /// compilation into.
    ///
    /// The returned object file will have an appropriate
    /// architecture/endianness for `self.triple()`, but at this time it is
    /// always an ELF file, regardless of target platform.
    fn object(&self) -> Result<Object<'static>> {
        use target_lexicon::Architecture::*;

        let triple = self.triple();
        Ok(Object::new(
            BinaryFormat::Elf,
            match triple.architecture {
                X86_32(_) => Architecture::I386,
                X86_64 => Architecture::X86_64,
                Arm(_) => Architecture::Arm,
                Aarch64(_) => Architecture::Aarch64,
                S390x => Architecture::S390x,
                architecture => {
                    anyhow::bail!("target architecture {:?} is unsupported", architecture,);
                }
            },
            match triple.endianness().unwrap() {
                target_lexicon::Endianness::Little => object::Endianness::Little,
                target_lexicon::Endianness::Big => object::Endianness::Big,
            },
        ))
    }

    /// Returns the target triple that this compiler is compiling for.
    fn triple(&self) -> &target_lexicon::Triple;

    /// Returns a list of configured settings for this compiler.
    fn flags(&self) -> BTreeMap<String, FlagValue>;

    /// Same as [`Compiler::flags`], but ISA-specific (a cranelift-ism)
    fn isa_flags(&self) -> BTreeMap<String, FlagValue>;
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
