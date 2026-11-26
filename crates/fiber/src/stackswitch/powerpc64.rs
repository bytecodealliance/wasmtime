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
// This file implements the 64-bit ELFv2 ABI only, where a function pointer is
// the address of the function's code. ELFv1 function descriptors are not
// handled here.
//
// The vector registers are saved with AltiVec instructions. The default CPUs
// of all ELFv2 targets have AltiVec, but code built for a CPU without it, like
// the e5500, will fault on the first switch.

use core::arch::naked_asm;

/// The indices, in words from a suspended raw fiber's saved stack pointer, of
/// the frame pointer and return address that `wasmtime_fiber_switch_` spills
/// for its caller. As on s390x the "frame pointer" is the caller's stack
/// pointer, which addresses its back chain; `stdu` stores it in the first word
/// of our frame, and the return address goes into the frame header of the
/// caller, right above our 512-byte frame. Keep in sync with that routine.
pub(crate) const SUSPENDED_FRAME: Option<(usize, usize)> = Some((0, 66));

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* r3 */) {
    naked_asm!(
        "
        // Save the link register and the condition register in the frame
        // header of our caller, which is where the ABI wants them, and then
        // allocate a 512-byte frame for everything else.
        mflr 0
        std 0, 16(1)
        mfcr 0
        stw 0, 8(1)
        stdu 1, -512(1)

        // We're switching to arbitrary code somewhere else, so pessimistically
        // assume that all callee-save register are clobbered. This means we
        // need to save/restore all of them: v20-v31, f14-f31 and r14-r31.
        //
        // The thread pointer in r13 is deliberately left alone since a fiber
        // may be resumed on a different thread, and the TOC pointer in r2 is
        // never modified here.
        //
        // Note that this order for saving is important since we use CFI
        // directives below to point to where all the saved registers are.
        li 11, 32
        stvx 20, 11, 1
        li 11, 48
        stvx 21, 11, 1
        li 11, 64
        stvx 22, 11, 1
        li 11, 80
        stvx 23, 11, 1
        li 11, 96
        stvx 24, 11, 1
        li 11, 112
        stvx 25, 11, 1
        li 11, 128
        stvx 26, 11, 1
        li 11, 144
        stvx 27, 11, 1
        li 11, 160
        stvx 28, 11, 1
        li 11, 176
        stvx 29, 11, 1
        li 11, 192
        stvx 30, 11, 1
        li 11, 208
        stvx 31, 11, 1
        stfd 14, 224(1)
        stfd 15, 232(1)
        stfd 16, 240(1)
        stfd 17, 248(1)
        stfd 18, 256(1)
        stfd 19, 264(1)
        stfd 20, 272(1)
        stfd 21, 280(1)
        stfd 22, 288(1)
        stfd 23, 296(1)
        stfd 24, 304(1)
        stfd 25, 312(1)
        stfd 26, 320(1)
        stfd 27, 328(1)
        stfd 28, 336(1)
        stfd 29, 344(1)
        stfd 30, 352(1)
        stfd 31, 360(1)
        std 14, 368(1)
        std 15, 376(1)
        std 16, 384(1)
        std 17, 392(1)
        std 18, 400(1)
        std 19, 408(1)
        std 20, 416(1)
        std 21, 424(1)
        std 22, 432(1)
        std 23, 440(1)
        std 24, 448(1)
        std 25, 456(1)
        std 26, 464(1)
        std 27, 472(1)
        std 28, 480(1)
        std 29, 488(1)
        std 30, 496(1)
        std 31, 504(1)

        // Load pointer that we're going to resume at and store where we're
        // going to get resumed from. This is in accordance with the diagram at
        // the top of unix.rs.
        ld 4, -16(3)
        std 1, -16(3)

        // Swap stacks and restore all our callee-saved registers
        mr 1, 4
        li 11, 32
        lvx 20, 11, 1
        li 11, 48
        lvx 21, 11, 1
        li 11, 64
        lvx 22, 11, 1
        li 11, 80
        lvx 23, 11, 1
        li 11, 96
        lvx 24, 11, 1
        li 11, 112
        lvx 25, 11, 1
        li 11, 128
        lvx 26, 11, 1
        li 11, 144
        lvx 27, 11, 1
        li 11, 160
        lvx 28, 11, 1
        li 11, 176
        lvx 29, 11, 1
        li 11, 192
        lvx 30, 11, 1
        li 11, 208
        lvx 31, 11, 1
        lfd 14, 224(1)
        lfd 15, 232(1)
        lfd 16, 240(1)
        lfd 17, 248(1)
        lfd 18, 256(1)
        lfd 19, 264(1)
        lfd 20, 272(1)
        lfd 21, 280(1)
        lfd 22, 288(1)
        lfd 23, 296(1)
        lfd 24, 304(1)
        lfd 25, 312(1)
        lfd 26, 320(1)
        lfd 27, 328(1)
        lfd 28, 336(1)
        lfd 29, 344(1)
        lfd 30, 352(1)
        lfd 31, 360(1)
        ld 14, 368(1)
        ld 15, 376(1)
        ld 16, 384(1)
        ld 17, 392(1)
        ld 18, 400(1)
        ld 19, 408(1)
        ld 20, 416(1)
        ld 21, 424(1)
        ld 22, 432(1)
        ld 23, 440(1)
        ld 24, 448(1)
        ld 25, 456(1)
        ld 26, 464(1)
        ld 27, 472(1)
        ld 28, 480(1)
        ld 29, 488(1)
        ld 30, 496(1)
        ld 31, 504(1)

        // Pop our frame, then restore the callee-saved fields of the condition
        // register (cr2-cr4) and the return address from the frame header
        // above it.
        addi 1, 1, 512
        lwz 0, 8(1)
        mtcrf 0x38, 0
        ld 0, 16(1)
        mtlr 0
        blr
        ",
    );
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8) -> *mut u8,
    entry_arg0: *mut u8,
) {
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        // The frame of `wasmtime_fiber_switch_`, which is popped the first
        // time this fiber is switched to.
        switch_back_chain: *mut u8,
        switch_frame_header: [u64; 3],
        vrs: [u64; 24],
        fprs: [u64; 18],
        r14: *mut u8,
        r15: *mut u8,
        r16: *mut u8,
        r17: *mut u8,
        gprs: [u64; 14],

        // The frame of `wasmtime_fiber_start`. The ABI lets a callee store the
        // condition register and its return address in the header of its
        // caller's frame, so `wasmtime_fiber_switch_` loads these two from
        // here and the functions `wasmtime_fiber_start` calls store theirs
        // here as well.
        back_chain: *mut u8,
        cr: u64,
        lr: *mut u8,
        toc: *mut u8,
        parameter_save_area: [u64; 8],

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            switch_back_chain: (&raw mut (*initial_stack).back_chain).cast(),
            r14: entry_point as *mut u8,
            r15: entry_arg0,
            r16: top_of_stack,
            r17: wasmtime_fiber_switch_ as *mut u8,
            lr: wasmtime_fiber_start as *mut u8,
            last_sp: initial_stack.cast(),
            ..InitialStack::default()
        });
    }
}

