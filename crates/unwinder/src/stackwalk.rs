//! Stack-walking of a Wasm stack.
//!
//! A stack walk requires a first and last frame pointer (FP), and it
//! only works on code that has been compiled with frame pointers
//! enabled (`preserve_frame_pointers` Cranelift option enabled). The
//! stack walk follows the singly-linked list of saved frame pointer
//! and return address pairs on the stack that is naturally built by
//! function prologues.
//!
//! This crate makes use of the fact that Wasmtime surrounds Wasm
//! frames by trampolines both at entry and exit, and is "up the
//! stack" from the point doing the unwinding: in other words, host
//! code invokes Wasm code via an entry trampoline, that code may call
//! other Wasm code, and ultimately it calls back to host code via an
//! exit trampoline. That exit trampoline is able to provide the
//! "start FP" (FP at exit trampoline) and "end FP" (FP at entry
//! trampoline) and this stack-walker can visit all Wasm frames
//! active on the stack between those two.
//!
//! This module provides a visitor interface to frames, but is
//! agnostic to the desired use-case or consumer of the frames, and to
//! the overall runtime structure.

use core::ops::ControlFlow;

/// Implementation necessary to unwind the stack, used by `Backtrace`.
///
/// # Safety
///
/// This trait is `unsafe` because the return values of each function are
/// required to be semantically correct when connected to the `visit_frames`
/// function below. Incorrect and/or arbitrary values in this trait will cause
/// unwinding to segfault or otherwise result in UB.
pub unsafe trait Unwind {
    /// Returns the offset, from the current frame pointer, of where to get to
    /// the previous frame pointer on the stack.
    fn next_older_fp_from_fp_offset(&self) -> usize;

    /// Returns the offset, from the current frame pointer, of the
    /// stack pointer of the next older frame.
    fn next_older_sp_from_fp_offset(&self) -> usize;

    /// Load the return address of a frame given the frame pointer for that
    /// frame.
    ///
    /// # Safety
    ///
    /// This function is expected to read raw memory from `fp` and thus is not
    /// safe to operate on any value of `fp` passed in, instead it must be a
    /// trusted Cranelift-defined frame pointer.
    unsafe fn get_next_older_pc_from_fp(&self, fp: usize) -> usize;

    /// Debug assertion that the frame pointer is aligned.
    fn assert_fp_is_aligned(&self, fp: usize);
}

/// A stack frame within a Wasm stack trace.
#[derive(Debug)]
pub struct Frame {
    /// The program counter in this frame. Because every frame in the
    /// stack-walk is paused at a call (as we are in host code called
    /// by Wasm code below these frames), the PC is at the return
    /// address, i.e., points to the instruction after the call
    /// instruction.
    pc: usize,
    /// The frame pointer value corresponding to this frame.
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

    /// Read out a machine-word-sized value at the given offset from
    /// FP in this frame.
    ///
    /// # Safety
    ///
    /// Requires that this frame is a valid, active frame. A `Frame`
    /// provided by `visit_frames()` will be valid for the duration of
    /// the invoked closure.
    ///
    /// Requires that `offset` falls within the size of this
    /// frame. This ordinarily requires knowledge passed from the
    /// compiler that produced the running function, e.g., Cranelift.
    pub unsafe fn read_slot_from_fp(&self, offset: isize) -> usize {
        // SAFETY: we required that this is a valid frame, and that
        // `offset` is a valid offset within that frame.
        unsafe { *(self.fp.wrapping_add_signed(offset) as *mut usize) }
    }
}

/// A cursor over `Frame`s in a single activation of Wasm.
#[derive(Clone)]
pub struct FrameCursor {
    pc: usize,
    fp: usize,
    trampoline_fp: usize,
}

