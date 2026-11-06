//! Raw fibers: fibers whose entry point and suspension points are machine code
//! supplied by the caller rather than a Rust closure.
//!
//! See [`RawFiber`] for what raw fibers can do and the contract their code
//! must uphold.

use crate::FiberStack;
use core::sync::atomic::Ordering;
use wasmtime_environ::collections::TryVec;
use wasmtime_environ::error::{Error, Result, bail, ensure};

cfg_select! {
    all(windows, feature = "std", not(miri)) => {
        mod windows;
        use windows as sys;
    }
    _ => {
        mod stackswitch;
        use stackswitch as sys;
    }
}

/// The entry point of a raw fiber.
///
/// This is called on the fiber's stack with the `arg` given to
/// [`RawFiber::new`] and the fiber's [`RawFiber::switch_arg`]. It must never
/// return.
pub type RawFiberEntry = unsafe extern "C" fn(arg: *mut u8, switch_arg: *mut u8);

/// The lifecycle of a [`RawFiber`].
///
/// A raw fiber is only ever observed while stopped; it runs only for the
/// duration of a call to [`RawFiber::resume`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RawFiberState {
    /// The fiber has not yet been resumed; resuming it calls its entry point.
    Initial,
    /// The fiber suspended itself and may be resumed.
    Suspended,
    /// The fiber suspended for the last time, as recorded by
    /// [`RawFiber::finish`]. It has no continuation, so it may not be resumed
    /// or snapshotted unless an earlier snapshot is restored first.
    Terminal,
}

/// A fiber running caller-supplied machine code.
///
/// Raw fibers provide one key capability that ordinary fibers, which
/// wrap a Rust closure, do not: the ability to snapshot and
/// resume. (This capability is useful for Wasmtime features like
/// record/replay with reversible execution.)
///
/// A raw fiber requires an entry point that is an `extern "C"`
/// function with a particular signature (two opaque pointer args),
/// and that *never returns*. Unlike ordinary fibers, this entry-point
/// function is the first and only frame on the fiber stack when it
/// starts executing.
///
/// A raw fiber's state -- stack and SP -- can be snapshotted and
/// restored, potentially multiple times ("multi-shot
/// continuations"). The safety of this is fairly subtle and has the
/// following requirements:
///
/// - No Rust-compiled frame must exist anywhere on the fiber stack
///   (because multiple returns from a function call in Rust are
///   UB). This implies, in practice, either handwritten assembly or
///   machine code that the user otherwise completely controls
///   (e.g. Cranelift-compiled code in Wasmtime).
/// - Any live pointers that may be in registers or on the stack
///   (i.e., live anywhere in the executing code) continue to be live
///   when execution is rewound to an earlier snapshot. In practice,
///   it is safest to have "completely invariant pointers": e.g., VM
///   data structures or memories/heaps that stay at the same address
///   for the lifetime of a fiber.
/// - The raw fiber entry point must never return. Instead, it can implement
///   termination by yielding one final time, having set some state so that
///   the rest of the runtime will then know to drop the raw fiber (thus freeing
///   its stack).
pub struct RawFiber {
    stack: FiberStack,
    state: RawFiberState,
    id: RawFiberId,
    sys: sys::Fiber,
}

/// A copy of a stopped [`RawFiber`]'s stack and lifecycle state.
pub struct RawFiberSnapshot {
    fiber: RawFiberId,
    sp: usize,
    bytes: TryVec<u8>,
    state: RawFiberState,
}

/// A process-unique identifier of a [`RawFiber`], by which snapshots are
/// matched to the fiber they were taken from.
///
/// IDs must never be reused, since a reused ID would let a snapshot be
/// restored onto a different fiber. Where 64-bit atomics are available, IDs
/// are 64 bits wide, so they never run out in practice; elsewhere, allocating
/// an ID panics once a 32-bit counter is exhausted rather than wrapping.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RawFiberId(u64);

