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

pub(crate) mod input {
    use crate::binemit::CodeOffset;
    use alloc::vec::Vec;
    #[cfg(feature = "enable-serde")]
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
    pub(crate) enum UnwindCode<Reg> {
        SaveRegister {
            offset: CodeOffset,
            reg: Reg,
        },
        RestoreRegister {
            offset: CodeOffset,
            reg: Reg,
        },
        SaveXmmRegister {
            offset: CodeOffset,
            reg: Reg,
            stack_offset: u32,
        },
        StackAlloc {
            offset: CodeOffset,
            size: u32,
        },
        StackDealloc {
            offset: CodeOffset,
            size: u32,
        },
        SetFramePointer {
            offset: CodeOffset,
            reg: Reg,
        },
        RememberState {
            offset: CodeOffset,
        },
        RestoreState {
            offset: CodeOffset,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
    pub struct UnwindInfo<Reg> {
        pub(crate) prologue_size: CodeOffset,
        pub(crate) prologue_unwind_codes: Vec<UnwindCode<Reg>>,
        pub(crate) epilogues_unwind_codes: Vec<Vec<UnwindCode<Reg>>>,
        pub(crate) function_size: CodeOffset,
    }
}
