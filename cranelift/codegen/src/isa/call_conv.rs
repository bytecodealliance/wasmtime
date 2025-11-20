use crate::ir::Type;
use crate::ir::types;
use crate::settings::{self, LibcallCallConv};
use core::fmt;
use core::str;
use target_lexicon::{CallingConvention, Triple};

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// Calling convention identifiers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum CallConv {
    /// Best performance, not ABI-stable.
    Fast,
    /// Smallest caller code size, not ABI-stable.
    Cold,
    /// Supports tail calls, not ABI-stable except for exception
    /// payload registers.
    ///
    /// On exception resume, a caller to a `tail`-convention function
    /// assumes that the exception payload values are in the following
    /// registers (per platform):
    /// - x86-64: rax, rdx
    /// - aarch64: x0, x1
    /// - riscv64: a0, a1
    /// - pulley{32,64}: x0, x1
    //
    // Currently, this is basically sys-v except that callees pop stack
    // arguments, rather than callers. Expected to change even more in the
    // future, however!
    Tail,
    /// System V-style convention used on many platforms.
    SystemV,
    /// Windows "fastcall" convention, also used for x64 and ARM.
    WindowsFastcall,
    /// Mac aarch64 calling convention, which is a tweaked aarch64 ABI.
    AppleAarch64,
    /// Specialized convention for the probestack function.
    Probestack,
    /// The winch calling convention, not ABI-stable.
    ///
    /// The main difference to SystemV is that the winch calling convention
    /// defines no callee-save registers, and restricts the number of return
    /// registers to one integer, and one floating point.
    Winch,
    /// Calling convention for patchable-call instructions.
    ///
    /// This is designed for a very specific need: we want a *single*
    /// call instruction at our callsite, with no other setup, and we
    /// don't want any registers clobbered. This allows patchable
    /// callsites to be as unobtrusive as possible.
    ///
    /// The ABI is based on the native register-argument ABI on each
    /// respective platform, but puts severe restrictions on allowable
    /// signatures: only up to four arguments of integer type, and no
    /// return values. It does not support tail-calls, and disallows
    /// any extension modes on arguments.
    ///
    /// The ABI specifies that *no* registers, not even argument
    /// registers, are clobbered. This is pretty unique: it means that
    /// the call instruction will constrain regalloc to have any args
    /// in the right registers, but those registers will be preserved,
    /// so multiple patchable callsites can reuse those values. This
    /// further reduces the cost of the callsites.
    Patchable,
}

impl CallConv {
    /// Return the default calling convention for the given target triple.
    pub fn triple_default(triple: &Triple) -> Self {
        match triple.default_calling_convention() {
            // Default to System V for unknown targets because most everything
            // uses System V.
            Ok(CallingConvention::SystemV) | Err(()) => Self::SystemV,
            Ok(CallingConvention::AppleAarch64) => Self::AppleAarch64,
            Ok(CallingConvention::WindowsFastcall) => Self::WindowsFastcall,
            Ok(unimp) => unimplemented!("calling convention: {:?}", unimp),
        }
    }

    /// Returns the calling convention used for libcalls according to the current flags.
    pub fn for_libcall(flags: &settings::Flags, default_call_conv: CallConv) -> Self {
        match flags.libcall_call_conv() {
            LibcallCallConv::IsaDefault => default_call_conv,
            LibcallCallConv::Fast => Self::Fast,
            LibcallCallConv::Cold => Self::Cold,
            LibcallCallConv::SystemV => Self::SystemV,
            LibcallCallConv::WindowsFastcall => Self::WindowsFastcall,
            LibcallCallConv::AppleAarch64 => Self::AppleAarch64,
            LibcallCallConv::Probestack => Self::Probestack,
        }
    }

    /// Does this calling convention support tail calls?
    pub fn supports_tail_calls(&self) -> bool {
        match self {
            CallConv::Tail => true,
            _ => false,
        }
    }

    /// Does this calling convention support exceptions?
    pub fn supports_exceptions(&self) -> bool {
        match self {
            CallConv::Tail | CallConv::SystemV | CallConv::Winch => true,
            _ => false,
        }
    }

    /// What types do the exception payload value(s) have?
    ///
    /// Note that this function applies to the *callee* of a `try_call`
    /// instruction. The calling convention of the callee may differ from the
    /// caller, but the exceptional payload types available are defined by the
    /// callee calling convention.
    ///
    /// Also note that individual backends are responsible for reporting
    /// register destinations for exceptional types. Internally Cranelift
    /// asserts that the backend supports the exact same number of register
    /// destinations as this return value.
    pub fn exception_payload_types(&self, pointer_ty: Type) -> &[Type] {
        match self {
            CallConv::Tail | CallConv::SystemV => match pointer_ty {
                types::I32 => &[types::I32, types::I32],
                types::I64 => &[types::I64, types::I64],
                _ => unreachable!(),
            },
            _ => &[],
        }
    }
}

impl fmt::Display for CallConv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            Self::Fast => "fast",
            Self::Cold => "cold",
            Self::Tail => "tail",
            Self::SystemV => "system_v",
            Self::WindowsFastcall => "windows_fastcall",
            Self::AppleAarch64 => "apple_aarch64",
            Self::Probestack => "probestack",
            Self::Winch => "winch",
            Self::Patchable => "patchable",
        })
    }
}

impl str::FromStr for CallConv {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fast" => Ok(Self::Fast),
            "cold" => Ok(Self::Cold),
            "tail" => Ok(Self::Tail),
            "system_v" => Ok(Self::SystemV),
            "windows_fastcall" => Ok(Self::WindowsFastcall),
            "apple_aarch64" => Ok(Self::AppleAarch64),
            "probestack" => Ok(Self::Probestack),
            "winch" => Ok(Self::Winch),
            "patchable" => Ok(Self::Patchable),
            _ => Err(()),
        }
    }
}