impl RawFiberId {
    fn new() -> Self {
        cfg_select! {
            target_has_atomic = "64" => {
                use core::sync::atomic::AtomicU64;
                static NEXT_ID: AtomicU64 = AtomicU64::new(0);
                RawFiberId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
            }
            _ => {
                use core::sync::atomic::AtomicU32;
                static NEXT_ID: AtomicU32 = AtomicU32::new(0);
                let mut id = NEXT_ID.load(Ordering::Relaxed);
                loop {
                    let next = id.checked_add(1).expect("raw fiber IDs exhausted");
                    match NEXT_ID.compare_exchange_weak(
                        id,
                        next,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break RawFiberId(u64::from(id)),
                        Err(actual) => id = actual,
                    }
                }
            }
        }
    }
}

impl RawFiber {
    /// Whether raw fibers are supported on this platform and configuration.
    ///
    /// Raw fibers require one of the stack-switching routines in this crate
    /// (not an embedder's `custom` one), or Windows fibers on x86_64 or
    /// aarch64. They are not supported under Miri or with AddressSanitizer,
    /// whose fiber-switch handshakes would require Rust code at every
    /// suspension point, nor with hardware-enforced shadow stacks, which
    /// snapshots do not restore.
    pub fn is_supported() -> bool {
        sys::supported() && !cfg!(asan)
    }

    /// Creates a new raw fiber that calls `entry(arg, switch_arg)` when first
    /// resumed.
    ///
    /// On error the stack is handed back to the caller.
    ///
    /// # Safety
    ///
    /// `entry` must uphold the contract of [`RawFiberEntry`] and suspend only
    /// through [`RawFiber::switch_routine`], passing [`RawFiber::switch_arg`].
    /// `arg` must remain valid for as long as the fiber may run.
    pub unsafe fn new(
        mut stack: FiberStack,
        entry: RawFiberEntry,
        arg: *mut u8,
    ) -> Result<Self, (Error, FiberStack)> {
        if !Self::is_supported() {
            return Err((
                wasmtime_environ::error::format_err!(
                    "raw fibers are not supported on this platform"
                ),
                stack,
            ));
        }
        // SAFETY: the stack is owned by this fiber, and `entry` and `arg`
        // are valid per this function's contract.
        let sys = match unsafe { sys::Fiber::new(&mut stack, entry, arg) } {
            Ok(sys) => sys,
            Err(e) => return Err((e, stack)),
        };
        Ok(RawFiber {
            stack,
            state: RawFiberState::Initial,
            id: RawFiberId::new(),
            sys,
        })
    }

    /// The assembly routine which code on a raw fiber calls, with the
    /// C calling convention and [`RawFiber::switch_arg`] as its only
    /// argument, to suspend itself. It returns when the fiber is next
    /// resumed.
    pub fn switch_routine() -> unsafe extern "C" fn(*mut u8) {
        sys::switch_routine()
    }

    /// The argument which code on this fiber passes to
    /// [`RawFiber::switch_routine`]. This is the top of the fiber's stack as
    /// reported by [`FiberStack::top`].
    pub fn switch_arg(&self) -> *mut u8 {
        self.stack.top().unwrap()
    }

    /// The current lifecycle state of this fiber.
    pub fn state(&self) -> RawFiberState {
        self.state
    }

    /// The stack this fiber runs on.
    pub fn stack(&self) -> &FiberStack {
        &self.stack
    }

    /// Destroys this fiber, returning its stack. Nothing on the fiber's stack
    /// is run or unwound.
    pub fn into_stack(self) -> FiberStack {
        let mut stack = self.stack;
        self.sys.destroy(&mut stack);
        stack
    }

    /// Runs this fiber until it next suspends itself.
    ///
    /// Returns an error, without running anything, if the fiber is terminal.
    ///
    /// # Safety
    ///
    /// Everything the fiber's code requires must be valid for the duration of
    /// the call. In particular after a [`RawFiber::restore`], every pointer the
    /// restored continuation can dereference must refer to live storage.
    pub unsafe fn resume(&mut self) -> Result<()> {
        ensure!(
            self.state != RawFiberState::Terminal,
            "cannot resume a terminal raw fiber"
        );
        let top = self.switch_arg();
        // SAFETY: the fiber is stopped and everything it requires is valid
        // per this function's contract.
        unsafe { self.sys.resume(top) };
        self.state = RawFiberState::Suspended;
        Ok(())
    }

