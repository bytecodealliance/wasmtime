//! Represents information relating to function unwinding.
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

pub mod systemv;

/// Represents unwind information for a single function.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum UnwindInfo {
    /// Windows x64 ABI unwind information.
    #[cfg(feature = "x86")]
    WindowsX64(super::x86::unwind::windows::UnwindInfo),
    /// System V ABI unwind information.
    SystemV(systemv::UnwindInfo),
}
