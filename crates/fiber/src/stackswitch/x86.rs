// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!
//
// This file is modeled after x86_64.rs and comments are not copied over. For
// reference be sure to review the other file. Note that the pointer size is
// different so the reserved space at the top of the stack is 8 bytes, not 16
// bytes. Still two pointers though.

use core::arch::naked_asm;

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8) {
    naked_asm!(
        "
        // Load our stack-to-use
        mov eax, 0x4[esp]
        mov ecx, -0x8[eax]

        // Save callee-saved registers
        push ebp
        push ebx
        push esi
        push edi

        // Save our current stack and jump to the stack-to-use
        mov -0x8[eax], esp
        mov esp, ecx

        // Restore callee-saved registers
        pop edi
        pop esi
        pop ebx
        pop ebp
        ret
        ",
    )
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8),
    entry_arg0: *mut u8,
) {
    // Our stack from top-to-bottom looks like:
    //
    //	  * 8 bytes of reserved space per unix.rs (two-pointers space)
    //	  * 8 bytes of arguments (two arguments wasmtime_fiber_start forwards)
    //	  * 4 bytes of return address
    //	  * 16 bytes of saved registers
    //
    // Note that after the return address the stack is conveniently 16-byte
    // aligned as required, so we just leave the arguments on the stack in
    // `wasmtime_fiber_start` and immediately do the call.
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        // state that will get resumed into from a `wasmtime_fiber_switch`
        // starting up this fiber.
        edi: *mut u8,
        esi: *mut u8,
        ebx: *mut u8,
        ebp: *mut u8,
        return_address: *mut u8,

        // two arguments to `entry_point`
        arg1: *mut u8,
        arg2: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            ebp: entry_point as *mut u8,
            return_address: wasmtime_fiber_start as *mut u8,
            arg1: entry_arg0,
            arg2: top_of_stack,
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
        .cfi_escape 0x0f, /* DW_CFA_def_cfa_expression */ \
            5,            /* the byte length of this expression */ \
            0x74, 0x08,   /* DW_OP_breg4 (%esp) + 8 */ \
            0x06,         /* DW_OP_deref */ \
            0x23, 0x14    /* DW_OP_plus_uconst 0x14 */

        .cfi_rel_offset eip, -4
        .cfi_rel_offset ebp, -8
        .cfi_rel_offset ebx, -12
        .cfi_rel_offset esi, -16
        .cfi_rel_offset edi, -20

        // Our arguments and stack alignment are all prepped by
        // `wasmtime_fiber_init`.
        call ebp
        ud2
        .cfi_endproc
        ",
    );
}