    /// Records that this fiber's most recent suspension was its last one.
    ///
    /// The fiber's owner learns this through its own protocol with the code
    /// running on the fiber.
    pub fn finish(&mut self) -> Result<()> {
        ensure!(
            self.state == RawFiberState::Suspended,
            "only a suspended raw fiber can finish"
        );
        self.state = RawFiberState::Terminal;
        Ok(())
    }

    fn saved_sp_slot(&self) -> *mut usize {
        // See the layout diagrams in `unix.rs` and `raw/windows.rs`: the
        // second slot below the top holds the stack pointer to resume the
        // fiber at.
        self.switch_arg().cast::<usize>().wrapping_sub(2)
    }

    /// The stack pointer at which this fiber will resume.
    ///
    /// # Panics
    ///
    /// Panics if the fiber is terminal: it will not resume, so it has no
    /// continuation to inspect.
    fn saved_sp(&self) -> usize {
        assert_ne!(
            self.state,
            RawFiberState::Terminal,
            "a terminal raw fiber has no saved stack pointer"
        );
        // SAFETY: the reserved slot is initialized when the fiber is created
        // and maintained by the switch routine.
        unsafe { self.saved_sp_slot().read() }
    }

    /// Copies this stopped fiber's stack and lifecycle state.
    ///
    /// Returns an error if the fiber is terminal: it has no continuation to
    /// copy.
    pub fn snapshot(&self) -> Result<RawFiberSnapshot> {
        ensure!(
            self.state != RawFiberState::Terminal,
            "cannot snapshot a terminal raw fiber"
        );
        let top = self.switch_arg() as usize;
        let sp = self.saved_sp();
        let range = self.stack.range();
        ensure!(
            range.is_none_or(|r| r.start <= sp) && sp < top,
            "raw fiber has an invalid saved stack pointer"
        );
        let len = top - sp;
        let mut bytes = TryVec::new();
        // SAFETY: `sp..top` is within this fiber's stack, which is not
        // running because `self` is borrowed here.
        bytes.extend_from_slice(unsafe { core::slice::from_raw_parts(sp as *const u8, len) })?;
        Ok(RawFiberSnapshot {
            fiber: self.id,
            sp,
            bytes,
            state: self.state,
        })
    }

    /// Restores a snapshot previously taken from this fiber, replacing its
    /// current continuation and lifecycle state. The next [`RawFiber::resume`]
    /// continues from the snapshot's suspension point (or the entry point, for
    /// a snapshot of an initial fiber), returning to that resume's caller.
    ///
    /// Snapshots taken from a different fiber are rejected without modifying
    /// this fiber.
    ///
    /// # Safety
    ///
    /// The snapshot must have been taken at a point eligible for restoration,
    /// as described in the [`RawFiber`] documentation.
    pub unsafe fn restore(&mut self, snapshot: &RawFiberSnapshot) -> Result<()> {
        if snapshot.fiber != self.id {
            bail!("raw fiber snapshot belongs to a different fiber");
        }
        // SAFETY: the snapshot came from this fiber's stack, so the range is
        // in bounds. It includes the saved stack pointer slot, which this
        // overwrites with `snapshot.sp`, and the stack is not running.
        unsafe {
            core::ptr::copy_nonoverlapping(
                snapshot.bytes.as_ptr(),
                snapshot.sp as *mut u8,
                snapshot.bytes.len(),
            );
        }
        self.state = snapshot.state;
        debug_assert_eq!(self.saved_sp(), snapshot.sp);
        Ok(())
    }

    /// The return address and frame pointer of the code that called the
    /// switch routine, for walking a suspended fiber's frames.
    ///
    /// Returns `None` if the fiber has not suspended or if this is not
    /// implemented for the current architecture.
    ///
    /// # Panics
    ///
    /// Panics if the fiber is terminal: it will not resume, so it has no
    /// continuation to inspect.
    pub fn suspended_frame(&self) -> Option<(usize, usize)> {
        assert_ne!(
            self.state,
            RawFiberState::Terminal,
            "a terminal raw fiber has no suspended frame"
        );
        let (fp, pc) = sys::SUSPENDED_FRAME?;
        if self.state == RawFiberState::Initial {
            return None;
        }
        let sp = self.saved_sp() as *const usize;
        // SAFETY: a suspended fiber's saved stack pointer addresses the
        // registers spilled by the switch routine.
        unsafe { Some((sp.add(pc).read(), sp.add(fp).read())) }
    }
}

