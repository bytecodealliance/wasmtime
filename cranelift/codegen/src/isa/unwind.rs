//! Represents information relating to function unwinding.
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

pub mod systemv;
pub mod winx64;

/// Represents unwind information for a single function.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum UnwindInfo {
    /// Windows x64 ABI unwind information.
    WindowsX64(winx64::UnwindInfo),
    /// System V ABI unwind information.
    SystemV(systemv::UnwindInfo),
}