// See the x86_64 file for more commentary on what these CFI directives are
// doing. Like over there note that the relative offsets to registers here
// match the frame layout in `wasmtime_fiber_switch_`.
#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_start() -> ! {
    naked_asm!(
        "
        .cfi_startproc simple
        .cfi_def_cfa_offset 0

        // Our stack pointer is 112 bytes below the top of the stack here: the
        // 16 bytes reserved by unix.rs plus our 96-byte frame, so the stack
        // pointer to resume the parent at is 96 bytes above it. Loading that
        // gives the stack pointer of `wasmtime_fiber_switch_`, and its frame is
        // 512 bytes large.
        .cfi_escape 0x0f,      /* DW_CFA_def_cfa_expression */ \
            7,                 /* the byte length of this expression */ \
            0x71, 0xe0, 0x00,  /* DW_OP_breg1 96 */ \
            0x06,              /* DW_OP_deref */ \
            0x23, 0x80, 0x04   /* DW_OP_plus_uconst 512 */

        .cfi_rel_offset lr, 16
        .cfi_rel_offset r31, -8
        .cfi_rel_offset r30, -16
        .cfi_rel_offset r29, -24
        .cfi_rel_offset r28, -32
        .cfi_rel_offset r27, -40
        .cfi_rel_offset r26, -48
        .cfi_rel_offset r25, -56
        .cfi_rel_offset r24, -64
        .cfi_rel_offset r23, -72
        .cfi_rel_offset r22, -80
        .cfi_rel_offset r21, -88
        .cfi_rel_offset r20, -96
        .cfi_rel_offset r19, -104
        .cfi_rel_offset r18, -112
        .cfi_rel_offset r17, -120
        .cfi_rel_offset r16, -128
        .cfi_rel_offset r15, -136
        .cfi_rel_offset r14, -144

        // Load our two arguments prepared by `wasmtime_fiber_init` and call
        // the entry point. An indirect call wants the address of the callee in
        // r12, and our TOC pointer is saved in its slot of the frame header
        // across the call. Note that this is a function call so our frame
        // stays on the stack to backtrace through.
        std 2, 24(1)
        mr 3, 15      // entry_arg0
        mr 4, 16      // top_of_stack
        mr 12, 14     // entry_point
        mtctr 12
        bctrl
        ld 2, 24(1)

        // The entry function returns where to switch to as the final switch,
        // so that's performed here.
        mr 12, 17     // wasmtime_fiber_switch_
        mtctr 12
        bctrl

        // Unreachable, here for safety. This should help catch unexpected
        // behaviors.
        trap
        .cfi_endproc
        ",
    );
}