impl RawFiberSnapshot {
    /// The lifecycle state the fiber will have once this is restored.
    pub fn state(&self) -> RawFiberState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    /// State shared between a test and the code on its raw fiber.
    #[derive(Default)]
    struct Shared {
        observed: Cell<u64>,
        finished: Cell<bool>,
    }

    /// Yields the values 1, 2, 3 and then finishes. The counter is local to
    /// the fiber, so a restored snapshot rewinds it.
    unsafe extern "C" fn count_to_three(arg: *mut u8, switch_arg: *mut u8) {
        // SAFETY: tests pass a `Shared` that outlives the fiber's use.
        let shared = unsafe { &*arg.cast::<Shared>() };
        let switch = RawFiber::switch_routine();
        let mut count = 0;
        while count < 3 {
            count += 1;
            shared.observed.set(count);
            // SAFETY: this is the switch routine for this fiber.
            unsafe { switch(switch_arg) };
        }
        shared.finished.set(true);
        // SAFETY: as above.
        unsafe { switch(switch_arg) };
        // Resuming a terminal fiber is rejected by `RawFiber::resume`.
        std::process::abort();
    }

    fn fiber(shared: &Shared) -> Option<RawFiber> {
        if !RawFiber::is_supported() {
            return None;
        }
        let stack = FiberStack::new(64 * 1024, false).unwrap();
        let arg = core::ptr::from_ref(shared).cast_mut().cast();
        Some(
            unsafe { RawFiber::new(stack, count_to_three, arg) }
                .map_err(|e| e.0)
                .unwrap(),
        )
    }

    /// Resumes `fiber` from a host stack frame `depth` calls deeper than the
    /// caller, so that each resume has a different host continuation.
    #[inline(never)]
    fn resume_at_depth(fiber: &mut RawFiber, depth: usize) {
        if depth == 0 {
            unsafe { fiber.resume().unwrap() };
        } else {
            resume_at_depth(fiber, depth - 1);
            core::hint::black_box(depth);
        }
    }

    #[test]
    fn lifecycle() {
        let shared = Shared::default();
        let Some(mut fiber) = fiber(&shared) else {
            return;
        };
        assert_eq!(fiber.state(), RawFiberState::Initial);
        for i in 1..=3 {
            resume_at_depth(&mut fiber, i as usize);
            assert_eq!(shared.observed.get(), i);
            assert_eq!(fiber.state(), RawFiberState::Suspended);
        }
        unsafe { fiber.resume().unwrap() };
        assert!(shared.finished.get());
        fiber.finish().unwrap();
        assert_eq!(fiber.state(), RawFiberState::Terminal);
        assert!(unsafe { fiber.resume() }.is_err());
        assert!(fiber.finish().is_err());
        assert!(fiber.snapshot().is_err());
        // A terminal fiber is destroyed without being resumed.
        drop(fiber.into_stack());
    }

    #[test]
    fn rewind() {
        let shared = Shared::default();
        let Some(mut fiber) = fiber(&shared) else {
            return;
        };
        let initial = fiber.snapshot().unwrap();
        unsafe { fiber.resume().unwrap() };
        let first = fiber.snapshot().unwrap();
        assert_eq!(first.state(), RawFiberState::Suspended);

        // Repeatedly rewind to the first yield, resuming from different host
        // frames each time.
        for depth in 0..3 {
            unsafe { fiber.resume().unwrap() };
            assert_eq!(shared.observed.get(), 2);
            unsafe { fiber.restore(&first).unwrap() };
            resume_at_depth(&mut fiber, depth);
            assert_eq!(shared.observed.get(), 2);
            unsafe { fiber.restore(&first).unwrap() };
        }

        // Run to completion, then rewind after the final yield.
        for _ in 0..3 {
            unsafe { fiber.resume().unwrap() };
        }
        assert!(shared.finished.get());
        fiber.finish().unwrap();
        unsafe { fiber.restore(&first).unwrap() };
        assert_eq!(fiber.state(), RawFiberState::Suspended);
        unsafe { fiber.resume().unwrap() };
        assert_eq!(shared.observed.get(), 2);

        // Restart from the very beginning.
        unsafe { fiber.restore(&initial).unwrap() };
        assert_eq!(fiber.state(), RawFiberState::Initial);
        unsafe { fiber.resume().unwrap() };
        assert_eq!(shared.observed.get(), 1);
    }

