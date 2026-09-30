use super::maybe_yield_fiber;
use core::arch::naked_asm;

/// See the comment on the implementation in x86_64.rs.
///
/// x18 is deliberately not saved as it is not allocatable in Cranelift.
#[unsafe(naked)]
pub(super) unsafe extern "C" fn task_switch_trampoline(_vmctx: usize) {
    naked_asm!(
        "
        // Establish an ordinary AAPCS64 frame record so stack walks can see
        // through, and set x29 as our frame pointer so it can be handed to
        // `maybe_yield_fiber` as the trampoline FP.
        stp x29, x30, [sp, #-16]!
        mov x29, sp

        // Save the registers that an AAPCS64 call may clobber. x19-x28 and the
        // low halves of v8-v15 are callee-saved, but we save the vector
        // registers wholesale anyway since their upper halves are not.
        sub sp, sp, #656
        stp x0, x1, [sp, #0]
        stp x2, x3, [sp, #16]
        stp x4, x5, [sp, #32]
        stp x6, x7, [sp, #48]
        stp x8, x9, [sp, #64]
        // We don't save x10 because `dead_load_with_context` defs it; we fill
        // it below with the next interrupt page pointer. We leave an 8-byte
        // hole here because the stp instructions below that store the q
        // registers need their offsets to be 16-byte aligned, and this is as
        // good a place as any for the padding.
        str x11, [sp, #88]
        stp x12, x13, [sp, #96]
        stp x14, x15, [sp, #112]
        stp x16, x17, [sp, #128]

        stp q0, q1, [sp, #144]
        stp q2, q3, [sp, #176]
        stp q4, q5, [sp, #208]
        stp q6, q7, [sp, #240]
        stp q8, q9, [sp, #272]
        stp q10, q11, [sp, #304]
        stp q12, q13, [sp, #336]
        stp q14, q15, [sp, #368]
        stp q16, q17, [sp, #400]
        stp q18, q19, [sp, #432]
        stp q20, q21, [sp, #464]
        stp q22, q23, [sp, #496]
        stp q24, q25, [sp, #528]
        stp q26, q27, [sp, #560]
        stp q28, q29, [sp, #592]
        stp q30, q31, [sp, #624]

        // vmctx is already in x0, care of the signal handler.
        //
        // The following instructions prepare:
        // `x1`: the value of `x9`, which is the scratch register with the return
        // address
        // `x2`: the value of `x29`, which is the frame pointer
        // `x3`: `load_ptr`, pinned by `dead_load_with_context` to x10
        mov x1, x9
        mov x2, x29
        mov x3, x10
        // Call maybe_yield_fiber() to do the task switch.
        bl {}
        // Move return value (the new MMU interrupt page ptr) to x10 to be
        // returned by the dead_load_with_context instruction, which we're in
        // the middle of.
        mov x10, x0

        // Restore registers.
        ldp q0, q1, [sp, #144]
        ldp q2, q3, [sp, #176]
        ldp q4, q5, [sp, #208]
        ldp q6, q7, [sp, #240]
        ldp q8, q9, [sp, #272]
        ldp q10, q11, [sp, #304]
        ldp q12, q13, [sp, #336]
        ldp q14, q15, [sp, #368]
        ldp q16, q17, [sp, #400]
        ldp q18, q19, [sp, #432]
        ldp q20, q21, [sp, #464]
        ldp q22, q23, [sp, #496]
        ldp q24, q25, [sp, #528]
        ldp q26, q27, [sp, #560]
        ldp q28, q29, [sp, #592]
        ldp q30, q31, [sp, #624]

        ldp x0, x1, [sp, #0]
        ldp x2, x3, [sp, #16]
        ldp x4, x5, [sp, #32]
        ldp x6, x7, [sp, #48]
        ldp x8, x9, [sp, #64]
        ldr x11, [sp, #88]
        ldp x12, x13, [sp, #96]
        ldp x14, x15, [sp, #112]
        ldp x16, x17, [sp, #128]
        add sp, sp, #656

        ldp x29, x30, [sp], #16

        // Resume at the load instruction that triggered the signal handler,
        // re-executing it. x9 was restored above, so it again holds the
        // return address.
        br x9
        ",
        sym maybe_yield_fiber
    );
}

pub(crate) fn resume_into_task_switch_trampoline(
    ucontext: &mut libc::ucontext_t,
    return_address: *const (),
) {
    ucontext.uc_mcontext.pc = task_switch_trampoline as *const () as u64;
    ucontext.uc_mcontext.regs[9] = return_address as u64;
}
