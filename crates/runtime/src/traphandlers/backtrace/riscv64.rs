//
pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    *(fp as *mut usize).offset(1)
}

// And the current frame pointer points to the next older frame pointer.
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

pub fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool {
    // Calls in riscv64 push two i64s (old FP and return PC) so our entry SP is
    // two i64s above the first Wasm FP.
    fp == first_wasm_sp - 16
}

pub fn assert_entry_sp_is_aligned(sp: usize) {
    assert_eq!(sp % 16, 0, "stack should always be aligned to 16");
}

pub fn assert_fp_is_aligned(fp: usize) {
    assert_eq!(fp % 16, 0, "stack should always be aligned to 16");
}