impl FrameCursor {
    /// Provide a cursor that walks through a contiguous sequence of Wasm
    /// frames starting with the frame at the given PC and FP and ending
    /// at `trampoline_fp`. This FP should correspond to that of a
    /// trampoline that was used to enter the Wasm code.
    ///
    /// We require that the initial PC, FP, and `trampoline_fp` values are
    /// non-null (non-zero).
    ///
    /// The returned type is a cursor, not a literal `Iterator`
    /// implementation, because we do not want to capture the `&dyn
    /// Unwind` (rather the cursor's `next` method requires the `&dyn
    /// Unwind` separately, which permits more flexible usage).
    ///
    /// # Safety
    ///
    /// This function is not safe as `unwind`, `pc`, `fp`, and `trampoline_fp` must
    /// all be "correct" in that if they're wrong or mistakenly have the wrong value
    /// then this method may segfault. These values must point to valid Wasmtime
    /// compiled code which respects the frame pointers that Wasmtime currently
    /// requires.
    ///
    /// The iterator that this function returns *must* be consumed while
    /// the frames are still active. That is, it cannot be stashed and
    /// consumed after returning back into the Wasm activation that is
    /// being iterated over.
    ///
    /// Ordinarily this can be ensured by holding the unsafe iterator
    /// together with a borrow of the `Store` that owns the stack;
    /// higher-level layers wrap the two together.
    pub unsafe fn new(pc: usize, fp: usize, trampoline_fp: usize) -> FrameCursor {
        log::trace!("=== Tracing through contiguous sequence of Wasm frames ===");
        log::trace!("trampoline_fp = 0x{trampoline_fp:016x}");
        log::trace!("   initial pc = 0x{pc:016x}");
        log::trace!("   initial fp = 0x{fp:016x}");

        // Safety requirements documented above.
        assert_ne!(pc, 0);
        assert_ne!(fp, 0);
        assert_ne!(trampoline_fp, 0);

        FrameCursor {
            pc,
            fp,
            trampoline_fp,
        }
    }

    /// Get the frame that this cursor currently points at.
    pub fn frame(&self) -> Frame {
        assert!(!self.done());
        Frame {
            pc: self.pc,
            fp: self.fp,
        }
    }

    /// Returns whether the cursor is "done", i.e., no other frame is
    /// available in this activation.
    pub fn done(&self) -> bool {
        self.fp == self.trampoline_fp
    }

    /// Move to the next frame in this activation, if any.
    ///
    /// # Safety
    ///
    /// The `unwind` passed in must correspond to the host
    /// implementation from which this stack came.
    pub unsafe fn advance(&mut self, unwind: &dyn Unwind) {
        // This logic will walk the linked list of frame pointers starting
        // at `fp` and going up until `trampoline_fp`. We know that both
        // `fp` and `trampoline_fp` are "trusted values" aka generated and
        // maintained by Wasmtime. This means that it should be safe to
        // walk the linked list of pointers and inspect Wasm frames.
        //
        // Note, though, that any frames outside of this range are not
        // guaranteed to have valid frame pointers. For example native code
        // might be using the frame pointer as a general purpose register. Thus
        // we need to be careful to only walk frame pointers in this one
        // contiguous linked list.
        //
        // To know when to stop iteration all architectures' stacks currently
        // look something like this:
        //
        //     | ...               |
        //     | Native Frames     |
        //     | ...               |
        //     |-------------------|
        //     | ...               | <-- Trampoline FP            |
        //     | Trampoline Frame  |                              |
        //     | ...               | <-- Trampoline SP            |
        //     |-------------------|                            Stack
        //     | Return Address    |                            Grows
        //     | Previous FP       | <-- Wasm FP                Down
        //     | ...               |                              |
        //     | Cranelift Frames  |                              |
        //     | ...               |                              V
        //
        // The trampoline records its own frame pointer (`trampoline_fp`),
        // which is guaranteed to be above all Wasm code. To check when

        // to check when the next frame pointer is equal to
        // `trampoline_fp`. Once that's hit then we know that the entire
        // linked list has been traversed.
        //
        // Note that it might be possible that this loop doesn't execute
        // at all.  For example if the entry trampoline called Wasm code
        // which `return_call`'d an exit trampoline, then `fp ==
        // trampoline_fp` on the entry of this function, meaning the loop
        // won't actually execute anything.
        if self.fp == self.trampoline_fp {
            log::trace!("=== Done tracing contiguous sequence of Wasm frames ===");
            return;
        }

        // At the start of each iteration of the loop, we know that
        // `fp` is a frame pointer from Wasm code. Therefore, we know
        // it is not being used as an extra general-purpose register,
        // and it is safe dereference to get the PC and the next older
        // frame pointer.
        //
        // The stack also grows down, and therefore any frame pointer
        // we are dealing with should be less than the frame pointer
        // on entry to Wasm code. Finally also assert that it's
        // aligned correctly as an additional sanity check.
        assert!(
            self.trampoline_fp > self.fp,
            "{:#x} > {:#x}",
            self.trampoline_fp,
            self.fp
        );
        unwind.assert_fp_is_aligned(self.fp);

        log::trace!("--- Tracing through one Wasm frame ---");
        log::trace!("pc = {:p}", self.pc as *const ());
        log::trace!("fp = {:p}", self.fp as *const ());

        // SAFETY: this unsafe traversal of the linked list on the stack is
        // reflected in the contract of this function where `pc`, `fp`,
        // `trampoline_fp`, and `unwind` must all be trusted/correct values.
        unsafe {
            self.pc = unwind.get_next_older_pc_from_fp(self.fp);

            // We rely on this offset being zero for all supported
            // architectures in
            // `crates/cranelift/src/component/compiler.s`r when we set
            // the Wasm exit FP. If this ever changes, we will need to
            // update that code as well!
            assert_eq!(unwind.next_older_fp_from_fp_offset(), 0);

            // Get the next older frame pointer from the current Wasm
            // frame pointer.
            let next_older_fp = *(self.fp as *mut usize).add(unwind.next_older_fp_from_fp_offset());

            // Because the stack always grows down, the older FP must be greater
            // than the current FP.
            assert!(
                next_older_fp > self.fp,
                "{next_older_fp:#x} > {fp:#x}",
                fp = self.fp
            );
            self.fp = next_older_fp;
        }
    }
}

