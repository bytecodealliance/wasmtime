use crate::TrampolineKind;
use anyhow::{anyhow, Result};
use core::fmt::Formatter;
use cranelift_codegen::isa::{CallConv, IsaBuilder};
use cranelift_codegen::settings;
use cranelift_codegen::{Final, MachBufferFinalized, TextSectionBuilder};
use std::{
    error,
    fmt::{self, Debug, Display},
};
use target_lexicon::{Architecture, Triple};
use wasmparser::{FuncValidator, FunctionBody, ValidatorResources};
use wasmtime_environ::{ModuleTranslation, WasmFuncType};

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

/// Calling conventions supported by Winch. Winch supports the `Wasmtime*`
/// variations of the system's ABI calling conventions and an internal default
/// calling convention.
///
/// This enum is a reduced subset of the calling conventions defined in
/// [cranelift_codegen::isa::CallConv]. Introducing this enum makes it easier
/// to enforce the invariant of all the calling conventions supported by Winch.
pub enum CallingConvention {
    /// See [cranelift_codegen::isa::CallConv::WasmtimeSystemV]
    WasmtimeSystemV,
    /// See [cranelift_codegen::isa::CallConv::WindowsFastcall]
    WindowsFastcall,
    /// See [cranelift_codegen::isa::CallConv::AppleAarch64]
    AppleAarch64,
    /// The default calling convention for Winch. It largely follows SystemV
    /// for parameter and result handling. This calling convention is part of
    /// Winch's default ABI [crate::abi::ABI].
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
            CallingConvention::WasmtimeSystemV => true,
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
    fn is_default(&self) -> bool {
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
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<MachBufferFinalized<Final>>;

    /// Get the default calling convention of the underlying target triple.
    fn default_call_conv(&self) -> CallConv {
        CallConv::triple_default(&self.triple())
    }

    /// Derive Wasmtime's calling convention from the triple's default
    /// calling convention.
    fn wasmtime_call_conv(&self) -> CallingConvention {
        match self.default_call_conv() {
            CallConv::AppleAarch64 => CallingConvention::AppleAarch64,
            CallConv::SystemV => CallingConvention::WasmtimeSystemV,
            CallConv::WindowsFastcall => CallingConvention::WindowsFastcall,
            cc => unimplemented!("calling convention: {:?}", cc),
        }
    }

    /// Get the endianess of the underlying target triple.
    fn endianness(&self) -> target_lexicon::Endianness {
        self.triple().endianness().unwrap()
    }

    /// See `cranelift_codegen::isa::TargetIsa::create_systemv_cie`.
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        // By default, an ISA cannot create a System V CIE.
        None
    }

    /// See `cranelift_codegen::isa::TargetIsa::text_section_builder`.
    fn text_section_builder(&self, num_labeled_funcs: usize) -> Box<dyn TextSectionBuilder>;

    /// See `cranelift_codegen::isa::TargetIsa::function_alignment`.
    fn function_alignment(&self) -> u32;

    /// Compile a trampoline kind.
    ///
    /// This function, internally dispatches to the right trampoline to emit
    /// depending on the `kind` paramter.
    fn compile_trampoline(
        &self,
        ty: &WasmFuncType,
        kind: TrampolineKind,
    ) -> Result<MachBufferFinalized<Final>>;

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
