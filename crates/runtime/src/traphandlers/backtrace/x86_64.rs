pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    // The calling convention always pushes the return pointer (aka the PC of
    // the next older frame) just before this frame.
    *(fp as *mut usize).offset(1)
}

// And the current frame pointer points to the next older frame pointer.
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

pub fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool {
    // When the FP is just below the SP (because we are in a function prologue
    // where the `call` pushed the return pointer, but the callee hasn't pushed
    // the frame pointer yet) we are done.
    fp == first_wasm_sp - 8
}

pub fn assert_entry_sp_is_aligned(sp: usize) {
    // The stack pointer should always be aligned to 16 bytes *except* inside
    // function prologues where the return PC is pushed to the stack but before
    // the old frame pointer has been saved to the stack via `push rbp`. And
    // this happens to be exactly where we are inside of our host-to-Wasm
    // trampoline that records the value of SP when we first enter
    // Wasm. Therefore, the SP should *always* be 8-byte aligned but *never*
    // 16-byte aligned.
    assert_eq!(sp % 8, 0);
    assert_eq!(sp % 16, 8);
}

pub fn assert_fp_is_aligned(fp: usize) {
    assert_eq!(fp % 16, 0, "stack should always be aligned to 16");
}
