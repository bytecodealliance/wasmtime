//! Backtrace, stack walking, and unwinding functionality for Wasm.

use crate::traphandlers::{tls, CallThreadState};
use cfg_if::cfg_if;
use std::ops::ControlFlow;

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
    /// Capture the current Wasm stack in a backtrace.
    pub fn new() -> Backtrace {
        tls::with(|state| match state {
            Some(state) => unsafe { Self::new_with_state(state, None) },
            None => Backtrace(vec![]),
        })
    }

    pub(crate) unsafe fn new_with_state(
        state: &CallThreadState,
        initial_pc_and_fp: Option<(usize, usize)>,
    ) -> Backtrace {
        let mut frames = vec![];
        Self::trace_with_state(state, initial_pc_and_fp, |frame| {
            frames.push(frame);
            ControlFlow::Continue(())
        });
        Backtrace(frames)
    }

    /// Walk the current Wasm stack, calling `f` for each frame we walk.
    pub fn trace(f: impl FnMut(Frame) -> ControlFlow<()>) {
        tls::with(|state| match state {
            Some(state) => unsafe { Self::trace_with_state(state, None, f) },
            None => {}
        });
    }

    pub(crate) unsafe fn trace_with_state(
        state: &CallThreadState,
        initial_pc_and_fp: Option<(usize, usize)>,
        mut f: impl FnMut(Frame) -> ControlFlow<()>,
    ) {
        let (last_wasm_exit_pc, last_wasm_exit_fp) = match initial_pc_and_fp {
            Some((pc, fp)) => (pc, fp),
            None => (
                *(*state.limits).last_wasm_exit_pc.get(),
                *(*state.limits).last_wasm_exit_fp.get(),
            ),
        };

        assert!(
            (last_wasm_exit_fp != 0 && last_wasm_exit_pc != 0)
                || (last_wasm_exit_fp == 0 && last_wasm_exit_pc == 0)
        );

        if last_wasm_exit_pc == 0 {
            return;
        }

        Self::trace_through_wasm(
            last_wasm_exit_pc,
            last_wasm_exit_fp,
            *(*state.limits).last_wasm_entry_sp.get(),
            &mut f,
        );

        for state in state.iter() {
            assert!(
                (state.old_last_wasm_exit_fp != 0 && state.old_last_wasm_exit_pc != 0)
                    || (state.old_last_wasm_exit_fp == 0 && state.old_last_wasm_exit_pc == 0)
            );

            if state.old_last_wasm_exit_pc == 0 {
                return;
            }

            Self::trace_through_wasm(
                state.old_last_wasm_exit_pc,
                state.old_last_wasm_exit_fp,
                state.old_last_wasm_entry_sp,
                &mut f,
            );
        }
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
            log::trace!("Empty sequence of Wasm frames");
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

        // The stack pointer should always be aligned to 16 bytes
        // *except* inside function prologues where the return PC is
        // pushed to the stack but before the old frame pointer has been
        // saved to the stack via `push rbp`. And this happens to be
        // exactly where we are inside of our host-to-Wasm trampoline
        // that records the value of SP when we first enter
        // Wasm. Therefore, the SP should *always* be 8-byte aligned but
        // *never* 16-byte aligned.
        if cfg!(target_arch = "x86_64") {
            assert_eq!(first_wasm_sp % 8, 0);
            assert_eq!(first_wasm_sp % 16, 8);
        } else if cfg!(target_arch = "aarch64") {
            assert_eq!(first_wasm_sp % 16, 0);
        }

        loop {
            assert_eq!(fp % 16, 0, "stack should always be aligned to 16");

            log::trace!("--- Tracing through one Wasm frame ---");
            log::trace!("pc = 0x{:016x}", pc);
            log::trace!("fp = 0x{:016x}", fp);

            f(Frame { pc, fp })?;

            // If our FP has reached the SP upon entry to Wasm from the
            // host, then we've successfully walked all the Wasm frames,
            // and have now reached a host frame. We're done iterating
            // through this contiguous sequence of Wasm frames.
            if Self::reached_entry_sp(fp, first_wasm_sp) {
                return ControlFlow::Continue(());
            }

            // If we didn't return above, then we know we are still in a
            // Wasm frame, and since Cranelift maintains frame pointers,
            // we know that the FP isn't an arbitrary value and it is
            // safe to dereference it to read the next PC/FP.

            // The calling convention always pushes the return pointer
            // (aka the PC of the next older frame) just before this
            // frame.
            pc = Self::get_next_older_pc_from_fp(fp);

            // And the current frame pointer points to the next older
            // frame pointer. Because the stack grows down, the older FP
            // must be greater than the current FP.
            let next_older_fp = Self::get_next_older_fp_from_fp(fp);
            assert!(next_older_fp > fp, "{next_older_fp:#x} > {fp:#x}");
            fp = next_older_fp;
        }
    }

    unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
        cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                *(fp as *mut usize).offset(1)
            } else if #[cfg(target_arch = "aarch64")] {
                *(fp as *mut usize).offset(1)
            } else {
                compile_error!("platform not supported")
            }
        }
    }

    unsafe fn get_next_older_fp_from_fp(fp: usize) -> usize {
        cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                *(fp as *mut usize)
            } else if #[cfg(target_arch = "aarch64")] {
                *(fp as *mut usize)
            } else {
                compile_error!("platform not supported")
            }
        }
    }

    unsafe fn reached_entry_sp(fp: usize, first_wasm_sp: usize) -> bool {
        cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                fp == first_wasm_sp - 8
            } else if #[cfg(target_arch = "aarch64")] {
                fp == first_wasm_sp - 16
            } else {
                compile_error!("platform not supported")
            }
        }
    }

    /// Iterate over the frames inside this backtrace.
    pub fn frames<'a>(&'a self) -> impl Iterator<Item = &'a Frame> + 'a {
        self.0.iter()
    }
}
