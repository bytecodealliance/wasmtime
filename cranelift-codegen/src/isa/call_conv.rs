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
            Ok(CallingConvention::SystemV) | Err(()) => CallConv::SystemV,
            Ok(CallingConvention::WindowsFastcall) => CallConv::WindowsFastcall,
            Ok(unimp) => unimplemented!("calling convention: {:?}", unimp),
        }
    }

    /// Returns the calling convention used for libcalls for the given ISA.
    pub fn for_libcall(isa: &dyn TargetIsa) -> Self {
        match isa.flags().libcall_call_conv() {
            LibcallCallConv::IsaDefault => isa.default_call_conv(),
            LibcallCallConv::Fast => CallConv::Fast,
            LibcallCallConv::Cold => CallConv::Cold,
            LibcallCallConv::SystemV => CallConv::SystemV,
            LibcallCallConv::WindowsFastcall => CallConv::WindowsFastcall,
            LibcallCallConv::BaldrdashSystemV => CallConv::BaldrdashSystemV,
            LibcallCallConv::BaldrdashWindows => CallConv::BaldrdashWindows,
            LibcallCallConv::Probestack => CallConv::Probestack,
        }
    }

    /// Is the calling convention extending the Windows Fastcall ABI?
    pub fn extends_windows_fastcall(&self) -> bool {
        match self {
            CallConv::WindowsFastcall | CallConv::BaldrdashWindows => true,
            _ => false,
        }
    }

    /// Is the calling convention extending the Baldrdash ABI?
    pub fn extends_baldrdash(&self) -> bool {
        match self {
            CallConv::BaldrdashSystemV | CallConv::BaldrdashWindows => true,
            _ => false,
        }
    }
}

impl fmt::Display for CallConv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            CallConv::Fast => "fast",
            CallConv::Cold => "cold",
            CallConv::SystemV => "system_v",
            CallConv::WindowsFastcall => "windows_fastcall",
            CallConv::BaldrdashSystemV => "baldrdash_system_v",
            CallConv::BaldrdashWindows => "baldrdash_windows",
            CallConv::Probestack => "probestack",
        })
    }
}

impl str::FromStr for CallConv {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fast" => Ok(CallConv::Fast),
            "cold" => Ok(CallConv::Cold),
            "system_v" => Ok(CallConv::SystemV),
            "windows_fastcall" => Ok(CallConv::WindowsFastcall),
            "baldrdash_system_v" => Ok(CallConv::BaldrdashSystemV),
            "baldrdash_windows" => Ok(CallConv::BaldrdashWindows),
            "probestack" => Ok(CallConv::Probestack),
            _ => Err(()),
        }
    }
}
