use super::*;
use crate::isa::unwind::UnwindInfo;
use crate::result::CodegenResult;

pub struct AArch64UnwindInfo;

impl UnwindInfoGenerator<Inst> for AArch64UnwindInfo {
    fn create_unwind_info(
        _context: UnwindInfoContext<Inst>,
        _kind: UnwindInfoKind,
    ) -> CodegenResult<Option<UnwindInfo>> {
        // TODO
        Ok(None)
    }
}
