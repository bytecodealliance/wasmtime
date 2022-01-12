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

#include "header.h"

#ifndef CFG_TARGET_OS_macos
// We need to tell whatever loads the following code (e.g. the dynamic linker)
// that it is compatible with BTI, so that the corresponding executable memory
// pages have the necessary attribute set (if supported by the environment). To
// this end, we follow the ELF for the Arm® 64-bit Architecture standard, and
// use a special metadata section. Further details are in section 6.2 of the
// specification:
//
// https://github.com/ARM-software/abi-aa/blob/2022Q1/aaelf64/aaelf64.rst#program-property
//
// We also set the PAuth (PAC) property, even though it is optional, for the
// sake of completeness.
.pushsection .note.gnu.property, "a";
.p2align 3;
.word 4;
.word 16;
.word 5;
.asciz "GNU";
.word 0xc0000000; // GNU_PROPERTY_AARCH64_FEATURE_1_AND
.word 4;
.word 3; // GNU_PROPERTY_AARCH64_FEATURE_1_BTI | GNU_PROPERTY_AARCH64_FEATURE_1_PAC
.word 0;
.popsection
#endif

// fn(top_of_stack(%x0): *mut u8)
HIDDEN(wasmtime_fiber_switch)
GLOBL(wasmtime_fiber_switch)
.p2align 2
TYPE(wasmtime_fiber_switch)
FUNCTION(wasmtime_fiber_switch):
    .cfi_startproc
#ifndef CFG_TARGET_OS_macos
    hint #25 // paciasp
    .cfi_window_save
#endif
    // Save all callee-saved registers on the stack since we're assuming
    // they're clobbered as a result of the stack switch.
    stp x29, x30, [sp, -16]!
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
    ldp x29, x30, [sp], 16
#ifndef CFG_TARGET_OS_macos
    hint #29 // autiasp
    .cfi_window_save
#endif
    ret
    .cfi_endproc
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
    .cfi_startproc
    hint #34 // bti c
    // We set up the newly initialized fiber, so that it resumes execution from
    // wasmtime_fiber_start(). As a result, we need a signed address of this
    // function, so there are 2 requirements:
    // * The fiber stack pointer value that is used by the signing operation
    //   must match the value when the pointer is authenticated inside
    //   wasmtime_fiber_switch(), otherwise the latter would fault
    // * We would like to use an instruction that is executed as a no-op by
    //   processors that do not support PAuth, so that the code is backward-
    //   compatible and there is no duplication; `PACIA1716` is a suitable
    //   one, which has the following operand register conventions:
    //   * X17 contains the pointer value to sign
    //   * X16 contains the modifier value
    //
    // TODO: Use the PACGA instruction to authenticate the saved register state,
    // which avoids creating signed pointers to wasmtime_fiber_start(), and
    // provides wider coverage.
    sub x16, x0, #16
    adr x17, FUNCTION(wasmtime_fiber_start)
#ifndef CFG_TARGET_OS_macos
    hint #8 // pacia1716
#endif
    str x17, [x16, -0x8] // x17 => lr
    str x0, [x16, -0x18] // x0 => x19
    stp x2, x1, [x0, -0x38] // x1 => x20, x2 => x21

    // `wasmtime_fiber_switch` has an 0xa0 byte stack, and we add 0x10 more for
    // the original reserved 16 bytes.
    add x8, x0, -0xb0
    str x8, [x0, -0x10]
    ret
    .cfi_endproc
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

    .cfi_rel_offset x29, -0x10
#ifndef CFG_TARGET_OS_macos
    .cfi_window_save
#endif
    .cfi_rel_offset x30, -0x08
    .cfi_rel_offset x19, -0x18
    .cfi_rel_offset x20, -0x20
    .cfi_rel_offset x21, -0x28
    .cfi_rel_offset x22, -0x30
    .cfi_rel_offset x23, -0x38
    .cfi_rel_offset x24, -0x40
    .cfi_rel_offset x25, -0x48
    .cfi_rel_offset x26, -0x50
    .cfi_rel_offset x27, -0x58

    // Load our two arguments from the stack, where x1 is our start procedure
    // and x0 is its first argument. This also blows away the stack space used
    // by those two arguments.
    mov x0, x21
    mov x1, x19

    // ... and then we call the function! Note that this is a function call so
    // our frame stays on the stack to backtrace through.
    blr x20
    // Unreachable, here for safety. This should help catch unexpected behaviors.
    // Use a noticeable payload so one can grep for it in the codebase.
    brk 0xf1b3
    .cfi_endproc
SIZE(wasmtime_fiber_start)

// This omits the `.subsections_via_symbols` directive on macOS which means we
// can't GC specific intrinsics from this file, but it enables usage of the
// `adr` instruction above in lieu of figuring out a slightly more complicated
// way of implementing that.
#ifndef CFG_TARGET_OS_macos
FOOTER
#endif
