//! Routines to bounce control from a signal handler, through a
//! register-preserving asm trampoline, and into a function which causes the
//! fiber to yield

use crate::runtime;
use crate::runtime::vm::traphandlers::raise_preexisting_trap;
use crate::runtime::vm::{Instance, VMContext, VmPtr};
use std::ptr::NonNull;

/// Causes the active fiber to yield in response to an MMU interruption. If, on
/// deeper examination, the interruption is shown to be the result of a stale
/// interrupt-page-ptr cache, the yield is skipped so the fiber can receive its
/// due full timeslice.
///
/// If it does yield, this function returns only after the fiber resumes,
/// appearing to be a normal synchronous function from the standpoint of the
/// caller.
///
/// Returns the address of the interrupt page that should cause this fiber's
/// next interruption. In a scheduling scheme which sets up a relationship
/// between time and pages, the return value effectively chooses the next time
/// at which it is interrupted. In the case of a stale cache, the return value
/// brings the cache up to date.
///
/// `wasm_resume_pc` is the address of the load that triggered the signal; it is
/// re-executed on resume. `trampoline_fp` is a pointer to
/// `task_switch_trampoline`'s frame, which points at the slot where it saved
/// the Wasm caller's frame pointer. Providing these allows this to ape the
/// behavior of the wasm-to-host trampoline so backtrace capture works in the
/// case of fiber cancellation. `load_ptr` is the memory address the interrupt
/// check loaded from and segfaulted on.
///
/// If this fiber gets cancelled within the duration of our yield, this function
/// never returns, instead initiating an unwind.
///
/// # Safety
///
/// `vmctx` must be a currently-entered `VMContext`. In current use,
/// `task_switch_trampoline` ensures the first argument register still holds the
/// Wasm caller's vmctx and sets up the other argument registers as well.
unsafe extern "C" fn maybe_yield_fiber(
    vmctx: NonNull<VMContext>,
    wasm_resume_pc: usize,
    trampoline_fp: usize,
    load_ptr: VmPtr<libc::c_void>,
) -> VmPtr<libc::c_void> {
    let mut next_interrupt_page: Option<VmPtr<libc::c_void>> = None;
    unsafe {
        // is_cancelled means an error occurred and unwind info has been stored
        // in TLS.
        let is_cancelled = !Instance::enter_host_from_wasm(vmctx, |store, _instance| {
            // Mirror the behavior of `block_on!`, which would panic in
            // `assert_ready()` if the Store lacked Asyncness because
            // `yield_now()` is always initially unready.
            debug_assert!(
                store.can_block(),
                "mmu-interruption should automatically enable asyncness on all stores referencing the engine on which it's configured, but somehow asyncness was off"
            );
            let store_ctx = store.vm_store_context();

            // Check the ptr we loaded through that caused the interruption
            // fault. Ideally, it is the same as the one stored in the
            // VMStoreContext, meaning the active Wasm function's local
            // interrupt-page-ptr cache was up to date and we can proceed with
            // switching fibers, care of the code below. However, we have
            // elected to avoid doing an unconditional store to the cache after
            // every interruption check, and so sometimes it is stale. This
            // shows up as the 2 ptrs being different. In this case, we dodge
            // what would be an erroneous yield and return the up-to-date ptr to
            // refresh the cache. In a call tree like A() → B() → C() → D(),
            // where D interrupts, we can expect one of these cache-refreshing
            // bogus interruptions for each Wasm stack frame above it: so, A, B,
            // and C, assuming each of them has any remaining interruption
            // checks to hit. Without this cache-refreshing facility, those 3
            // functions would continue to throw interrupts at every checkpoint
            // until they return. See more about the cache at
            // `FuncEnvironment.mmu_interrupt_page_ptr_var`.
            if let Some(true_ptr) = store_ctx.mmu_interrupt_page_ptr
                && true_ptr != load_ptr
            {
                next_interrupt_page = Some(true_ptr);
                return Ok(());
            }

            // Record Wasm-exit state just as a Cranelift-emitted wasm-to-host
            // trampoline would so that any backtrace capture triggered
            // while we are in the host (in particular, on the cancellation
            // path below) sees a coherent topmost Wasm activation. Otherwise,
            // it hits a debug assert and crashes.
            *store_ctx.last_wasm_exit_pc.get() = wasm_resume_pc;
            *store_ctx.last_wasm_exit_trampoline_fp.get() = trampoline_fp;

            // Actually switch fibers. (No Wasm runs during this yield.)
            //
            // `block_on()` documents that the store may not be used and no
            // other fiber resumed until this one is. Thus, the store is
            // idle--empty of running fibers--during the yield. The fall of the
            // store's fiber count to 0, overseen by `decrement_fibers()`,
            // ensures that the old interrupt page is released. A new one is
            // then acquired just when the first fiber on the store begins to
            // run.
            let result = store.with_blocking(|_store, cx| cx.block_on(runtime::store::yield_now()));

            if result.is_ok() {
                // Clear the exit state again so it doesn't appear stale once we
                // resume Wasm.
                let store_ctx = store.vm_store_context();
                *store_ctx.last_wasm_exit_pc.get() = 0;
                *store_ctx.last_wasm_exit_trampoline_fp.get() = 0;

                // Get the address of the latest interrupt page.
                next_interrupt_page = store_ctx.mmu_interrupt_page_ptr;
            }
            // Else leave exit state in place so `record_unwind` (called via
            // `raise_preexisting_trap` below) can capture a backtrace.

            result
        });
        if is_cancelled {
            // `block_on()` returned an Err, meaning this fiber has been
            // cancelled and needs to exit. We unwind it using
            // `raise_preexisting_trap()`, which, on native targets (the only
            // ones MMU epochs apply to), never returns, doing a longjmp out and
            // thus making anything in stack frames above irrelevant. Thus, it
            // doesn't matter that we never get to restore registers in
            // `task_switch_trampoline()` or jmp back to the signal-raising
            // address.
            Instance::enter_host_from_wasm(vmctx, |store, _instance| {
                raise_preexisting_trap(store);
            });
        }
    }

    // We will never reach here if a trap is raised and everything unwinds.
    next_interrupt_page.expect(
        "under MMU interruption, a running store should always have an interrupt page ptr assigned",
    )
}

cfg_select! {
    target_arch = "x86_64" => {
        mod x86_64;
        use x86_64 as imp;
    }
    target_arch = "aarch64" => {
        mod aarch64;
        use aarch64 as imp;
    }
    _ => {
        // This could happen only if `has_mmu_interruption` is accidentally
        // broadened to allow archs not supported here.
        compile_error!("MMU interruption is not supported on this architecture");
    }
}

/// Arranges for the `ucontext` of an MMU-interrupt segfault to resume at
/// `task_switch_trampoline` rather than at the original `return_address`.
///
/// Leaves the original `return_address` in the scratch register where the
/// trampoline can find it.
///
/// The vmctx is already in the first argument register, pinned there by
/// `dead_load_with_context`, so the trampoline needs no help finding it.
pub(crate) use imp::resume_into_task_switch_trampoline;
