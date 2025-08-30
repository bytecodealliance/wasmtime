//! Throw action computation (handler search).
//!
//! In order to throw exceptions from within Cranelift-compiled code,
//! we provide a runtime function helper meant to be called by host
//! code that is invoked by guest code.
//!
//! The helper below must be provided a delimited range on the stack
//! corresponding to Cranelift frames above the current host code. It
//! will look for any handlers in this code, given a closure that
//! knows how to use an absolute PC to look up a module's exception
//! table and its start-of-code-segment. If a handler is found, the
//! helper below will return the SP, FP and PC that must be
//! restored. Architecture-specific helpers are provided to jump to
//! this new context with payload values. Otherwise, if no handler is
//! found, the return type indicates this, and it is the caller's
//! responsibility to invoke alternative behavior (e.g., abort the
//! program or unwind all the way to initial Cranelift-code entry).

use crate::{Frame, Unwind};
use core::ops::ControlFlow;

/// Description of a frame on the stack that is ready to catch an exception.
#[derive(Debug)]
pub struct Handler {
    /// Program counter of handler return point.
    pub pc: usize,
    /// Stack pointer to restore before jumping to handler.
    pub sp: usize,
    /// Frame pointer to restore before jumping to handler.
    pub fp: usize,
}

impl Handler {
    /// Implementation of stack-walking to find a handler.
    ///
    /// This function searches for a handler in the given range of stack
    /// frames, starting from the throw stub and up to a specified entry
    /// frame.
    ///
    /// # Safety
    ///
    /// The safety of this function is the same as [`crate::visit_frames`] where the
    /// values passed in configuring the frame pointer walk must be correct and
    /// Wasm-defined for this to not have UB.
    pub unsafe fn find<F: Fn(&Frame) -> Option<(usize, usize)>>(
        unwind: &dyn Unwind,
        frame_handler: F,
        exit_pc: usize,
        exit_trampoline_frame: usize,
        entry_frame: usize,
    ) -> Option<Handler> {
        // SAFETY: the safety of `visit_frames` relies on the correctness of the
        // parameters passed in which is forwarded as a contract to this function
        // tiself.
        let result = unsafe {
            crate::stackwalk::visit_frames(
                unwind,
                exit_pc,
                exit_trampoline_frame,
                entry_frame,
                |frame| {
                    log::trace!("visit_frame: frame {frame:?}");
                    if let Some((handler_pc, handler_sp)) = frame_handler(&frame) {
                        return ControlFlow::Break(Handler {
                            pc: handler_pc,
                            sp: handler_sp,
                            fp: frame.fp(),
                        });
                    }
                    ControlFlow::Continue(())
                },
            )
        };
        match result {
            ControlFlow::Break(handler) => Some(handler),
            ControlFlow::Continue(()) => None,
        }
    }
}
