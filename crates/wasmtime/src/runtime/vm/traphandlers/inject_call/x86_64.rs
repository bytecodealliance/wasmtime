//! x86-64 injected-call trampoline.
//!
//! The purpose of this trampoline is to save all registers to the
//! stack (which we know will be valid at the point that a
//! trap-causing signal occurs in Wasm code) and invoke a
//! hostcall.

use core::arch::naked_asm;

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn injected_call_trampoline(_hostcall: usize, _store: usize) {
    naked_asm!(
        "
        // When control reaches here, we have just returned from a signal
        // handler after rewriting PC to point to this trampoline but updating
        // no other register state. We will have interrupted an instruction known
        // to cause traps, but it is not treated as a register-clobbering
        // instruction; thus, we need to take care to save all registers.
        //
        // We can assume the stack has enough space for this state-saving. We
        // ensure this via our implementation of stack limit checks in
        // Cranelift-compiled code.
        //
        // The signal handler will have placed the address of the hostcall
        // in our first argument register (rdi), and the store value in our
        // second argument register (rsi). These are necessary because
        // we otherwise know nothing about user state here and e.g. where to
        // find the store and look up trampolines.
        //
        // We call the hostcall trampoline with pointers to the places
        // where we saved PC (rip) and the argument registers (rdi/rsi), so
        // it can fill back in the original values before we return.

        // Push a fake return address; it will be filled in by the hostcall.
        push 0
        // This is an ordinary frame as seen by stack-walks.
        push rbp

        // Save all GPRs excep rbp/rsp (saved above and by normal stack
        // discipline, respectively).
        push rax
        push rbx
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11
        push r12
        push r13
        push r14
        push r15

        // N.B.: we don't save rflags; Cranelift-compiled code
        // never assumes it is saved across instructions outside of
        // flag-generation / flag-consumption pairs, and the only
        // resumable traps we are interested in are not flags-related.

        sub rsp, 256 // enough for all 16 XMM registers.
        movdqu [rsp +  0 * 16], xmm0
        movdqu [rsp +  1 * 16], xmm1
        movdqu [rsp +  2 * 16], xmm2
        movdqu [rsp +  3 * 16], xmm3
        movdqu [rsp +  4 * 16], xmm4
        movdqu [rsp +  5 * 16], xmm5
        movdqu [rsp +  6 * 16], xmm6
        movdqu [rsp +  7 * 16], xmm7
        movdqu [rsp +  8 * 16], xmm8
        movdqu [rsp +  9 * 16], xmm9
        movdqu [rsp + 10 * 16], xmm10
        movdqu [rsp + 11 * 16], xmm11
        movdqu [rsp + 12 * 16], xmm12
        movdqu [rsp + 13 * 16], xmm13
        movdqu [rsp + 14 * 16], xmm14
        movdqu [rsp + 15 * 16], xmm15

        // Move host-entry trampoline call target to rax, and generate
        // parameters as addresses to fill in original PC, RDI, and RSI.
        // Note that the trampoline is called with `tail` calling convention.
        mov rax, rdi
        mov rdi, rsi  // vmctx.
        lea rsi, [rsp + 16 * 16 + 15 * 8] // saved RIP above (to restore).
        lea rdx, [rsp + 16 * 16 +  9 * 8] // saved RDI above (to restore).
        lea rcx, [rsp + 16 * 16 +  8 * 8] // saved RSI above (to restore).
        call rax

        // Now restore everything and return normally.
        movdqu xmm0,  [rsp +  0 * 16]
        movdqu xmm1,  [rsp +  1 * 16]
        movdqu xmm2,  [rsp +  2 * 16]
        movdqu xmm3,  [rsp +  3 * 16]
        movdqu xmm4,  [rsp +  4 * 16]
        movdqu xmm5,  [rsp +  5 * 16]
        movdqu xmm6,  [rsp +  6 * 16]
        movdqu xmm7,  [rsp +  7 * 16]
        movdqu xmm8,  [rsp +  8 * 16]
        movdqu xmm9,  [rsp +  9 * 16]
        movdqu xmm10, [rsp + 10 * 16]
        movdqu xmm11, [rsp + 11 * 16]
        movdqu xmm12, [rsp + 12 * 16]
        movdqu xmm13, [rsp + 13 * 16]
        movdqu xmm14, [rsp + 14 * 16]
        movdqu xmm15, [rsp + 15 * 16]
        add rsp, 256

        pop r15
        pop r14
        pop r13
        pop r12
        pop r11
        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rdx
        pop rcx
        pop rbx
        pop rax

        pop rbp
        ret
    ",
    );
}
