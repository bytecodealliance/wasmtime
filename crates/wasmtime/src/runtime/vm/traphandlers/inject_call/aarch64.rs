//! aarch64 injected-call trampoline.
//!
//! see [`super::x86_64`] for documentation on how the
//! architecture-specific trampolines are used.

use core::arch::naked_asm;

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn injected_call_trampoline(_hostcall: usize, _store: usize) {
    naked_asm!(
        "
        // Fake return address and saved FP.
        stp fp, xzr, [sp, #-16]!
        mov fp, sp

        // Saved GPRs.
        stp  x0,  x1, [sp, #-16]!
        stp  x2,  x3, [sp, #-16]!
        stp  x4,  x5, [sp, #-16]!
        stp  x6,  x7, [sp, #-16]!
        stp  x8,  x9, [sp, #-16]!
        stp x10, x11, [sp, #-16]!
        stp x12, x13, [sp, #-16]!
        stp x14, x15, [sp, #-16]!
        stp x16, x17, [sp, #-16]!
        stp x18, x19, [sp, #-16]!
        stp x20, x21, [sp, #-16]!
        stp x22, x23, [sp, #-16]!
        stp x24, x25, [sp, #-16]!
        stp x26, x27, [sp, #-16]!
        stp x28, xzr, [sp, #-16]!

        // Saved float/vector registers.
        stp  q0,  q1, [sp, #-32]!
        stp  q2,  q3, [sp, #-32]!
        stp  q4,  q5, [sp, #-32]!
        stp  q6,  q7, [sp, #-32]!
        stp  q8,  q9, [sp, #-32]!
        stp q10, q11, [sp, #-32]!
        stp q12, q13, [sp, #-32]!
        stp q14, q15, [sp, #-32]!
        stp q16, q17, [sp, #-32]!
        stp q18, q19, [sp, #-32]!
        stp q20, q21, [sp, #-32]!
        stp q22, q23, [sp, #-32]!
        stp q24, q25, [sp, #-32]!
        stp q26, q27, [sp, #-32]!
        stp q28, q29, [sp, #-32]!
        stp q30, q31, [sp, #-32]!

        mov x4, x0
        mov x0, x1
        add x1, sp, #(16 * 32 + 15 * 16 + 8) // saved LR.
        add x2, sp, #(16 * 32 + 14 * 16 + 0) // saved X0.
        add x3, sp, #(16 * 32 + 14 * 16 + 8) // saved X1.
        blr x4

        ldp q30, q31, [sp], #32
        ldp q28, q29, [sp], #32
        ldp q26, q27, [sp], #32
        ldp q24, q25, [sp], #32
        ldp q22, q23, [sp], #32
        ldp q20, q21, [sp], #32
        ldp q18, q19, [sp], #32
        ldp q16, q17, [sp], #32
        ldp q14, q15, [sp], #32
        ldp q12, q13, [sp], #32
        ldp q10, q11, [sp], #32
        ldp  q8,  q9, [sp], #32
        ldp  q6,  q7, [sp], #32
        ldp  q4,  q5, [sp], #32
        ldp  q2,  q3, [sp], #32
        ldp  q0,  q1, [sp], #32

        ldp x28, x29, [sp], #16
        ldp x26, x27, [sp], #16
        ldp x24, x25, [sp], #16
        ldp x22, x23, [sp], #16
        ldp x20, x21, [sp], #16
        ldp x18, x19, [sp], #16
        ldp x16, x17, [sp], #16
        ldp x14, x15, [sp], #16
        ldp x12, x13, [sp], #16
        ldp x10, x11, [sp], #16
        ldp  x8,  x9, [sp], #16
        ldp  x6,  x7, [sp], #16
        ldp  x4,  x5, [sp], #16
        ldp  x2,  x3, [sp], #16
        ldp  x0,  x1, [sp], #16
        ldp  fp,  lr, [sp], #16

        // N.B.: this leaves LR clobbered; but Cranelift
        // code interrupted by a signal will already have
        // unconditionally saved LR in its frame.
        ret
    ",
    );
}
