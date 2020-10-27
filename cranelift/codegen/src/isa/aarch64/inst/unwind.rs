use super::*;
use crate::isa::unwind::input::UnwindInfo;
use crate::result::CodegenResult;

pub struct AArch64UnwindInfo;

impl UnwindInfoGenerator<Inst> for AArch64UnwindInfo {
    fn create_unwind_info(
        _context: UnwindInfoContext<Inst>,
    ) -> CodegenResult<Option<UnwindInfo<Reg>>> {
        // TODO
        Ok(None)
    }
}
