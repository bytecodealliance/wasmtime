use super::*;
use crate::binemit::CodeOffset;
use crate::isa::unwind::UnwindInfo;
use crate::result::CodegenResult;
use alloc::boxed::Box;
use core::ops::Range;

pub struct AArch64UnwindInfo;

impl UnwindInfoGenerator<Inst> for AArch64UnwindInfo {
    fn create_unwind_info(
        _kind: UnwindInfoKind,
        _insts: &[Inst],
        _insts_layout: &[CodeOffset],
        _len: CodeOffset,
        _prologue_epilogue: &(Range<u32>, Box<[Range<u32>]>),
    ) -> CodegenResult<Option<UnwindInfo>> {
        // TODO
        Ok(None)
    }
}
