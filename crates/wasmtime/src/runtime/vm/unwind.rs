//! Support for low-level primitives of unwinding the stack.

use crate::runtime::vm::arch;

/// Implementation necessary to unwind the stack, used by `Backtrace`.
pub unsafe trait Unwind {
    /// Returns the offset, from the current frame pointer, of where to get to
    /// the previous frame pointer on the stack.
    fn next_older_fp_from_fp_offset(&self) -> usize;

    /// Load the return address of a frame given the frame pointer for that
    /// frame.
    unsafe fn get_next_older_pc_from_fp(&self, fp: usize) -> usize;

    /// Debug assertion that the frame pointer is aligned.
    fn assert_fp_is_aligned(&self, fp: usize);
}

/// A host-backed implementation of unwinding, using the native platform ABI
/// that Cranelift has.
pub struct UnwindHost;

unsafe impl Unwind for UnwindHost {
    fn next_older_fp_from_fp_offset(&self) -> usize {
        arch::NEXT_OLDER_FP_FROM_FP_OFFSET
    }
    unsafe fn get_next_older_pc_from_fp(&self, fp: usize) -> usize {
        arch::get_next_older_pc_from_fp(fp)
    }
    fn assert_fp_is_aligned(&self, fp: usize) {
        arch::assert_fp_is_aligned(fp)
    }
}

/// An implementation specifically designed for unwinding Pulley's runtime stack
/// (which might not match the native host).
pub struct UnwindPulley;

unsafe impl Unwind for UnwindPulley {
    fn next_older_fp_from_fp_offset(&self) -> usize {
        0
    }
    unsafe fn get_next_older_pc_from_fp(&self, fp: usize) -> usize {
        // The calling convention always pushes the return pointer (aka the PC
        // of the next older frame) just before this frame.
        *(fp as *mut usize).offset(1)
    }
    fn assert_fp_is_aligned(&self, fp: usize) {
        let expected = if cfg!(target_pointer_width = "32") {
            8
        } else {
            16
        };
        assert_eq!(fp % expected, 0, "stack should always be aligned");
    }
}
