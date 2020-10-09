use super::*;
use crate::binemit::CodeOffset;
use crate::isa::unwind::UnwindInfo;
use crate::result::CodegenResult;
use alloc::boxed::Box;

pub struct Arm32UnwindInfo;

impl UnwindInfoGenerator<Inst> for Arm32UnwindInfo {
    fn create_unwind_info(
        _kind: UnwindInfoKind,
        _insts: &[Inst],
        _insts_layout: &[CodeOffset],
        _len: CodeOffset,
        _prologue_epilogue: &(u32, u32, Box<[(u32, u32)]>),
    ) -> CodegenResult<Option<UnwindInfo>> {
        // TODO
        Ok(None)
    }
}
