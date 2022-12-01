//! Backtrace and stack walking functionality for Wasm.
//!
//! Walking the Wasm stack is comprised of
//!
//! 1. identifying sequences of contiguous Wasm frames on the stack
//!    (i.e. skipping over native host frames), and
//!
//! 2. walking the Wasm frames within such a sequence.
//!
//! To perform (1) we maintain the entry stack pointer (SP) and exit frame
//! pointer (FP) and program counter (PC) each time we call into Wasm and Wasm
//! calls into the host via trampolines (see
//! `crates/runtime/src/trampolines`). The most recent entry is stored in
//! `VMRuntimeLimits` and older entries are saved in `CallThreadState`. This
//! lets us identify ranges of contiguous Wasm frames on the stack.
//!
//! To solve (2) and walk the Wasm frames within a region of contiguous Wasm
//! frames on the stack, we configure Cranelift's `preserve_frame_pointers =
//! true` setting. Then we can do simple frame pointer traversal starting at the
//! exit FP and stopping once we reach the entry SP (meaning that the next older
//! frame is a host frame).

use crate::traphandlers::{tls, CallThreadState};
use cfg_if::cfg_if;
use std::ops::ControlFlow;

// Architecture-specific bits for stack walking. Each of these modules should
// define and export the following functions:
//
// * `unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize`
// * `unsafe fn get_next_older_fp_from_fp(fp: usize) -> usize`
// * `fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool`
// * `fn assert_entry_sp_is_aligned(sp: usize)`
// * `fn assert_fp_is_aligned(fp: usize)`
cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        use x86_64 as arch;
    } else if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        use aarch64 as arch;
    } else if #[cfg(target_arch = "s390x")] {
        mod s390x;
        use s390x as arch;
    } else if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        use riscv64 as arch;
    } else {
        compile_error!("unsupported architecture");
    }
}

/// A WebAssembly stack trace.
#[derive(Debug)]
pub struct Backtrace(Vec<Frame>);

/// A stack frame within a Wasm stack trace.
#[derive(Debug)]
pub struct Frame {
    pc: usize,
    fp: usize,
}

impl Frame {
    /// Get this frame's program counter.
    pub fn pc(&self) -> usize {
        self.pc
    }

    /// Get this frame's frame pointer.
    pub fn fp(&self) -> usize {
        self.fp
    }
}

impl Backtrace {
    /// Returns an empty backtrace
    pub fn empty() -> Backtrace {
        Backtrace(Vec::new())
    }

    /// Capture the current Wasm stack in a backtrace.
    pub fn new() -> Backtrace {
        tls::with(|state| match state {
            Some(state) => unsafe { Self::new_with_trap_state(state, None) },
            None => Backtrace(vec![]),
        })
    }

