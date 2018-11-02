use std::fmt;
use std::str;
use target_lexicon::{OperatingSystem, Triple};

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
    /// SpiderMonkey WebAssembly convention
    Baldrdash,
    /// Specialized convention for the probestack function
    Probestack,
}

impl CallConv {
    /// Return the default calling convention for the given target triple.
    pub fn default_for_triple(triple: &Triple) -> Self {
        match triple.operating_system {
            OperatingSystem::Unknown
            | OperatingSystem::Bitrig
            | OperatingSystem::Cloudabi
            | OperatingSystem::Darwin
            | OperatingSystem::Dragonfly
            | OperatingSystem::Freebsd
            | OperatingSystem::Fuchsia
            | OperatingSystem::Haiku
            | OperatingSystem::Ios
            | OperatingSystem::L4re
            | OperatingSystem::Linux
            | OperatingSystem::Nebulet
            | OperatingSystem::Netbsd
            | OperatingSystem::Openbsd
            | OperatingSystem::Redox
            | OperatingSystem::Solaris => CallConv::SystemV,
            OperatingSystem::Windows => CallConv::WindowsFastcall,
            os => panic!("unsupported operating system: {}", os),
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
            CallConv::Baldrdash => "baldrdash",
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
            "baldrdash" => Ok(CallConv::Baldrdash),
            "probestack" => Ok(CallConv::Probestack),
            _ => Err(()),
        }
    }
}
