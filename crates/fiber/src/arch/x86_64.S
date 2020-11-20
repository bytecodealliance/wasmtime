// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!

.text

#if CFG_TARGET_OS_macos

#define GLOBL(fnname) .globl _##fnname
#define TYPE(fnname)
#define FUNCTION(fnname) _##fnname
#define SIZE(fnname)

#else

#define GLOBL(fnname) .globl fnname
#define TYPE(fnname) .type fnname,@function
#define FUNCTION(fnname) fnname
#define SIZE(fnname) .size fnname,.-fnname

#endif

// fn(top_of_stack(%rdi): *mut u8)
GLOBL(wasmtime_fiber_switch)
.align 16
TYPE(wasmtime_fiber_switch)
FUNCTION(wasmtime_fiber_switch):
    // We're switching to arbitrary code somewhere else, so pessimistically
    // assume that all callee-save register are clobbered. This means we need
    // to save/restore all of them.
    //
    // Note that this order for saving is important since we use CFI directives
    // below to point to where all the saved registers are.
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15

    // Load pointer that we're going to resume at and store where we're going
    // to get resumed from. This is in accordance with the diagram at the top
    // of unix.rs.
    movq -0x10(%rdi), %rax
    mov %rsp, -0x10(%rdi)

    // Swap stacks and restore all our callee-saved registers
    mov %rax, %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    ret
SIZE(wasmtime_fiber_switch)

// fn(
//    top_of_stack(%rdi): *mut u8,
//    entry_point(%rsi): extern fn(*mut u8, *mut u8),
//    entry_arg0(%rdx): *mut u8,
// )
GLOBL(wasmtime_fiber_init)
.align 16
TYPE(wasmtime_fiber_init)
FUNCTION(wasmtime_fiber_init):
    // The first 16 bytes of the stack are reserved (see unix.rs) so we store
    // the initial data used in `wasmtime_fiber_start` just below
    // that.
    movq %rdi, -0x18(%rdi)
    movq %rsi, -0x20(%rdi)
    movq %rdx, -0x28(%rdi)

    // After these arguments is the return address of where to switch to,
    // which for the first run is `wasmtime_fiber_start`.
    lea FUNCTION(wasmtime_fiber_start)(%rip), %rax
    movq %rax, -0x30(%rdi)

    // And then we specify the stack pointer resumption should begin at. Our
    // `wasmtime_fiber_switch` function saves 6 registers so we need to ensure
    // that there's space for that as well. 0x30 + 6 * 8 == 0x60 here.
    lea -0x60(%rdi), %rax
    movq %rax, -0x10(%rdi)
    ret
SIZE(wasmtime_fiber_init)

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
.align 16
TYPE(wasmtime_fiber_start)
FUNCTION(wasmtime_fiber_start):
// Use the `simple` directive on the startproc here which indicates that some
// default settings for the platform are omitted, since this function is so
// nonstandard
.cfi_startproc simple
    // This is where things get special, we're specifying a custom dwarf
    // expression for how to calculate the CFA. The goal here is that we need
    // to load the parent's stack pointer just before the call it made into
    // `wasmtime_fiber_switch`. Note that the CFA value changes over time as
    // well because a fiber may be resumed multiple times from different points
    // on the original stack. This means that our custom CFA directive involves
    // `DW_OP_deref`, which loads data from memory.
    //
    // The expression we're encoding here is that the CFA, the stack pointer of
    // whatever called into `wasmtime_fiber_start`, is:
    //
    //        *($rsp + 0x18) + 0x38
    //
    // $rsp is the stack pointer of `wasmtime_fiber_start` at the time the next
    // instruction after the `.cfi_escape` is executed. Our $rsp at the start
    // of this function is 3 words below stack start (0xAff0 in
    // the diagram in unix.rs). The $rsp to resume at is at 0xAff0, so we
    // add an offset to $rsp to get to that memory location and then we
    // dereference it.
    //
    // After dereferencing, though, we have the $rsp value for
    // `wasmtime_fiber_switch` itself. That's a weird function which sort of
    // and sort of doesn't exist on the stack.  We want to point to the caller
    // of `wasmtime_fiber_switch`, so to do that we need to skip the stack space
    // reserved by `wasmtime_fiber_switch`, which is the 6 saved registers plus
    // the return address of the caller's `call` instruction. Hence we offset
    // another 0x38 bytes.
    .cfi_escape 0x0f, /* DW_CFA_def_cfa_expression */ \
        5,            /* the byte length of this expression */ \
        0x77, 0x18,   /* DW_OP_breg7 (%rsp) + 0x18 */ \
        0x06,         /* DW_OP_deref */ \
        0x23, 0x38    /* DW_OP_plus_uconst 0x38 */

    // And now after we've indicated where our CFA is for our parent function,
    // we can define that where all of the saved registers are located. This
    // uses standard `.cfi` directives which indicate that these registers are
    // all stored relative to the CFA. Note that this order is kept in sync
    // with the above register spills in `wasmtime_fiber_switch`.
    .cfi_rel_offset rip, -8
    .cfi_rel_offset rbp, -16
    .cfi_rel_offset rbx, -24
    .cfi_rel_offset r12, -32
    .cfi_rel_offset r13, -40
    .cfi_rel_offset r14, -48
    .cfi_rel_offset r15, -56

    // Update the CFA expression after each adjustment of $rsp as we load
    // registers to call the entrypoint. The major change is that the $rsp
    // offset is decreasing by 8, and for the last adjustment a 0 offset means
    // we can use DW_OP_reg7.
    popq %rdi
    .cfi_escape 0x0f, 5, 0x77, 0x10, 0x06, 0x23, 0x38
    popq %rax
    .cfi_escape 0x0f, 5, 0x77, 0x08, 0x06, 0x23, 0x38
    popq %rsi
    .cfi_escape 0x0f, 4, 0x57,       0x06, 0x23, 0x38

    // And finally head off into the fiber. Note the `callq` keeps this frame
    // on the stack so all our CFI directives can be read. Additionally this
    // is not expected to ever return, but for safety we put a `ud2` at the end.
    callq *%rax
    ud2
    .cfi_endproc
SIZE(wasmtime_fiber_start)

// Mark that we don't need executable stack.
#if defined(__ELF__)
.section .note.GNU-stack,"",%progbits
#endif
