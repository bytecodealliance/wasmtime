//! A `Compilation` contains the compiled function bodies for a WebAssembly
//! module.

use crate::{obj, Tunables};
use crate::{
    BuiltinFunctionIndex, DefinedFuncIndex, FlagValue, FuncIndex, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypesBuilder, ObjectKind, PrimaryMap, WasmError, WasmFuncType,
    WasmFunctionInfo,
};
use anyhow::Result;
use object::write::{Object, SymbolId};
use object::{Architecture, BinaryFormat, FileFlags};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use std::path;
use std::sync::Arc;
use thiserror::Error;

mod address_map;
mod module_artifacts;
mod trap_encoding;

pub use self::address_map::*;
pub use self::module_artifacts::*;
pub use self::trap_encoding::*;

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

/// What relocations can be applied against.
///
/// Each wasm function may refer to various other `RelocationTarget` entries.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RelocationTarget {
    /// This is a reference to another defined wasm function in the same module.
    Wasm(FuncIndex),
    /// This is a reference to a trampoline for a builtin function.
    Builtin(BuiltinFunctionIndex),
    /// A compiler-generated libcall.
    HostLibcall(obj::LibCall),
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

    /// Enables clif output in the directory specified.
    fn clif_dir(&mut self, _path: &path::Path) -> Result<()> {
        anyhow::bail!("clif output not supported");
    }

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
    ///
    /// This will return an error if the compiler does not support incremental compilation.
    fn enable_incremental_compilation(&mut self, cache_store: Arc<dyn CacheStore>) -> Result<()>;

    /// Set the tunables for this compiler.
    fn set_tunables(&mut self, tunables: Tunables) -> Result<()>;

    /// Builds a new [`Compiler`] object from this configuration.
    fn build(&self) -> Result<Box<dyn Compiler>>;

    /// Enables or disables wmemcheck during runtime according to the wmemcheck CLI flag.
    fn wmemcheck(&mut self, _enable: bool) {}
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
    ///
    /// This function returns a tuple:
    ///
    /// 1. Metadata about the wasm function itself.
    /// 2. The function itself, as an `Any` to get downcasted later when passed
    ///    to `append_code`.
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError>;

    /// Compile a trampoline for an array-call host function caller calling the
    /// `index`th Wasm function.
    ///
    /// The trampoline should save the necessary state to record the
    /// host-to-Wasm transition (e.g. registers used for fast stack walking).
    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Compile a trampoline for a native-call host function caller calling the
    /// `index`th Wasm function.
    ///
    /// The trampoline should save the necessary state to record the
    /// host-to-Wasm transition (e.g. registers used for fast stack walking).
    fn compile_native_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Compile a trampoline for a Wasm caller calling a native callee with the
    /// given signature.
    ///
    /// The trampoline should save the necessary state to record the
    /// Wasm-to-host transition (e.g. registers used for fast stack walking).
    fn compile_wasm_to_native_trampoline(
        &self,
        wasm_func_ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Creates a tramopline that can be used to call Wasmtime's implementation
    /// of the builtin function specified by `index`.
    ///
    /// The trampoline created can technically have any ABI but currently has
    /// the native ABI. This will then perform all the necessary duties of an
    /// exit trampoline from wasm and then perform the actual dispatch to the
    /// builtin function. Builtin functions in Wasmtime are stored in an array
    /// in all `VMContext` pointers, so the call to the host is an indirect
    /// call.
    fn compile_wasm_to_builtin(
        &self,
        index: BuiltinFunctionIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError>;

    /// Returns the list of relocations required for a function from one of the
    /// previous `compile_*` functions above.
    fn compiled_function_relocation_targets<'a>(
        &'a self,
        func: &'a dyn Any,
    ) -> Box<dyn Iterator<Item = RelocationTarget> + 'a>;

    /// Appends a list of compiled functions to an in-memory object.
    ///
    /// This function will receive the same `Box<dyn Any>` produced as part of
    /// compilation from functions like `compile_function`,
    /// `compile_host_to_wasm_trampoline`, and other component-related shims.
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
    /// wasm module. The closure here takes two arguments:
    ///
    /// 1. First, the index within `funcs` that is being resolved,
    ///
    /// 2. and next the `RelocationTarget` which is the relocation target to
    /// resolve.
    ///
    /// The return value is an index within `funcs` that the relocation points
    /// to.
    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        resolve_reloc: &dyn Fn(usize, RelocationTarget) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>>;

    /// Inserts two trampolines into `obj` for a array-call host function:
    ///
    /// 1. A wasm-call trampoline: A trampoline that takes arguments in their
    ///    wasm-call locations, moves them to their array-call locations, calls
    ///    the array-call host function, and finally moves the return values
    ///    from the array-call locations to the wasm-call return
    ///    locations. Additionally, this trampoline manages the wasm-to-host
    ///    state transition for the runtime.
    ///
    /// 2. A native-call trampoline: A trampoline that takes arguments in their
    ///    native-call locations, moves them to their array-call locations,
    ///    calls the array-call host function, and finally moves the return
    ///    values from the array-call locations to the native-call return
    ///    locations. Does not need to manage any wasm/host state transitions,
    ///    since both caller and callee are on the host side.
    ///
    /// This will configure the same sections as `append_code`, but will likely
    /// be much smaller.
    ///
    /// The two returned `FunctionLoc` structures describe where to find these
    /// trampolines in the text section, respectively.
    ///
    /// These trampolines are only valid for in-process JIT usage. They bake in
    /// the function pointer to the host code.
    fn emit_trampolines_for_array_call_host_func(
        &self,
        ty: &WasmFuncType,
        // Actually `host_fn: VMArrayCallFunction` but that type is not
        // available in `wasmtime-environ`.
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
    fn page_size_align(&self) -> u64 {
        use target_lexicon::*;
        match (self.triple().operating_system, self.triple().architecture) {
            (
                OperatingSystem::MacOSX { .. }
                | OperatingSystem::Darwin
                | OperatingSystem::Ios
                | OperatingSystem::Tvos,
                Architecture::Aarch64(..),
            ) => 0x4000,
            // 64 KB is the maximal page size (i.e. memory translation granule size)
            // supported by the architecture and is used on some platforms.
            (_, Architecture::Aarch64(..)) => 0x10000,
            _ => 0x1000,
        }
    }

    /// Returns a list of configured settings for this compiler.
    fn flags(&self) -> Vec<(&'static str, FlagValue<'static>)>;

    /// Same as [`Compiler::flags`], but ISA-specific (a cranelift-ism)
    fn isa_flags(&self) -> Vec<(&'static str, FlagValue<'static>)>;

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
        dwarf_package_bytes: Option<&[u8]>,
        tunables: &Tunables,
    ) -> Result<()>;

    /// Creates a new System V Common Information Entry for the ISA.
    ///
    /// Returns `None` if the ISA does not support System V unwind information.
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        // By default, an ISA cannot create a System V CIE.
        None
    }
}
