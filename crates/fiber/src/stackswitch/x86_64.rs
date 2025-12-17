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
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* rdi */) {
    naked_asm!(
        "
        // We're switching to arbitrary code somewhere else, so pessimistically
        // assume that all callee-save register are clobbered. This means we need
        // to save/restore all of them.
        //
        // Note that this order for saving is important since we use CFI directives
        // below to point to where all the saved registers are.
        push rbp
        push rbx
        push r12
        push r13
        push r14
        push r15

        // Load pointer that we're going to resume at and store where we're going
        // to get resumed from. This is in accordance with the diagram at the top
        // of unix.rs.
        mov rax, -0x10[rdi]
        mov -0x10[rdi], rsp

        // Swap stacks and restore all our callee-saved registers
        mov rsp, rax
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp
        ret
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
        r15: *mut u8,
        r14: *mut u8,
        r13: *mut u8,
        r12: *mut u8,
        rbx: *mut u8,
        rbp: *mut u8,
        return_address: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            r12: entry_arg0,
            rbx: entry_point as *mut u8,
            rbp: top_of_stack,
            return_address: wasmtime_fiber_start as *mut u8,
            last_sp: initial_stack.cast(),
            ..InitialStack::default()
        });
    }
}

// This is a pretty special function that has no real signature. Its use is to
// be the "base" function of all fibers. This entrypoint is used in
// `wasmtime_fiber_init` to bootstrap the execution of a new fiber.
//
// We also use this function as a persistent frame on the stack to emit dwarf
// information to unwind into the caller. This allows us to unwind from the
// fiber's stack back to the main stack that the fiber was called from. We use
// special dwarf directives here to do so since this is a pretty nonstandard
// function.
//
// If you're curious a decent introduction to CFI things and unwinding is at
// https://www.imperialviolet.org/2017/01/18/cfi.html
#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_start() -> ! {
    naked_asm!(
        "
        // Use the `simple` directive on the startproc here which indicates that
        // some default settings for the platform are omitted, since this
        // function is so nonstandard
        .cfi_startproc simple
        .cfi_def_cfa_offset 0

        // This is where things get special, we're specifying a custom dwarf
        // expression for how to calculate the CFA. The goal here is that we
        // need to load the parent's stack pointer just before the call it made
        // into `wasmtime_fiber_switch`. Note that the CFA value changes over
        // time as well because a fiber may be resumed multiple times from
        // different points on the original stack. This means that our custom
        // CFA directive involves `DW_OP_deref`, which loads data from memory.
        //
        // The expression we're encoding here is that the CFA, the stack pointer
        // of whatever called into `wasmtime_fiber_start`, is:
        //
        //        *$rsp + 0x38
        //
        // $rsp is the stack pointer of `wasmtime_fiber_start` at the time the
        // next instruction after the `.cfi_escape` is executed. Our $rsp at the
        // start of this function is 16 bytes below the top of the stack (0xAff0
        // in the diagram in unix.rs). The $rsp to resume at is stored at that
        // location, so we dereference the stack pointer to load it.
        //
        // After dereferencing, though, we have the $rsp value for
        // `wasmtime_fiber_switch` itself. That's a weird function which sort of
        // and sort of doesn't exist on the stack.  We want to point to the
        // caller of `wasmtime_fiber_switch`, so to do that we need to skip the
        // stack space reserved by `wasmtime_fiber_switch`, which is the 6 saved
        // registers plus the return address of the caller's `call` instruction.
        // Hence we offset another 0x38 bytes.
        .cfi_escape 0x0f, /* DW_CFA_def_cfa_expression */ \
            4,            /* the byte length of this expression */ \
            0x57,         /* DW_OP_reg7 (rsp) */ \
            0x06,         /* DW_OP_deref */ \
            0x23, 0x38    /* DW_OP_plus_uconst 0x38 */

        // And now after we've indicated where our CFA is for our parent
        // function, we can define that where all of the saved registers are
        // located. This uses standard `.cfi` directives which indicate that
        // these registers are all stored relative to the CFA. Note that this
        // order is kept in sync with the above register spills in
        // `wasmtime_fiber_switch`.
        .cfi_rel_offset rip, -8
        .cfi_rel_offset rbp, -16
        .cfi_rel_offset rbx, -24
        .cfi_rel_offset r12, -32
        .cfi_rel_offset r13, -40
        .cfi_rel_offset r14, -48
        .cfi_rel_offset r15, -56

        // The body of this function is pretty similar. All our parameters are
        // already loaded into registers by the switch function. The
        // `wasmtime_fiber_init` routine arranged the various values to be
        // materialized into the registers used here. Our job is to then move
        // the values into the ABI-defined registers and call the entry-point.
        // Note that `call` is used here to leave this frame on the stack so we
        // can use the dwarf info here for unwinding. The trailing `ud2` is just
        // for safety.
        mov rdi, r12
        mov rsi, rbp
        call rbx
        ud2
        .cfi_endproc
        ",
    );
}
