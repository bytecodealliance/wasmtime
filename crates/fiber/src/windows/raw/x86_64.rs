// A WORD OF CAUTION
//
// This file needs to be kept in sync with itself and with the layout
// documented in `windows/raw.rs`, and `SUSPENDED_FRAME` with the frame that
// `wasmtime_fiber_suspend` spills.

use core::arch::naked_asm;
use windows_sys::Win32::System::Threading::SwitchToFiber;

pub const SUPPORTED_ARCH: bool = true;

/// The indices, in words from a suspended fiber's saved stack pointer, of the
/// frame pointer and return address that `wasmtime_fiber_suspend` spills.
pub const SUSPENDED_FRAME: Option<(usize, usize)> = Some((28, 29));

/// Suspends the raw fiber whose stack top is `top`, returning when it is next
/// resumed.
#[unsafe(naked)]
pub unsafe extern "C" fn wasmtime_fiber_suspend(top: *mut u8 /* rcx */) {
    naked_asm!(
        "
        .seh_proc {suspend}

        // Spill all nonvolatile registers.
        push rbp
        .seh_pushreg rbp
        push rbx
        .seh_pushreg rbx
        push rdi
        .seh_pushreg rdi
        push rsi
        .seh_pushreg rsi
        push r12
        .seh_pushreg r12
        push r13
        .seh_pushreg r13
        push r14
        .seh_pushreg r14
        push r15
        .seh_pushreg r15
        sub rsp, 0xa8
        .seh_stackalloc 0xa8
        movaps [rsp + 0x00], xmm6
        .seh_savexmm xmm6, 0x00
        movaps [rsp + 0x10], xmm7
        .seh_savexmm xmm7, 0x10
        movaps [rsp + 0x20], xmm8
        .seh_savexmm xmm8, 0x20
        movaps [rsp + 0x30], xmm9
        .seh_savexmm xmm9, 0x30
        movaps [rsp + 0x40], xmm10
        .seh_savexmm xmm10, 0x40
        movaps [rsp + 0x50], xmm11
        .seh_savexmm xmm11, 0x50
        movaps [rsp + 0x60], xmm12
        .seh_savexmm xmm12, 0x60
        movaps [rsp + 0x70], xmm13
        .seh_savexmm xmm13, 0x70
        movaps [rsp + 0x80], xmm14
        .seh_savexmm xmm14, 0x80
        movaps [rsp + 0x90], xmm15
        .seh_savexmm xmm15, 0x90
        // Unwind through the frame pointer while on the canonical frame.
        mov rbp, rsp
        .seh_setframe rbp, 0
        .seh_endprologue

        // Store where to resume, then switch to the host fiber from the
        // canonical frame, leaving shadow space above it.
        mov -0x10[rcx], rsp
        mov rax, rcx
        mov rcx, -0x08[rax]
        lea rsp, -0x30[rax]
        call {switch_to_fiber}

        // Resumed. Only the stack pointer is meaningful here: Windows restored
        // the other registers from whichever suspension last saved them, which
        // need not be the one resumed after a snapshot was restored. Take the
        // registers spilled above from the stack pointer in the slot.
        mov rbp, 0x20[rsp]
        movaps xmm6, [rbp + 0x00]
        movaps xmm7, [rbp + 0x10]
        movaps xmm8, [rbp + 0x20]
        movaps xmm9, [rbp + 0x30]
        movaps xmm10, [rbp + 0x40]
        movaps xmm11, [rbp + 0x50]
        movaps xmm12, [rbp + 0x60]
        movaps xmm13, [rbp + 0x70]
        movaps xmm14, [rbp + 0x80]
        movaps xmm15, [rbp + 0x90]
        lea rsp, [rbp + 0xa8]
        pop r15
        pop r14
        pop r13
        pop r12
        pop rsi
        pop rdi
        pop rbx
        pop rbp
        ret
        .seh_endproc
        ",
        suspend = sym wasmtime_fiber_suspend,
        switch_to_fiber = sym SwitchToFiber,
    );
}

/// Sets up the top of the stack as documented in `windows/raw.rs`, reports `top`
/// through `top_out`, suspends, and then calls `entry(arg, top)` when first
/// resumed.
///
/// This never returns, so it does not preserve its caller's nonvolatile
/// registers; the unwind information only describes how to find the caller's
/// frame.
#[unsafe(naked)]
pub unsafe extern "C" fn wasmtime_fiber_start(
    entry: crate::RawFiberEntry,  // rcx
    arg: *mut u8,                 // rdx
    host: *mut core::ffi::c_void, // r8
    top_out: *mut *mut u8,        // r9
) -> ! {
    naked_asm!(
        "
        .seh_proc {start}
        push rbp
        .seh_pushreg rbp
        mov rbp, rsp
        .seh_setframe rbp, 0
        .seh_endprologue

        // The frame pointer is `top`, which is 16-byte aligned since the
        // stack was 8 bytes off alignment at entry.
        mov -0x08[rbp], r8
        mov [r9], rbp
        mov r12, rcx
        mov r13, rdx

        // Move the stack pointer below the reserved slots and the canonical
        // frame, touching the page in between first so that Windows grows the
        // stack one page at a time.
        mov rax, -0x1000[rbp]
        lea rsp, -0x1020[rbp]

        // The initial suspension.
        mov rcx, rbp
        call {suspend}

        // Resumed for the first time, or from a snapshot of the initial
        // suspension. Call the entry point, which never returns.
        mov rcx, r13
        mov rdx, rbp
        call r12
        ud2
        .seh_endproc
        ",
        start = sym wasmtime_fiber_start,
        suspend = sym wasmtime_fiber_suspend,
    );
}
