//! s390x-specific definitions of architecture-specific functions in Wasmtime.

#[inline]
#[allow(missing_docs)]
pub fn get_stack_pointer() -> usize {
    psm::stack_pointer() as usize
}

pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    // The next older PC can be found in register %r14 at function entry, which
    // was saved into slot 14 of the register save area pointed to by "FP" (the
    // backchain pointer).
    *(fp as *mut usize).offset(14)
}

// The next older "FP" (backchain pointer) was saved in the slot pointed to
// by the current "FP".
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

pub fn assert_fp_is_aligned(fp: usize) {
    assert_eq!(fp % 8, 0, "stack should always be aligned to 8");
}
