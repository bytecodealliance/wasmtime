// A WORD OF CAUTION
//
// This file needs to be kept in sync with itself and with the layout
// documented in `raw/windows.rs`, and `SUSPENDED_FRAME` with the frame that
// `wasmtime_fiber_suspend` spills. It closely follows `x86_64.rs`.

use core::arch::naked_asm;
use windows_sys::Win32::System::Threading::SwitchToFiber;

pub const SUPPORTED_ARCH: bool = true;

/// The indices, in words from a suspended fiber's saved stack pointer, of the
/// frame pointer and return address that `wasmtime_fiber_suspend` spills.
pub const SUSPENDED_FRAME: Option<(usize, usize)> = Some((0, 1));

/// Suspends the raw fiber whose stack top is `top`, returning when it is next
/// resumed.
#[unsafe(naked)]
pub unsafe extern "C" fn wasmtime_fiber_suspend(top: *mut u8 /* x0 */) {
    naked_asm!(
        "
        .seh_proc {suspend}

        // Spill all nonvolatile registers, ending with a frame record.
        stp d8, d9, [sp, -16]!
        .seh_save_fregp_x d8, 16
        stp d10, d11, [sp, -16]!
        .seh_save_fregp_x d10, 16
        stp d12, d13, [sp, -16]!
        .seh_save_fregp_x d12, 16
        stp d14, d15, [sp, -16]!
        .seh_save_fregp_x d14, 16
        stp x19, x20, [sp, -16]!
        .seh_save_regp_x x19, 16
        stp x21, x22, [sp, -16]!
        .seh_save_regp_x x21, 16
        stp x23, x24, [sp, -16]!
        .seh_save_regp_x x23, 16
        stp x25, x26, [sp, -16]!
        .seh_save_regp_x x25, 16
        stp x27, x28, [sp, -16]!
        .seh_save_regp_x x27, 16
        stp x29, x30, [sp, -16]!
        .seh_save_fplr_x 16
        // Unwind through the frame pointer while on the canonical frame.
        mov x29, sp
        .seh_set_fp
        .seh_endprologue

        // Store where to resume, then switch to the host fiber from the
        // canonical frame.
        mov x9, sp
        str x9, [x0, -0x10]
        ldr x9, [x0, -0x08]
        sub sp, x0, 0x10
        mov x0, x9
        bl {switch_to_fiber}

        // Resumed. Only the stack pointer is meaningful here: Windows restored
        // the other registers from whichever suspension last saved them, which
        // need not be the one resumed after a snapshot was restored. Take the
        // registers spilled above from the stack pointer in the slot.
        ldr x29, [sp]
        .seh_startepilogue
        mov sp, x29
        .seh_set_fp
        ldp x29, x30, [sp], 16
        .seh_save_fplr_x 16
        ldp x27, x28, [sp], 16
        .seh_save_regp_x x27, 16
        ldp x25, x26, [sp], 16
        .seh_save_regp_x x25, 16
        ldp x23, x24, [sp], 16
        .seh_save_regp_x x23, 16
        ldp x21, x22, [sp], 16
        .seh_save_regp_x x21, 16
        ldp x19, x20, [sp], 16
        .seh_save_regp_x x19, 16
        ldp d14, d15, [sp], 16
        .seh_save_fregp_x d14, 16
        ldp d12, d13, [sp], 16
        .seh_save_fregp_x d12, 16
        ldp d10, d11, [sp], 16
        .seh_save_fregp_x d10, 16
        ldp d8, d9, [sp], 16
        .seh_save_fregp_x d8, 16
        .seh_endepilogue
        ret
        .seh_endproc
        ",
        suspend = sym wasmtime_fiber_suspend,
        switch_to_fiber = sym SwitchToFiber,
    );
}

/// Sets up the top of the stack as documented in `raw/windows.rs`, reports `top`
/// through `top_out`, suspends, and then calls `entry(arg, top)` when first
/// resumed.
///
/// This never returns, so it does not preserve its caller's nonvolatile
/// registers; the unwind information only describes how to find the caller's
/// frame.
#[unsafe(naked)]
pub unsafe extern "C" fn wasmtime_fiber_start(
    entry: crate::RawFiberEntry,  // x0
    arg: *mut u8,                 // x1
    host: *mut core::ffi::c_void, // x2
    top_out: *mut *mut u8,        // x3
) -> ! {
    naked_asm!(
        "
        .seh_proc {start}
        stp x29, x30, [sp, -16]!
        .seh_save_fplr_x 16
        mov x29, sp
        .seh_set_fp
        .seh_endprologue

        // The frame pointer is `top`.
        str x2, [x29, -0x08]
        str x29, [x3]
        mov x19, x0
        mov x20, x1

        // Move the stack pointer below the reserved slots and the canonical
        // frame, touching the page in between first so that Windows grows the
        // stack one page at a time.
        sub x9, x29, 0x1000
        ldr xzr, [x9]
        sub sp, x9, 0x10

        // The initial suspension.
        mov x0, x29
        bl {suspend}

        // Resumed for the first time, or from a snapshot of the initial
        // suspension. Call the entry point, which never returns.
        mov x0, x20
        mov x1, x29
        blr x19
        brk 0xf1b3
        .seh_endproc
        ",
        start = sym wasmtime_fiber_start,
        suspend = sym wasmtime_fiber_suspend,
    );
}
