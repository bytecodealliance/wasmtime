// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!
//
// Also at this time this file is heavily based off the x86_64 file, so you'll
// probably want to read that one as well.
//
// Finally, control flow integrity hardening has been applied to the code using
// the Pointer Authentication (PAuth) and Branch Target Identification (BTI)
// technologies from the Arm instruction set architecture:
// * All callable functions start with either the `BTI c` or `PACIASP`/`PACIBSP`
//   instructions
// * Return addresses are signed and authenticated using the stack pointer
//   value as a modifier (similarly to the salt in a HMAC operation); the
//   `DW_CFA_AARCH64_negate_ra_state` DWARF operation (aliased with the
//   `.cfi_window_save` assembler directive) informs an unwinder about this

use core::arch::naked_asm;

cfg_if::cfg_if! {
    if #[cfg(target_vendor = "apple")] {
        macro_rules! paci1716 { () => ("pacib1716\n"); }
        macro_rules! pacisp { () => ("pacibsp\n"); }
        macro_rules! autisp { () => ("autibsp\n"); }
    } else {
        macro_rules! paci1716 { () => ("pacia1716\n"); }
        macro_rules! pacisp { () => ("paciasp\n"); }
        macro_rules! autisp { () => ("autiasp\n"); }
    }
}

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* x0 */) {
    naked_asm!(concat!(
        "
            .cfi_startproc
        ",
        pacisp!(),
        "
            .cfi_window_save
            // Save all callee-saved registers on the stack since we're
            // assuming they're clobbered as a result of the stack switch.
            stp x29, x30, [sp, -16]!
            stp x27, x28, [sp, -16]!
            stp x25, x26, [sp, -16]!
            stp x23, x24, [sp, -16]!
            stp x21, x22, [sp, -16]!
            stp x19, x20, [sp, -16]!
            stp d14, d15, [sp, -16]!
            stp d12, d13, [sp, -16]!
            stp d10, d11, [sp, -16]!
            stp d8, d9, [sp, -16]!

            // Load our previously saved stack pointer to resume to, and save
            // off our current stack pointer on where to come back to
            // eventually.
            ldr x8, [x0, -0x10]
            mov x9, sp
            str x9, [x0, -0x10]

            // Switch to the new stack and restore all our callee-saved
            // registers after the switch and return to our new stack.
            mov sp, x8
            ldp d8, d9, [sp], 16
            ldp d10, d11, [sp], 16
            ldp d12, d13, [sp], 16
            ldp d14, d15, [sp], 16

            ldp x19, x20, [sp], 16
            ldp x21, x22, [sp], 16
            ldp x23, x24, [sp], 16
            ldp x25, x26, [sp], 16
            ldp x27, x28, [sp], 16
            ldp x29, x30, [sp], 16
        ",
        autisp!(),
        "
            .cfi_window_save
            ret
            .cfi_endproc
        ",
    ));
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8),
    entry_arg0: *mut u8, // x2
) {
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        d8: u64,
        d9: u64,
        d10: u64,
        d11: u64,
        d12: u64,
        d13: u64,
        d14: u64,
        d15: u64,

        x19: *mut u8,
        x20: *mut u8,
        x21: *mut u8,
        x22: *mut u8,
        x23: *mut u8,
        x24: *mut u8,
        x25: *mut u8,
        x26: *mut u8,
        x27: *mut u8,
        x28: *mut u8,

        fp: *mut u8,
        lr: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            x19: top_of_stack,
            x20: entry_point as *mut u8,
            x21: entry_arg0,

            // We set up the newly initialized fiber, so that it resumes
            // execution from wasmtime_fiber_start(). As a result, we need a
            // signed address of this function because `wasmtime_fiber_switch`
            // ends with a `auti{a,b}sp` instruction. There are 2 requirements:
            // * We would like to use an instruction that is executed as a no-op
            //   by processors that do not support PAuth, so that the code is
            //   backward-compatible and there is no duplication; `PACIA1716` is
            //   a suitable one.
            // * The fiber stack pointer value that is used by the signing
            //   operation must match the value when the pointer is
            //   authenticated inside wasmtime_fiber_switch(), which is 16 bytes
            //   below the `top_of_stack` which will be `sp` at the time of the
            //   `auti{a,b}sp`.
            //
            // TODO: Use the PACGA instruction to authenticate the saved register
            // state, which avoids creating signed pointers to
            // wasmtime_fiber_start(), and provides wider coverage.
            lr: paci1716(wasmtime_fiber_start as *mut u8, top_of_stack.sub(16)),

            last_sp: initial_stack.cast(),
            ..InitialStack::default()
        });
    }
}

/// Signs `r17` with the value in `r16` using either `paci{a,b}1716` depending
/// on the platform.
fn paci1716(mut r17: *mut u8, r16: *mut u8) -> *mut u8 {
    unsafe {
        core::arch::asm!(
            paci1716!(),
            inout("x17") r17,
            in("x16") r16,
        );
        r17
    }
}

// See the x86_64 file for more commentary on what these CFI directives are
// doing. Like over there note that the relative offsets to registers here
// match the frame layout in `wasmtime_fiber_switch`.
#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_start() -> ! {
    naked_asm!(
        "
        .cfi_startproc simple
        .cfi_def_cfa_offset 0
        .cfi_escape 0x0f,    /* DW_CFA_def_cfa_expression */ \
            5,               /* the byte length of this expression */ \
            0x6f,            /* DW_OP_reg31(%sp) */ \
            0x06,            /* DW_OP_deref */ \
            0x23, 0xa0, 0x1  /* DW_OP_plus_uconst 0xa0 */
        .cfi_rel_offset x30, -0x08
        .cfi_rel_offset x29, -0x10
        .cfi_window_save
        .cfi_rel_offset x28, -0x18
        .cfi_rel_offset x27, -0x20
        .cfi_rel_offset x26, -0x28
        .cfi_rel_offset x25, -0x30
        .cfi_rel_offset x24, -0x38
        .cfi_rel_offset x23, -0x40
        .cfi_rel_offset x22, -0x48
        .cfi_rel_offset x21, -0x50
        .cfi_rel_offset x20, -0x58
        .cfi_rel_offset x19, -0x60

        // Load our two arguments from the stack, where x1 is our start
        // procedure and x0 is its first argument. This also blows away the
        // stack space used by those two arguments.
        mov x0, x21
        mov x1, x19

        // ... and then we call the function! Note that this is a function call
        // so our frame stays on the stack to backtrace through.
        blr x20
        // Unreachable, here for safety. This should help catch unexpected
        // behaviors.  Use a noticeable payload so one can grep for it in the
        // codebase.
        brk 0xf1b3
        .cfi_endproc
        ",
    );
}
