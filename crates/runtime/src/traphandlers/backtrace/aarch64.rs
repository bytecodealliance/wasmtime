// The aarch64 calling conventions save the return PC one word above the FP and
// the previous FP is pointed to by the current FP:
//
// > Each frame shall link to the frame of its caller by means of a frame record
// > of two 64-bit values on the stack [...] The frame record for the innermost
// > frame [...] shall be pointed to by the frame pointer register (FP). The
// > lowest addressed double-word shall point to the previous frame record and the
// > highest addressed double-word shall contain the value passed in LR on entry
// > to the current function.
//
// - AAPCS64 section 6.2.3 The Frame Pointer[0]
pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    *(fp as *mut usize).offset(1)
}
pub unsafe fn get_next_older_fp_from_fp(fp: usize) -> usize {
    *(fp as *mut usize)
}

pub fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool {
    // Calls in aarch64 push two words (old FP and return PC) so our entry SP is
    // two words above the first Wasm FP.
    fp == first_wasm_sp - 16
}

pub fn assert_entry_sp_is_aligned(sp: usize) {
    assert_eq!(sp % 16, 0, "stack should always be aligned to 16");
}

pub fn assert_fp_is_aligned(_fp: usize) {
    // From AAPCS64, section 6.2.3 The Frame Pointer[0]:
    //
    // > The location of the frame record within a stack frame is not specified.
    //
    // So this presumably means that the FP can have any alignment, as its
    // location is not specified and nothing further is said about constraining
    // alignment.
    //
    // [0]: https://github.com/ARM-software/abi-aa/blob/2022Q1/aapcs64/aapcs64.rst#the-frame-pointer
}
