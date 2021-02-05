// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!
//
// Also at this time this file is heavily based off the x86_64 file, so you'll
// probably want to read that one as well.

#include "header.h"

// fn(top_of_stack(%x0): *mut u8)
HIDDEN(wasmtime_fiber_switch)
GLOBL(wasmtime_fiber_switch)
.p2align 2
TYPE(wasmtime_fiber_switch)
FUNCTION(wasmtime_fiber_switch):
    // Save all callee-saved registers on the stack since we're assuming
    // they're clobbered as a result of the stack switch.
    stp lr, fp, [sp, -16]!
    stp x20, x19, [sp, -16]!
    stp x22, x21, [sp, -16]!
    stp x24, x23, [sp, -16]!
    stp x26, x25, [sp, -16]!
    stp x28, x27, [sp, -16]!
    stp d9, d8, [sp, -16]!
    stp d11, d10, [sp, -16]!
    stp d13, d12, [sp, -16]!
    stp d15, d14, [sp, -16]!

    // Load our previously saved stack pointer to resume to, and save off our
    // current stack pointer on where to come back to eventually.
    ldr x8, [x0, -0x10]
    mov x9, sp
    str x9, [x0, -0x10]

    // Switch to the new stack and restore all our callee-saved registers after
    // the switch and return to our new stack.
    mov sp, x8
    ldp d15, d14, [sp], 16
    ldp d13, d12, [sp], 16
    ldp d11, d10, [sp], 16
    ldp d9, d8, [sp], 16
    ldp x28, x27, [sp], 16
    ldp x26, x25, [sp], 16
    ldp x24, x23, [sp], 16
    ldp x22, x21, [sp], 16
    ldp x20, x19, [sp], 16
    ldp lr, fp, [sp], 16
    ret
SIZE(wasmtime_fiber_switch)

// fn(
//    top_of_stack(%x0): *mut u8,
//    entry_point(%x1): extern fn(*mut u8, *mut u8),
//    entry_arg0(%x2): *mut u8,
// )
HIDDEN(wasmtime_fiber_init)
GLOBL(wasmtime_fiber_init)
.p2align 2
TYPE(wasmtime_fiber_init)
FUNCTION(wasmtime_fiber_init):
    adr x8, FUNCTION(wasmtime_fiber_start)
    stp x0, x8, [x0, -0x28] // x0 => x19, x8 => lr
    stp x2, x1, [x0, -0x38] // x1 => x20, x2 => x21

    // `wasmtime_fiber_switch` has an 0xa0 byte stack, and we add 0x10 more for
    // the original reserved 16 bytes.
    add x8, x0, -0xb0
    str x8, [x0, -0x10]
    ret
SIZE(wasmtime_fiber_init)

.p2align 2
TYPE(wasmtime_fiber_start)
FUNCTION(wasmtime_fiber_start):
.cfi_startproc simple

    // See the x86_64 file for more commentary on what these CFI directives are
    // doing. Like over there note that the relative offsets to registers here
    // match the frame layout in `wasmtime_fiber_switch`.
    .cfi_escape 0x0f,    /* DW_CFA_def_cfa_expression */ \
        5,               /* the byte length of this expression */ \
        0x6f,            /* DW_OP_reg31(%sp) */ \
        0x06,            /* DW_OP_deref */ \
        0x23, 0xa0, 0x1  /* DW_OP_plus_uconst 0xa0 */

    .cfi_rel_offset lr, -0x10
    .cfi_rel_offset x19, -0x18
    .cfi_rel_offset x20, -0x20
    .cfi_rel_offset x21, -0x28
    .cfi_rel_offset x22, -0x30
    .cfi_rel_offset x23, -0x38
    .cfi_rel_offset x24, -0x40
    .cfi_rel_offset x25, -0x48
    .cfi_rel_offset x26, -0x50
    .cfi_rel_offset x27, -0x58
    .cfi_rel_offset x29, -0x60

    // Load our two arguments from the stack, where x1 is our start procedure
    // and x0 is its first argument. This also blows away the stack space used
    // by those two arguments.
    mov x0, x21
    mov x1, x19

    // ... and then we call the function! Note that this is a function call so
    // our frame stays on the stack to backtrace through.
    blr x20
    // .. technically we shouldn't get here, and I would like to write in an
    // instruction which just aborts, but I don't know such an instruction in
    // aarch64 land.
    .cfi_endproc
SIZE(wasmtime_fiber_start)

// This omits the `.subsections_via_symbols` directive on macOS which means we
// can't GC specific intrinsics from this file, but it enables usage of the
// `adr` instruction above in lieu of figuring out a slightly more complicated
// way of implementing that.
#ifndef CFG_TARGET_OS_macos
FOOTER
#endif
