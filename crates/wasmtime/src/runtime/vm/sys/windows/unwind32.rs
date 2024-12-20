//! Module for Windows x86 ABI unwind registry.
//!
//! Note that this is not implemented at this time because there is no Cranelift
//! backend foR windows.

use crate::prelude::*;
use std::mem;

pub enum UnwindRegistration {}

impl UnwindRegistration {
    #[allow(missing_docs)]
    pub const SECTION_NAME: &'static str = ".pdata";

    #[allow(missing_docs)]
    pub unsafe fn new(
        _base_address: *const u8,
        _unwind_info: *const u8,
        _unwind_len: usize,
    ) -> Result<UnwindRegistration> {
        bail!("unwind registration unimplemented on i686 windows")
    }
}
