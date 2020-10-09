use super::{regs, Inst};
use crate::binemit::CodeOffset;
use crate::isa::unwind::UnwindInfo;
use crate::machinst::{UnwindInfoGenerator, UnwindInfoKind};
use crate::result::CodegenResult;
use alloc::boxed::Box;

pub use self::systemv::create_cie;

mod systemv;

pub struct X64UnwindInfo;

impl UnwindInfoGenerator<Inst> for X64UnwindInfo {
    fn create_unwind_info(
        kind: UnwindInfoKind,
        insts: &[Inst],
        insts_layout: &[CodeOffset],
        len: CodeOffset,
        prologue_epilogue: &(u32, u32, Box<[(u32, u32)]>),
    ) -> CodegenResult<Option<UnwindInfo>> {
        // Assumption: RBP is being used as the frame pointer for both calling conventions
        // In the future, we should be omitting frame pointer as an optimization, so this will change
        Ok(match kind {
            UnwindInfoKind::SystemV => systemv::create_unwind_info(
                insts,
                insts_layout,
                len,
                prologue_epilogue,
                Some(regs::rbp()),
            )?
            .map(UnwindInfo::SystemV),
            UnwindInfoKind::Windows => {
                //TODO super::unwind::winx64::create_unwind_info(func, isa)?.map(|u| UnwindInfo::WindowsX64(u))
                panic!();
            }
            _ => None,
        })
    }
}
