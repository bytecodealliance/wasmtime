//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::obj;
use crate::{
    DefinedFuncIndex, FilePos, FuncIndex, FunctionBodyData, ModuleTranslation, ModuleTypes,
    PrimaryMap, StackMap, Tunables, WasmError, WasmFuncType,
};
use anyhow::Result;
use object::write::{Object, SymbolId};
use object::{Architecture, BinaryFormat, FileFlags};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

/// Information about a function, such as trap information, address map,
/// and stack maps.
#[derive(Serialize, Deserialize, Default)]
#[allow(missing_docs)]
pub struct WasmFunctionInfo {
    pub start_srcloc: FilePos,
    pub stack_maps: Box<[StackMapInformation]>,
}

/// Description of where a function is located in the text section of a
/// compiled image.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct FunctionLoc {
    /// The byte offset from the start of the text section where this
    /// function starts.
    pub start: u32,
    /// The byte length of this function's function body.
    pub length: u32,
}

/// The offset within a function of a GC safepoint, and its associated stack
/// map.
#[derive(Serialize, Deserialize, Debug)]
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

/// Implementation of an incremental compilation's key/value cache store.
///
/// In theory, this could just be Cranelift's `CacheKvStore` trait, but it is not as we want to
/// make sure that wasmtime isn't too tied to Cranelift internals (and as a matter of fact, we
/// can't depend on the Cranelift trait here).
pub trait CacheStore: Send + Sync + std::fmt::Debug {
    /// Try to retrieve an arbitrary cache key entry, and returns a reference to bytes that were
    /// inserted via `Self::insert` before.
    fn get(&self, key: &[u8]) -> Option<Cow<[u8]>>;

    /// Given an arbitrary key and bytes, stores them in the cache.
    ///
    /// Returns false when insertion in the cache failed.
    fn insert(&self, key: &[u8], value: Vec<u8>) -> bool;
}

/// Abstract trait representing the ability to create a `Compiler` below.
///
/// This is used in Wasmtime to separate compiler implementations, currently
/// mostly used to separate Cranelift from Wasmtime itself.
pub trait CompilerBuilder: Send + Sync + fmt::Debug {
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

    /// Enables Cranelift's incremental compilation cache, using the given `CacheStore`
    /// implementation.
    fn enable_incremental_compilation(&mut self, cache_store: Arc<dyn CacheStore>);

    /// Builds a new [`Compiler`] object from this configuration.
    fn build(&self) -> Result<Box<dyn Compiler>>;
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

/// Types of objects that can be created by `Compiler::object`
pub enum ObjectKind {
    /// A core wasm compilation artifact
    Module,
    /// A component compilation artifact
    Component,
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
        types: &ModuleTypes,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError>;

    /// Creates a function of type `VMTrampoline` which will then call the
    /// function pointer argument which has the `ty` type provided.
    fn compile_host_to_wasm_trampoline(
        &self,
        ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Appends a list of compiled functions to an in-memory object.
    ///
    /// This function will receive the same `Box<dyn Ayn>` produced as part of
    /// compilation from functions like `compile_function`,
    /// compile_host_to_wasm_trampoline`, and other component-related shims.
    /// Internally this will take all of these functions and add information to
    /// the object such as:
    ///
    /// * Compiled code in a `.text` section
    /// * Unwind information in Wasmtime-specific sections
    /// * Relocations, if necessary, for the text section
    ///
    /// Each function is accompanied with its desired symbol name and the return
    /// value of this function is the symbol for each function as well as where
    /// each function was placed within the object.
    ///
    /// The `resolve_reloc` argument is intended to resolving relocations
    /// between function, chiefly resolving intra-module calls within one core
    /// wasm module. The closure here takes two arguments: first the index
    /// within `funcs` that is being resolved and next the `FuncIndex` which is
    /// the relocation target to resolve. The return value is an index within
    /// `funcs` that the relocation points to.
    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        tunables: &Tunables,
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>>;

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
    ) -> Result<(FunctionLoc, FunctionLoc)>;

    /// Creates a new `Object` file which is used to build the results of a
    /// compilation into.
    ///
    /// The returned object file will have an appropriate
    /// architecture/endianness for `self.triple()`, but at this time it is
    /// always an ELF file, regardless of target platform.
    fn object(&self, kind: ObjectKind) -> Result<Object<'static>> {
        use target_lexicon::Architecture::*;

        let triple = self.triple();
        let mut obj = Object::new(
            BinaryFormat::Elf,
            match triple.architecture {
                X86_32(_) => Architecture::I386,
                X86_64 => Architecture::X86_64,
                Arm(_) => Architecture::Arm,
                Aarch64(_) => Architecture::Aarch64,
                S390x => Architecture::S390x,
                Riscv64(_) => Architecture::Riscv64,
                architecture => {
                    anyhow::bail!("target architecture {:?} is unsupported", architecture,);
                }
            },
            match triple.endianness().unwrap() {
                target_lexicon::Endianness::Little => object::Endianness::Little,
                target_lexicon::Endianness::Big => object::Endianness::Big,
            },
        );
        obj.flags = FileFlags::Elf {
            os_abi: obj::ELFOSABI_WASMTIME,
            e_flags: match kind {
                ObjectKind::Module => obj::EF_WASMTIME_MODULE,
                ObjectKind::Component => obj::EF_WASMTIME_COMPONENT,
            },
            abi_version: 0,
        };
        Ok(obj)
    }

    /// Returns the target triple that this compiler is compiling for.
    fn triple(&self) -> &target_lexicon::Triple;

    /// Returns the alignment necessary to align values to the page size of the
    /// compilation target. Note that this may be an upper-bound where the
    /// alignment is larger than necessary for some platforms since it may
    /// depend on the platform's runtime configuration.
    fn page_size_align(&self) -> u64;

    /// Returns a list of configured settings for this compiler.
    fn flags(&self) -> BTreeMap<String, FlagValue>;

    /// Same as [`Compiler::flags`], but ISA-specific (a cranelift-ism)
    fn isa_flags(&self) -> BTreeMap<String, FlagValue>;

    /// Get a flag indicating whether branch protection is enabled.
    fn is_branch_protection_enabled(&self) -> bool;

    /// Returns a suitable compiler usable for component-related compliations.
    ///
    /// Note that the `ComponentCompiler` trait can also be implemented for
    /// `Self` in which case this function would simply return `self`.
    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn crate::component::ComponentCompiler;

    /// Appends generated DWARF sections to the `obj` specified for the compiled
    /// functions.
    fn append_dwarf(
        &self,
        obj: &mut Object<'_>,
        translation: &ModuleTranslation<'_>,
        funcs: &PrimaryMap<DefinedFuncIndex, (SymbolId, &(dyn Any + Send))>,
    ) -> Result<()>;
}

/// Value of a configured setting for a [`Compiler`]
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
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
