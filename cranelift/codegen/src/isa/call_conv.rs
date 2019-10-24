use crate::isa::TargetIsa;
use crate::settings::LibcallCallConv;
use core::fmt;
use core::str;
use target_lexicon::{CallingConvention, Triple};

/// Calling convention identifiers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CallConv {
    /// Best performance, not ABI-stable
    Fast,
    /// Smallest caller code size, not ABI-stable
    Cold,
    /// System V-style convention used on many platforms
    SystemV,
    /// Windows "fastcall" convention, also used for x64 and ARM
    WindowsFastcall,
    /// SpiderMonkey WebAssembly convention on systems using natively SystemV
    BaldrdashSystemV,
    /// SpiderMonkey WebAssembly convention on Windows
    BaldrdashWindows,
    /// Specialized convention for the probestack function
    Probestack,
}

impl CallConv {
    /// Return the default calling convention for the given target triple.
    pub fn triple_default(triple: &Triple) -> Self {
        match triple.default_calling_convention() {
            // Default to System V for unknown targets because most everything
            // uses System V.
            Ok(CallingConvention::SystemV) | Err(()) => Self::SystemV,
            Ok(CallingConvention::WindowsFastcall) => Self::WindowsFastcall,
            Ok(unimp) => unimplemented!("calling convention: {:?}", unimp),
        }
    }

    /// Returns the calling convention used for libcalls for the given ISA.
    pub fn for_libcall(isa: &dyn TargetIsa) -> Self {
        match isa.flags().libcall_call_conv() {
            LibcallCallConv::IsaDefault => isa.default_call_conv(),
            LibcallCallConv::Fast => Self::Fast,
            LibcallCallConv::Cold => Self::Cold,
            LibcallCallConv::SystemV => Self::SystemV,
            LibcallCallConv::WindowsFastcall => Self::WindowsFastcall,
            LibcallCallConv::BaldrdashSystemV => Self::BaldrdashSystemV,
            LibcallCallConv::BaldrdashWindows => Self::BaldrdashWindows,
            LibcallCallConv::Probestack => Self::Probestack,
        }
    }

    /// Is the calling convention extending the Windows Fastcall ABI?
    pub fn extends_windows_fastcall(self) -> bool {
        match self {
            Self::WindowsFastcall | Self::BaldrdashWindows => true,
            _ => false,
        }
    }

    /// Is the calling convention extending the Baldrdash ABI?
    pub fn extends_baldrdash(self) -> bool {
        match self {
            Self::BaldrdashSystemV | Self::BaldrdashWindows => true,
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
            Self::BaldrdashSystemV => "baldrdash_system_v",
            Self::BaldrdashWindows => "baldrdash_windows",
            Self::Probestack => "probestack",
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
            "baldrdash_system_v" => Ok(Self::BaldrdashSystemV),
            "baldrdash_windows" => Ok(Self::BaldrdashWindows),
            "probestack" => Ok(Self::Probestack),
            _ => Err(()),
        }
    }
}
