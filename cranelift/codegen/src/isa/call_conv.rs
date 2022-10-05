use crate::settings::{self, LibcallCallConv};
use core::fmt;
use core::str;
use target_lexicon::{CallingConvention, Triple};

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Calling convention identifiers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum CallConv {
    /// Best performance, not ABI-stable.
    Fast,
    /// Smallest caller code size, not ABI-stable.
    Cold,
    /// System V-style convention used on many platforms.
    SystemV,
    /// Windows "fastcall" convention, also used for x64 and ARM.
    WindowsFastcall,
    /// Mac aarch64 calling convention, which is a tweaked aarch64 ABI.
    AppleAarch64,
    /// Specialized convention for the probestack function.
    Probestack,
    /// Wasmtime equivalent of SystemV, not ABI-stable.
    ///
    /// Currently only differs in how multiple return values are handled,
    /// returning the first return value in a register and everything else
    /// through a return-pointer.
    WasmtimeSystemV,
    /// Wasmtime equivalent of WindowsFastcall, not ABI-stable.
    ///
    /// Differs from fastcall in the same way as `WasmtimeSystemV`.
    WasmtimeFastcall,
    /// Wasmtime equivalent of AppleAarch64, not ABI-stable.
    ///
    /// Differs from apple-aarch64 in the same way as `WasmtimeSystemV`.
    WasmtimeAppleAarch64,
    /// SystemV riscv have different ABI variants.
    SystemVRiscv(RiscvFloatCallConv),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiscvFloatCallConv {
    Soft,
    Single,
    Double,
    Quad,
}

impl RiscvFloatCallConv {
    /// Save me from rust ownership compile error.
    /// So the `system_v_riscv_` comes.
    pub fn system_v_riscv_v_abi_name(self) -> &'static str {
        match self {
            Self::Soft => "system_v_riscv_soft",
            Self::Single => "system_v_riscv_single",
            Self::Double => "system_v_riscv_double",
            Self::Quad => "system_v_riscv_quad",
        }
    }
    pub fn max_bits(self) -> u32 {
        match self {
            // 0 means don't use float register.
            Self::Soft => 0,
            Self::Single => 32,
            Self::Double => 64,
            Self::Quad => 128,
        }
    }
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

    /// Is the calling convention extending the Windows Fastcall ABI?
    pub fn extends_windows_fastcall(self) -> bool {
        match self {
            Self::WindowsFastcall | Self::WasmtimeFastcall => true,
            _ => false,
        }
    }

    /// Is the calling convention extending the Apple aarch64 ABI?
    pub fn extends_apple_aarch64(self) -> bool {
        match self {
            Self::AppleAarch64 | Self::WasmtimeAppleAarch64 => true,
            _ => false,
        }
    }

    /// Is the calling convention extending the Wasmtime ABI?
    pub fn extends_wasmtime(self) -> bool {
        match self {
            Self::WasmtimeSystemV | Self::WasmtimeFastcall | Self::WasmtimeAppleAarch64 => true,
            _ => false,
        }
    }
}

impl fmt::Display for CallConv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            Self::Fast => "fast",
            Self::Cold => "cold",
            Self::SystemV => "system_v",
            Self::WindowsFastcall => "windows_fastcall",
            Self::AppleAarch64 => "apple_aarch64",
            Self::Probestack => "probestack",
            Self::WasmtimeSystemV => "wasmtime_system_v",
            Self::WasmtimeFastcall => "wasmtime_fastcall",
            Self::WasmtimeAppleAarch64 => "wasmtime_apple_aarch64",
            Self::SystemVRiscv(f) => f.system_v_riscv_v_abi_name(),
        })
    }
}

impl str::FromStr for CallConv {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fast" => Ok(Self::Fast),
            "cold" => Ok(Self::Cold),
            "system_v" => Ok(Self::SystemV),
            "windows_fastcall" => Ok(Self::WindowsFastcall),
            "apple_aarch64" => Ok(Self::AppleAarch64),
            "probestack" => Ok(Self::Probestack),
            "wasmtime_system_v" => Ok(Self::WasmtimeSystemV),
            "wasmtime_fastcall" => Ok(Self::WasmtimeFastcall),
            "wasmtime_apple_aarch64" => Ok(Self::WasmtimeAppleAarch64),
            "system_v_riscv_soft" => Ok(Self::SystemVRiscv(RiscvFloatCallConv::Soft)),
            "system_v_riscv_single" => Ok(Self::SystemVRiscv(RiscvFloatCallConv::Single)),
            "system_v_riscv_double" => Ok(Self::SystemVRiscv(RiscvFloatCallConv::Double)),
            "system_v_riscv_quad" => Ok(Self::SystemVRiscv(RiscvFloatCallConv::Quad)),
            _ => Err(()),
        }
    }
}
