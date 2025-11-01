// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!

use core::arch::naked_asm;

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* x0 */) {
    naked_asm!(
        "
        // Save all callee-saved registers on the stack since we're assuming
        // they're clobbered as a result of the stack switch.
        stmg %r6, %r15, 48(%r15)
        aghi %r15, -64
        std %f8, 0(%r15)
        std %f9, 8(%r15)
        std %f10, 16(%r15)
        std %f11, 24(%r15)
        std %f12, 32(%r15)
        std %f13, 40(%r15)
        std %f14, 48(%r15)
        std %f15, 56(%r15)

        // Load our previously saved stack pointer to resume to, and save off our
        // current stack pointer on where to come back to eventually.
        lg %r1, -16(%r2)
        stg %r15, -16(%r2)

        // Switch to the new stack and restore all our callee-saved registers after
        // the switch and return to our new stack.
        ld %f8, 0(%r1)
        ld %f9, 8(%r1)
        ld %f10, 16(%r1)
        ld %f11, 24(%r1)
        ld %f12, 32(%r1)
        ld %f13, 40(%r1)
        ld %f14, 48(%r1)
        ld %f15, 56(%r1)
        lmg %r6, %r15, 112(%r1)
        br %r14
        ",
    );
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8),
    entry_arg0: *mut u8,
) {
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        f8: *mut u8,
        f9: *mut u8,
        f10: *mut u8,
        f11: *mut u8,
        f12: *mut u8,
        f13: *mut u8,
        f14: *mut u8,
        f15: *mut u8,

        back_chain: *mut u8,
        compiler_reserved: *mut u8,

        r2: *mut u8,
        r3: *mut u8,
        r4: *mut u8,
        r5: *mut u8,

        r6: *mut u8,
        r7: *mut u8,
        r8: *mut u8,
        r9: *mut u8,
        r10: *mut u8,
        r11: *mut u8,
        r12: *mut u8,
        r13: *mut u8,
        r14: *mut u8,
        r15: *mut u8,

        f0: *mut u8,
        f2: *mut u8,
        f4: *mut u8,
        f6: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            r15: (&raw mut (*initial_stack).back_chain).cast(),
            r14: wasmtime_fiber_start as *mut u8,
            r6: top_of_stack,
            r7: entry_point as *mut u8,
            r8: entry_arg0,

            last_sp: initial_stack.cast(),
            ..InitialStack::default()
        });
    }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_start() -> ! {
    naked_asm!(
        "
        .cfi_startproc simple
        .cfi_def_cfa_offset 0

        // See the x86_64 file for more commentary on what these CFI directives are
        // doing. Like over there note that the relative offsets to registers here
        // match the frame layout in `wasmtime_fiber_switch`.
        .cfi_escape 0x0f,    /* DW_CFA_def_cfa_expression */ \
            7,               /* the byte length of this expression */ \
            0x7f, 0xa0, 0x1, /* DW_OP_breg15 0x90 */ \
            0x06,            /* DW_OP_deref */ \
            0x23, 0xe0, 0x1  /* DW_OP_plus_uconst 0xe0 */

        .cfi_rel_offset 6, -112
        .cfi_rel_offset 7, -104
        .cfi_rel_offset 8, -96
        .cfi_rel_offset 9, -88
        .cfi_rel_offset 10, -80
        .cfi_rel_offset 11, -72
        .cfi_rel_offset 12, -64
        .cfi_rel_offset 13, -56
        .cfi_rel_offset 14, -48
        .cfi_rel_offset 15, -40

        // Load our two arguments prepared by `wasmtime_fiber_init`.
        lgr %r2, %r8  // entry_arg0
        lgr %r3, %r6  // top_of_stack

        // ... and then we call the function! Note that this is a function call so
        // our frame stays on the stack to backtrace through.
        basr %r14, %r7  // entry_point
        // .. technically we shouldn't get here, so just trap.
        .word 0x0000
        .cfi_endproc
        ",
    );
}