    /// Capture the current Wasm stack trace.
    ///
    /// If Wasm hit a trap, and we calling this from the trap handler, then the
    /// Wasm exit trampoline didn't run, and we use the provided PC and FP
    /// instead of looking them up in `VMRuntimeLimits`.
    pub(crate) unsafe fn new_with_trap_state(
        state: &CallThreadState,
        trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Backtrace {
        let mut frames = vec![];
        Self::trace_with_trap_state(state, trap_pc_and_fp, |frame| {
            frames.push(frame);
            ControlFlow::Continue(())
        });
        Backtrace(frames)
    }

    /// Walk the current Wasm stack, calling `f` for each frame we walk.
    pub fn trace(f: impl FnMut(Frame) -> ControlFlow<()>) {
        tls::with(|state| match state {
            Some(state) => unsafe { Self::trace_with_trap_state(state, None, f) },
            None => {}
        });
    }

    /// Walk the current Wasm stack, calling `f` for each frame we walk.
    ///
    /// If Wasm hit a trap, and we calling this from the trap handler, then the
    /// Wasm exit trampoline didn't run, and we use the provided PC and FP
    /// instead of looking them up in `VMRuntimeLimits`.
    pub(crate) unsafe fn trace_with_trap_state(
        state: &CallThreadState,
        trap_pc_and_fp: Option<(usize, usize)>,
        mut f: impl FnMut(Frame) -> ControlFlow<()>,
    ) {
        log::trace!("====== Capturing Backtrace ======");
        let (last_wasm_exit_pc, last_wasm_exit_fp) = match trap_pc_and_fp {
            // If we exited Wasm by catching a trap, then the Wasm-to-host
            // trampoline did not get a chance to save the last Wasm PC and FP,
            // and we need to use the plumbed-through values instead.
            Some((pc, fp)) => (pc, fp),
            // Either there is no Wasm currently on the stack, or we exited Wasm
            // through the Wasm-to-host trampoline.
            None => {
                let pc = *(*state.limits).last_wasm_exit_pc.get();
                let fp = *(*state.limits).last_wasm_exit_fp.get();
                assert_ne!(pc, 0);
                (pc, fp)
            }
        };

        // Trace through the first contiguous sequence of Wasm frames on the
        // stack.
        if let ControlFlow::Break(()) = Self::trace_through_wasm(
            last_wasm_exit_pc,
            last_wasm_exit_fp,
            *(*state.limits).last_wasm_entry_sp.get(),
            &mut f,
        ) {
            log::trace!("====== Done Capturing Backtrace ======");
            return;
        }

        // And then trace through each of the older contiguous sequences of Wasm
        // frames on the stack.
        for state in state.iter() {
            // If there is no previous call state, then there is nothing more to
            // trace through (since each `CallTheadState` saves the *previous*
            // call into Wasm's saved registers, and the youngest call into
            // Wasm's registers are saved in the `VMRuntimeLimits`)
            if state.prev().is_null() {
                debug_assert_eq!(state.old_last_wasm_exit_pc(), 0);
                debug_assert_eq!(state.old_last_wasm_exit_fp(), 0);
                debug_assert_eq!(state.old_last_wasm_entry_sp(), 0);
                log::trace!("====== Done Capturing Backtrace ======");
                return;
            }

            if let ControlFlow::Break(()) = Self::trace_through_wasm(
                state.old_last_wasm_exit_pc(),
                state.old_last_wasm_exit_fp(),
                state.old_last_wasm_entry_sp(),
                &mut f,
            ) {
                log::trace!("====== Done Capturing Backtrace ======");
                return;
            }
        }

        unreachable!()
    }

    /// Walk through a contiguous sequence of Wasm frames starting with the
    /// frame at the given PC and FP and ending at `first_wasm_sp`.
    unsafe fn trace_through_wasm(
        mut pc: usize,
        mut fp: usize,
        first_wasm_sp: usize,
        mut f: impl FnMut(Frame) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        log::trace!("=== Tracing through contiguous sequence of Wasm frames ===");
        log::trace!("first_wasm_sp = 0x{:016x}", first_wasm_sp);
        log::trace!("   initial pc = 0x{:016x}", pc);
        log::trace!("   initial fp = 0x{:016x}", fp);

        // In our host-to-Wasm trampoline, we save `-1` as a sentinal SP
        // value for when the callee is not actually a core Wasm
        // function (as determined by looking at the callee `vmctx`). If
        // we encounter `-1`, this is an empty sequence of Wasm frames
        // where a host called a host function so the following
        // happened:
        //
        // * We entered the host-to-wasm-trampoline, saved (an invalid
        //   sentinal for) entry SP, and tail called to the "Wasm"
        //   callee,
        //
        // * entered the Wasm-to-host trampoline, saved the exit FP and
        //   PC, and tail called to the host callee,
        //
        // * and are now in host code.
        //
        // Ultimately, this means that there are 0 Wasm frames in this
        // contiguous sequence of Wasm frames, and we have nothing to
        // walk through here.
        if first_wasm_sp == -1_isize as usize {
            log::trace!("=== Done tracing (empty sequence of Wasm frames) ===");
            return ControlFlow::Continue(());
        }

        // We use `0` as a sentinal value for when there is not any Wasm
        // on the stack and these values are non-existant. If we
        // actually entered Wasm (see above guard for `-1`) then, then
        // by the time we got here we should have either exited Wasm
        // through the Wasm-to-host trampoline and properly set these
        // values, or we should have caught a trap in a signal handler
        // and also properly recovered these values in that case.
        assert_ne!(pc, 0);
        assert_ne!(fp, 0);
        assert_ne!(first_wasm_sp, 0);

        // The stack grows down, and therefore any frame pointer we are
        // dealing with should be less than the stack pointer on entry
        // to Wasm.
        assert!(first_wasm_sp >= fp, "{first_wasm_sp:#x} >= {fp:#x}");

        arch::assert_entry_sp_is_aligned(first_wasm_sp);

        loop {
            arch::assert_fp_is_aligned(fp);

            log::trace!("--- Tracing through one Wasm frame ---");
            log::trace!("pc = {:p}", pc as *const ());
            log::trace!("fp = {:p}", fp as *const ());

            f(Frame { pc, fp })?;

            // If our FP has reached the SP upon entry to Wasm from the
            // host, then we've successfully walked all the Wasm frames,
            // and have now reached a host frame. We're done iterating
            // through this contiguous sequence of Wasm frames.
            if arch::reached_entry_sp(fp, first_wasm_sp) {
                log::trace!("=== Done tracing contiguous sequence of Wasm frames ===");
                return ControlFlow::Continue(());
            }

            // If we didn't return above, then we know we are still in a
            // Wasm frame, and since Cranelift maintains frame pointers,
            // we know that the FP isn't an arbitrary value and it is
            // safe to dereference it to read the next PC/FP.

            pc = arch::get_next_older_pc_from_fp(fp);

            // We rely on this offset being zero for all supported architectures
            // in `crates/cranelift/src/component/compiler.rs` when we set the
            // Wasm exit FP. If this ever changes, we will need to update that
            // code as well!
            assert_eq!(arch::NEXT_OLDER_FP_FROM_FP_OFFSET, 0);

            let next_older_fp = *(fp as *mut usize).add(arch::NEXT_OLDER_FP_FROM_FP_OFFSET);
            // Because the stack always grows down, the older FP must be greater
            // than the current FP.
            assert!(next_older_fp > fp, "{next_older_fp:#x} > {fp:#x}");
            fp = next_older_fp;
        }
    }

    /// Iterate over the frames inside this backtrace.
    pub fn frames<'a>(&'a self) -> impl ExactSizeIterator<Item = &'a Frame> + 'a {
        self.0.iter()
    }
}
