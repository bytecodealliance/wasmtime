//! x86-specific (also x86-64) definitions of architecture-specific functions in
//! Wasmtime.

#[inline]
#[allow(missing_docs)]
pub fn get_stack_pointer() -> usize {
    let stack_pointer: usize;
    unsafe {
        #[cfg(target_pointer_width = "64")]
        core::arch::asm!(
            "mov {}, rsp",
            out(reg) stack_pointer,
            options(nostack,nomem),
        );
        #[cfg(target_pointer_width = "32")]
        core::arch::asm!(
            "mov {}, esp",
            out(reg) stack_pointer,
            options(nostack,nomem),
        );
    }
    stack_pointer
}

pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    // The calling convention always pushes the return pointer (aka the PC of
    // the next older frame) just before this frame.
    *(fp as *mut usize).offset(1)
}

// And the current frame pointer points to the next older frame pointer.
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

/// Frame pointers are aligned if they're aligned to twice the size of a
/// pointer.
pub fn assert_fp_is_aligned(fp: usize) {
    let align = 2 * size_of::<usize>();
    assert_eq!(fp % align, 0, "stack should always be aligned to {align}");
}
