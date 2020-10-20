use super::*;
use crate::isa::unwind::UnwindInfo;
use crate::result::CodegenResult;

pub struct Arm32UnwindInfo;

impl UnwindInfoGenerator<Inst> for Arm32UnwindInfo {
    fn create_unwind_info(
        _context: UnwindInfoContext<Inst>,
        _kind: UnwindInfoKind,
    ) -> CodegenResult<Option<UnwindInfo>> {
        // TODO
        Ok(None)
    }
}
