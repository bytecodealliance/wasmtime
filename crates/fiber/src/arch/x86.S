// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!
//
// This file is modeled after x86_64.S and comments are not copied over. For
// reference be sure to review the other file. Note that the pointer size is
// different so the reserved space at the top of the stack is 8 bytes, not 16
// bytes. Still two pointers though.

#include "header.h"

// fn(top_of_stack: *mut u8)
HIDDEN(wasmtime_fiber_switch)
GLOBL(wasmtime_fiber_switch)
TYPE(wasmtime_fiber_switch)
FUNCTION(wasmtime_fiber_switch):
    // Load our stack-to-use
    mov 0x4(%esp), %eax
    mov -0x8(%eax), %ecx

    // Save callee-saved registers
    push %ebp
    push %ebx
    push %esi
    push %edi

    // Save our current stack and jump to the stack-to-use
    mov %esp, -0x8(%eax)
    mov %ecx, %esp

    // Restore callee-saved registers
    pop %edi
    pop %esi
    pop %ebx
    pop %ebp
    ret
SIZE(wasmtime_fiber_switch)

// fn(
//    top_of_stack: *mut u8,
//    entry_point: extern fn(*mut u8, *mut u8),
//    entry_arg0: *mut u8,
// )
HIDDEN(wasmtime_fiber_init)
GLOBL(wasmtime_fiber_init)
TYPE(wasmtime_fiber_init)
FUNCTION(wasmtime_fiber_init):
    mov 4(%esp), %eax

    // move top_of_stack to the 2nd argument
    mov %eax, -0x0c(%eax)

    // move entry_arg0 to the 1st argument
    mov 12(%esp), %ecx
    mov %ecx, -0x10(%eax)

    // Move our start function to the return address which the `ret` in
    // `wasmtime_fiber_start` will return to.
    lea FUNCTION(wasmtime_fiber_start), %ecx
    mov %ecx, -0x14(%eax)

    // And move `entry_point` to get loaded into `%ebp` through the context
    // switch. This'll get jumped to in `wasmtime_fiber_start`.
    mov 8(%esp), %ecx
    mov %ecx, -0x18(%eax)

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
    lea -0x24(%eax), %ecx
    mov %ecx, -0x08(%eax)
    ret
SIZE(wasmtime_fiber_init)

TYPE(wasmtime_fiber_start)
FUNCTION(wasmtime_fiber_start):
.cfi_startproc simple
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
    call *%ebp
    ud2
    .cfi_endproc
SIZE(wasmtime_fiber_start)

FOOTER