/// Wrap `FrameCursor` in a true iterator.
///
/// This captures the `unwind` borrow for the duration of the
/// iterator's lifetime.
///
/// # Safety
///
/// Safety conditions for this function are the same as for the
/// [`FrameCursor::new`] function, plus the condition on `unwind` from
/// the [`FrameCursor::advance`] method.
pub unsafe fn frame_iterator(
    unwind: &dyn Unwind,
    pc: usize,
    fp: usize,
    trampoline_fp: usize,
) -> impl Iterator<Item = Frame> {
    // SAFETY: our safety conditions include those of
    // `FrameCursor::new`.
    let mut cursor = unsafe { FrameCursor::new(pc, fp, trampoline_fp) };
    core::iter::from_fn(move || {
        if cursor.done() {
            None
        } else {
            let frame = cursor.frame();
            // SAFETY: `unwind` is associated with the stack as per
            // our safety conditions.
            unsafe {
                cursor.advance(unwind);
            }
            Some(frame)
        }
    })
}

/// Walk through a contiguous sequence of Wasm frames starting with
/// the frame at the given PC and FP and ending at
/// `trampoline_fp`. This FP should correspond to that of a trampoline
/// that was used to enter the Wasm code.
///
/// We require that the initial PC, FP, and `trampoline_fp` values are
/// non-null (non-zero).
///
/// # Safety
///
/// This function is not safe as `unwind`, `pc`, `fp`, and `trampoline_fp` must
/// all be "correct" in that if they're wrong or mistakenly have the wrong value
/// then this method may segfault. These values must point to valid Wasmtime
/// compiled code which respects the frame pointers that Wasmtime currently
/// requires.
pub unsafe fn visit_frames<R>(
    unwind: &dyn Unwind,
    pc: usize,
    fp: usize,
    trampoline_fp: usize,
    mut f: impl FnMut(Frame) -> ControlFlow<R>,
) -> ControlFlow<R> {
    let iter = unsafe { frame_iterator(unwind, pc, fp, trampoline_fp) };
    for frame in iter {
        f(frame)?;
    }

    ControlFlow::Continue(())
}
