use super::maybe_yield_fiber;
use core::arch::naked_asm;

/// Switches tasks in response to a signal thrown under MMU-based epoch
/// interruption.
///
/// Saves register state, makes a host call to switch tasks, restores state, and
/// jumps back to the load instruction that triggered the signal, re-executing
/// it. (The interrupt page has been replaced with an unprotected one by then,
/// so the retry succeeds.) The address of that load instruction has been
/// squirreled away by the signal handler in the scratch register that
/// `dead_load_with_context` reserves: r10 on x64, x9 on aarch64. The signal
/// handler has also left the address of the vmctx in the first argument
/// register (rdi on x64, x0 on aarch64), where `dead_load_with_context` pinned
/// it. Finally, this returns the next interrupt page ptr to use (in r11 for
/// x64, x10 for aarch64).
///
/// # Safety
///
/// This is invoked only by the signal handler `trap_handler`, which fulfills
/// the prerequisites about register contents. It is reached not by a
/// straightforward call but by jimmying the ucontext to "resume into" this
/// (instead of the trapping location) when the handler exits.
///
/// This uses about 328b of stack space (on the normal stack, not the
/// sigaltstack) to save registers + a bit more to run `maybe_yield_fiber()`. In
/// practice, this should not create uncaught stack overflows because (1) this
/// trampoline runs only in async, (2) the default async_stack_size is 2MiB, of
/// which only 512KiB is reserved for the Wasm stack, and (3) the fiber stack
/// has a 4KiB guard page at the bottom, which causes `abort_stack_overflow()`
/// to run if we do crash into it.
///
/// When control reaches here, we have just returned from a signal handler after
/// rewriting PC to point to this trampoline but updating no other register
/// state.
///
/// The stack has enough space for this state-saving, ensured by the stack-limit
/// checks in Cranelift-compiled code.
#[unsafe(naked)]
pub(super) unsafe extern "C" fn task_switch_trampoline(_vmctx: usize) {
    naked_asm!(
        "
        // Push a fake return address just to keep the stack 16b-aligned for the
        // call, as SysV x64 demands.
        push 0
        // This is an ordinary frame as seen by stack-walks. We also establish
        // rbp as our frame pointer so that we can hand it to
        // `maybe_yield_fiber` as the trampoline FP: the saved wasm rbp lives
        // at [rbp], which is exactly what
        // `VMStoreContext::wasm_exit_fp_from_trampoline_fp` expects.
        push rbp

        // Preserve caller-saved GPRs except rbp and rsp (saved above and by
        // normal stack discipline, respectively). The interrupt location
        // doesn't know anything is being 'called', so we have to do the saving
        // ourselves. `maybe_yield_fiber()` and anything down that call chain
        // preserve the callee-saved registers (r12-r15 and rbx).
        //
        // We don't have to save r11 because `dead_load_with_context` defs it.

        push rdx

        // Now that rdx is pushed, take an intermission to put the original
        // value of rbp into it, for use as arg 3 to maybe_yield_fiber().
        lea rdx, [rsp + 8]

        // And do the rest of the GPRs.
        push rax
        push rcx
        push rdi
        push rsi
        push r8
        push r9
        push r10

        // N.B.: we don't save rflags; Cranelift-compiled code
        // never assumes it is saved across instructions outside of
        // flag-generation / flag-consumption pairs, and the only
        // resumable traps we are interested in are not flags-related.

        // 256 for the 16 XMM registers. No padding is needed: an even number
        // of GPRs was pushed above, so the stack is already 16b-aligned.
        sub rsp, 256
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

        // vmctx is already in rdi, care of the signal handler. Get the 2nd
        // (`wasm_resume_pc`) arg ready, the 3rd (`trampoline_fp`) already
        // having been put in rdx above...
        mov rsi, r10
        // Ready the 4th (`load_ptr`) arg, which is already in r11, having been
        // pinned there by `dead_load_with_context`.
        mov rcx, r11
        // ...and call maybe_yield_fiber() to do the task switch.
        call {}
        // Move return value (the new MMU interrupt page ptr) to r11 to be
        // returned by the dead_load_with_context instruction, which we're in
        // the middle of.
        mov r11, rax

        // Restore registers.
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

        pop r10
        pop r9
        pop r8
        pop rsi
        pop rdi
        pop rcx
        pop rax
        pop rdx

        pop rbp
        // Pop off the fake return address.
        add rsp, 8

        // Resume at the load instruction that triggered the signal handler,
        // re-executing it.
        jmp r10
        ",
        sym maybe_yield_fiber
    );
}

pub(crate) fn resume_into_task_switch_trampoline(
    ucontext: &mut libc::ucontext_t,
    return_address: *const (),
) {
    ucontext.uc_mcontext.gregs[libc::REG_RIP as usize] = task_switch_trampoline as *const () as i64;
    ucontext.uc_mcontext.gregs[libc::REG_R10 as usize] = return_address as i64;
}