    #[test]
    fn snapshots_belong_to_one_fiber() {
        let a = Shared::default();
        let b = Shared::default();
        let (Some(mut fa), Some(mut fb)) = (fiber(&a), fiber(&b)) else {
            return;
        };
        unsafe { fa.resume().unwrap() };
        let snapshot = fa.snapshot().unwrap();
        assert!(unsafe { fb.restore(&snapshot) }.is_err());
        assert_eq!(fb.state(), RawFiberState::Initial);

        // Destroying a fiber leaves its outstanding snapshots inert, even if a
        // new fiber reuses the same stack memory.
        unsafe { fa.resume().unwrap() };
        let stack = fa.into_stack();
        let arg = core::ptr::from_ref(&a).cast_mut().cast();
        let mut fc = unsafe { RawFiber::new(stack, count_to_three, arg) }
            .map_err(|e| e.0)
            .unwrap();
        assert!(unsafe { fc.restore(&snapshot) }.is_err());
        unsafe { fc.resume().unwrap() };
        assert_eq!(a.observed.get(), 1);
        drop(snapshot);
        unsafe { fb.resume().unwrap() };
        assert_eq!(b.observed.get(), 1);
    }

    #[test]
    fn resume_on_another_thread() {
        let shared = std::sync::Arc::new(SharedSync::default());
        if !RawFiber::is_supported() {
            return;
        }
        unsafe extern "C" fn entry(arg: *mut u8, switch_arg: *mut u8) {
            let shared = unsafe { &*arg.cast::<SharedSync>() };
            let switch = RawFiber::switch_routine();
            loop {
                shared.0.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                unsafe { switch(switch_arg) };
            }
        }
        #[derive(Default)]
        struct SharedSync(core::sync::atomic::AtomicUsize);

        let stack = FiberStack::new(64 * 1024, false).unwrap();
        let arg = std::sync::Arc::as_ptr(&shared).cast_mut().cast();
        let mut fiber = unsafe { RawFiber::new(stack, entry, arg) }
            .map_err(|e| e.0)
            .unwrap();
        unsafe { fiber.resume().unwrap() };
        let snapshot = fiber.snapshot().unwrap();
        let mut fiber = std::thread::spawn(move || {
            unsafe { fiber.resume().unwrap() };
            fiber
        })
        .join()
        .unwrap();
        unsafe { fiber.restore(&snapshot).unwrap() };
        unsafe { fiber.resume().unwrap() };
        assert_eq!(shared.0.load(core::sync::atomic::Ordering::Relaxed), 3);
    }

    #[test]
    fn terminal_fiber_has_no_suspended_frame() {
        let shared = Shared::default();
        let Some(mut fiber) = fiber(&shared) else {
            return;
        };
        for _ in 0..4 {
            unsafe { fiber.resume().unwrap() };
        }
        fiber.finish().unwrap();
        let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            fiber.suspended_frame();
        }));
        assert!(result.is_err());
    }

    #[test]
    fn suspended_frame_points_into_entry() {
        let shared = Shared::default();
        let Some(mut fiber) = fiber(&shared) else {
            return;
        };
        assert!(fiber.suspended_frame().is_none());
        unsafe { fiber.resume().unwrap() };
        if let Some((pc, fp)) = fiber.suspended_frame() {
            // Without frame pointers in this Rust entry, `fp` may still be
            // the initial one: the top of the stack.
            let range = fiber.stack().range().unwrap();
            assert!(
                range.start <= fp && fp <= range.end,
                "{fp:#x} vs {range:x?}"
            );
            let entry = count_to_three as *const () as usize;
            assert!(pc > entry && pc < entry + 4096, "{pc:#x} vs {entry:#x}");
        }
    }
}
