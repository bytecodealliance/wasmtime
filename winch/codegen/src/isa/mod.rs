use crate::BuiltinFunctions;
use anyhow::{anyhow, Result};
use core::fmt::Formatter;
use cranelift_codegen::isa::unwind::{UnwindInfo, UnwindInfoKind};
use cranelift_codegen::isa::{CallConv, IsaBuilder};
use cranelift_codegen::settings;
use cranelift_codegen::{Final, MachBufferFinalized, TextSectionBuilder};
use std::{
    error,
    fmt::{self, Debug, Display},
};
use target_lexicon::{Architecture, Triple};
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources};
use wasmtime_cranelift::CompiledFunction;
use wasmtime_environ::{ModuleTranslation, ModuleTypesBuilder, WasmFuncType};

#[cfg(feature = "x64")]
pub(crate) mod x64;

#[cfg(feature = "arm64")]
pub(crate) mod aarch64;

pub(crate) mod reg;

macro_rules! isa_builder {
    ($name: ident, $cfg_terms: tt, $triple: ident) => {{
        #[cfg $cfg_terms]
        {
            Ok($name::isa_builder($triple))
        }
        #[cfg(not $cfg_terms)]
        {
            Err(anyhow!(LookupError::SupportDisabled))
        }
    }};
}

pub type Builder = IsaBuilder<Result<Box<dyn TargetIsa>>>;

/// Look for an ISA builder for the given target triple.
pub fn lookup(triple: Triple) -> Result<Builder> {
    match triple.architecture {
        Architecture::X86_64 => {
            isa_builder!(x64, (feature = "x64"), triple)
        }
        Architecture::Aarch64 { .. } => {
            isa_builder!(aarch64, (feature = "arm64"), triple)
        }

        _ => Err(anyhow!(LookupError::Unsupported)),
    }
}

impl error::Error for LookupError {}
impl Display for LookupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LookupError::Unsupported => write!(f, "This target is not supported yet"),
            LookupError::SupportDisabled => write!(f, "Support for this target was disabled"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum LookupError {
    Unsupported,
    // This directive covers the case in which the consumer
    // enables the `all-arch` feature; in such case, this variant
    // will never be used. This is most likely going to change
    // in the future; this is one of the simplest options for now.
    #[allow(dead_code)]
    SupportDisabled,
}

/// Calling conventions supported by Winch. Winch supports a variation of
/// the calling conventions defined in this enum plus an internal default
/// calling convention.
///
/// This enum is a reduced subset of the calling conventions defined in
/// [cranelift_codegen::isa::CallConv]. Introducing this enum makes it easier
/// to enforce the invariant of all the calling conventions supported by Winch.
///
/// The main difference between the system calling conventions defined in
/// this enum and their native counterparts is how multiple returns are handled.
/// Given that Winch is not meant to be a standalone code generator, the code
/// it generates is tightly coupled to how Wasmtime expects multiple returns
/// to be handled: the first return in a register, dictated by the calling
/// convention and the rest, if any, via a return pointer.
#[derive(Copy, Clone, Debug)]
pub enum CallingConvention {
    /// See [cranelift_codegen::isa::CallConv::WasmtimeSystemV]
    SystemV,
    /// See [cranelift_codegen::isa::CallConv::WindowsFastcall]
    WindowsFastcall,
    /// See [cranelift_codegen::isa::CallConv::AppleAarch64]
    AppleAarch64,
    /// The default calling convention for Winch. It largely follows SystemV
    /// for parameter and result handling. This calling convention is part of
    /// Winch's default ABI `crate::abi::ABI`.
    Default,
}

impl CallingConvention {
    /// Returns true if the current calling convention is `WasmtimeFastcall`.
    fn is_fastcall(&self) -> bool {
        match &self {
            CallingConvention::WindowsFastcall => true,
            _ => false,
        }
    }

    /// Returns true if the current calling convention is `WasmtimeSystemV`.
    fn is_systemv(&self) -> bool {
        match &self {
            CallingConvention::SystemV => true,
            _ => false,
        }
    }

    /// Returns true if the current calling convention is `WasmtimeAppleAarch64`.
    fn is_apple_aarch64(&self) -> bool {
        match &self {
            CallingConvention::AppleAarch64 => true,
            _ => false,
        }
    }

    /// Returns true if the current calling convention is `Default`.
    pub fn is_default(&self) -> bool {
        match &self {
            CallingConvention::Default => true,
            _ => false,
        }
    }
}

/// A trait representing commonalities between the supported
/// instruction set architectures.
pub trait TargetIsa: Send + Sync {
    /// Get the name of the ISA.
    fn name(&self) -> &'static str;

    /// Get the target triple of the ISA.
    fn triple(&self) -> &Triple;

    /// Get the ISA-independent flags that were used to make this trait object.
    fn flags(&self) -> &settings::Flags;

    /// Get the ISA-dependent flag values that were used to make this trait object.
    fn isa_flags(&self) -> Vec<settings::Value>;

    /// Get a flag indicating whether branch protection is enabled.
    fn is_branch_protection_enabled(&self) -> bool {
        false
    }

    /// Compile a function.
    fn compile_function(
        &self,
        sig: &WasmFuncType,
        body: &FunctionBody,
        translation: &ModuleTranslation,
        types: &ModuleTypesBuilder,
        builtins: &mut BuiltinFunctions,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<CompiledFunction>;

    /// Get the default calling convention of the underlying target triple.
    fn default_call_conv(&self) -> CallConv {
        CallConv::triple_default(&self.triple())
    }

    /// Derive Wasmtime's calling convention from the triple's default
    /// calling convention.
    fn wasmtime_call_conv(&self) -> CallingConvention {
        match self.default_call_conv() {
            CallConv::AppleAarch64 => CallingConvention::AppleAarch64,
            CallConv::SystemV => CallingConvention::SystemV,
            CallConv::WindowsFastcall => CallingConvention::WindowsFastcall,
            cc => unimplemented!("calling convention: {:?}", cc),
        }
    }

    /// Get the endianness of the underlying target triple.
    fn endianness(&self) -> target_lexicon::Endianness {
        self.triple().endianness().unwrap()
    }

    fn emit_unwind_info(
        &self,
        _result: &MachBufferFinalized<Final>,
        _kind: UnwindInfoKind,
    ) -> Result<Option<UnwindInfo>>;

    /// See `cranelift_codegen::isa::TargetIsa::create_systemv_cie`.
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        // By default, an ISA cannot create a System V CIE.
        None
    }

    /// See `cranelift_codegen::isa::TargetIsa::text_section_builder`.
    fn text_section_builder(&self, num_labeled_funcs: usize) -> Box<dyn TextSectionBuilder>;

    /// See `cranelift_codegen::isa::TargetIsa::function_alignment`.
    fn function_alignment(&self) -> u32;

    /// Returns the pointer width of the ISA in bytes.
    fn pointer_bytes(&self) -> u8 {
        let width = self.triple().pointer_width().unwrap();
        width.bytes()
    }
}

impl Debug for &dyn TargetIsa {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Target ISA {{ triple: {:?}, calling convention: {:?} }}",
            self.triple(),
            self.default_call_conv()
        )
    }
}
