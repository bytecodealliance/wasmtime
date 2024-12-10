//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

mod backtrace;

#[cfg(feature = "coredump")]
#[path = "traphandlers/coredump_enabled.rs"]
mod coredump;
#[cfg(not(feature = "coredump"))]
#[path = "traphandlers/coredump_disabled.rs"]
mod coredump;

#[cfg(all(feature = "signals-based-traps", not(miri)))]
mod signals;
#[cfg(all(feature = "signals-based-traps", not(miri)))]
pub use self::signals::*;

use crate::prelude::*;
use crate::runtime::module::lookup_code;
use crate::runtime::store::StoreOpaque;
use crate::runtime::vm::sys::traphandlers;
use crate::runtime::vm::{Instance, InterpreterRef, VMContext, VMRuntimeLimits};
use crate::{StoreContextMut, WasmBacktrace};
use core::cell::Cell;
use core::ops::Range;
use core::ptr::{self, NonNull};

pub use self::backtrace::Backtrace;
pub use self::coredump::CoreDumpStack;
pub use self::tls::{tls_eager_initialize, AsyncWasmCallState, PreviousAsyncWasmCallState};

pub use traphandlers::SignalHandler;

pub(crate) struct TrapRegisters {
    pub pc: usize,
    pub fp: usize,
}

/// Return value from `test_if_trap`.
pub(crate) enum TrapTest {
    /// Not a wasm trap, need to delegate to whatever process handler is next.
    NotWasm,
    /// This trap was handled by the embedder via custom embedding APIs.
    #[cfg_attr(miri, expect(dead_code, reason = "using #[cfg] too unergonomic"))]
    HandledByEmbedder,
    /// This is a wasm trap, it needs to be handled.
    #[cfg_attr(miri, expect(dead_code, reason = "using #[cfg] too unergonomic"))]
    Trap {
        /// How to longjmp back to the original wasm frame.
        jmp_buf: *const u8,
    },
}

fn lazy_per_thread_init() {
    traphandlers::lazy_per_thread_init();
}

/// Raises a preexisting trap and unwinds.
///
/// This function will execute the `longjmp` to make its way back to the
/// original `setjmp` performed when wasm was entered. This is currently
/// only called from the `raise` builtin of Wasmtime. This builtin is only used
/// when the host returns back to wasm and indicates that a trap should be
/// raised. In this situation the host has already stored trap information
/// within the `CallThreadState` and this is the low-level operation to actually
/// perform an unwind.
///
/// This function won't be use with Pulley, for example, as the interpreter
/// halts differently than native code. Additionally one day this will ideally
/// be implemented by Cranelift itself without need of a libcall when Cranelift
/// implements the exception handling proposal for example.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub(super) unsafe fn raise_preexisting_trap() -> ! {
    tls::with(|info| info.unwrap().unwind())
}

/// Invokes the closure `f` and returns a `bool` if it succeeded.
///
/// This will invoke the closure `f` which returns a value that implements
/// `HostResult`. This trait abstracts over how host values are translated to
/// ABI values when going back into wasm. Some examples are:
///
/// * `T` - bare return types (not results) are simply returned as-is. No
///   `catch_unwind` happens as if a trap can't happen then the host shouldn't
///   be panicking or invoking user code.
///
/// * `Result<(), E>` - this represents an ABI return value of `bool` which
///   indicates whether the call succeeded. This return value will catch panics
///   and record trap information as `E`.
///
/// * `Result<u32, E>` - the ABI return value here is `u64` where on success
///   the 32-bit result is zero-extended and `u64::MAX` as a return value
///   indicates that a trap or panic happened.
///
/// This is primarily used in conjunction with the Cranelift-and-host boundary.
/// This function acts as a bridge between the two to appropriately handle
/// encoding host values to Cranelift-understood ABIs via the `HostResult`
/// trait.
pub fn catch_unwind_and_record_trap<R>(f: impl FnOnce() -> R) -> R::Abi
where
    R: HostResult,
{
    // Invoke the closure `f`, optionally catching unwinds depending on `R`. The
    // return value is always provided and if unwind information is provided
    // (e.g. `ret` is a "false"-y value) then it's recorded in TLS for the
    // unwind operation that's about to happen from Cranelift-generated code.
    let (ret, unwind) = R::maybe_catch_unwind(f);
    if let Some(unwind) = unwind {
        tls::with(|info| info.unwrap().record_unwind(unwind));
    }
    ret
}

/// A trait used in conjunction with `catch_unwind_and_record_trap` to convert a
/// Rust-based type to a specific ABI while handling traps/unwinds.
///
/// This type is implemented for return values from host function calls and
/// libcalls. The `Abi` value of this trait represents either a successful
/// execution with some payload state or that a failed execution happened. In
/// the event of a failed execution the state of the failure itself is stored
/// within `CallThreadState::unwind`. Cranelift-compiled code is expected to
/// test for this failure sentinel and process it accordingly.
///
/// See `catch_unwind_and_record_trap` for some more information as well.
pub trait HostResult {
    /// The type of the value that's returned to Cranelift-compiled code. Needs
    /// to be ABI-safe to pass through an `extern "C"` return value.
    type Abi: Copy;

    /// Executes `f` and returns the ABI/unwind information as a result.
    ///
    /// This may optionally catch unwinds during execution depending on this
    /// implementation. The ABI return value is unconditionally provided. If an
    /// unwind was detected (e.g. a host panic or a wasm trap) then that's
    /// additionally returned as well.
    ///
    /// If an unwind is returned then it's expected that when the host returns
    /// back to wasm (which should be soon after calling this through
    /// `catch_unwind_and_record_trap`) then wasm will very quickly turn around
    /// and initiate an unwind (currently through `raise_preexisting_trap`).
    fn maybe_catch_unwind(f: impl FnOnce() -> Self) -> (Self::Abi, Option<UnwindReason>);
}

// Base case implementations that do not catch unwinds. These are for libcalls
// that neither trap nor execute user code. The raw value is the ABI itself.
//
// Panics in these libcalls will result in a process abort as unwinding is not
// allowed via Rust through `extern "C"` function boundaries.
macro_rules! host_result_no_catch {
    ($($t:ty,)*) => {
        $(
            impl HostResult for $t {
                type Abi = $t;
                fn maybe_catch_unwind(f: impl FnOnce() -> $t) -> ($t, Option<UnwindReason>) {
                    (f(), None)
                }
            }
        )*
    }
}

host_result_no_catch! {
    (),
    bool,
    u32,
    *mut u8,
    u64,
}

impl HostResult for NonNull<u8> {
    type Abi = *mut u8;
    fn maybe_catch_unwind(f: impl FnOnce() -> Self) -> (*mut u8, Option<UnwindReason>) {
        (f().as_ptr(), None)
    }
}

/// Implementation of `HostResult` for `Result<T, E>`.
///
/// This is where things get interesting for `HostResult`. This is generically
/// defined to allow many shapes of the `Result` type to be returned from host
/// calls or libcalls. To do this an extra trait requirement is placed on the
/// successful result `T`: `HostResultHasUnwindSentinel`.
///
/// The general requirement is that `T` says what ABI it has, and the ABI must
/// have a sentinel value which indicates that an unwind in wasm should happen.
/// For example if `T = ()` then `true` means that the call succeeded and
/// `false` means that an unwind happened. Here the sentinel is `false` and the
/// ABI is `bool`.
///
/// This is the only implementation of `HostResult` which actually catches
/// unwinds as there's a sentinel to encode.
impl<T, E> HostResult for Result<T, E>
where
    T: HostResultHasUnwindSentinel,
    E: Into<TrapReason>,
{
    type Abi = T::Abi;

    fn maybe_catch_unwind(f: impl FnOnce() -> Result<T, E>) -> (T::Abi, Option<UnwindReason>) {
        // First prepare the closure `f` as something that'll be invoked to
        // generate the return value of this function. This is the
        // conditionally, below, passed to `catch_unwind`.
        let f = move || match f() {
            Ok(ret) => (ret.into_abi(), None),
            Err(reason) => (T::SENTINEL, Some(UnwindReason::Trap(reason.into()))),
        };

        // With `panic=unwind` use `std::panic::catch_unwind` to catch possible
        // panics to rethrow.
        #[cfg(all(feature = "std", panic = "unwind"))]
        {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                Ok(result) => result,
                Err(err) => (T::SENTINEL, Some(UnwindReason::Panic(err))),
            }
        }

        // With `panic=abort` there's no use in using `std::panic::catch_unwind`
        // since it won't actually catch anything. Note that
        // `std::panic::catch_unwind` will technically optimize to this but having
        // this branch avoids using the `std::panic` module entirely.
        #[cfg(not(all(feature = "std", panic = "unwind")))]
        {
            f()
        }
    }
}

/// Trait used in conjunction with `HostResult for Result<T, E>` where this is
/// the trait bound on `T`.
///
/// This is for values in the "ok" position of a `Result` return value. Each
/// value can have a separate ABI from itself (e.g. `type Abi`) and must be
/// convertible to the ABI. Additionally all implementations of this trait have
/// a "sentinel value" which indicates that an unwind happened. This means that
/// no valid instance of `Self` should generate the `SENTINEL` via the
/// `into_abi` function.
pub unsafe trait HostResultHasUnwindSentinel {
    /// The Cranelift-understood ABI of this value (should not be `Self`).
    type Abi: Copy;

    /// A value that indicates that an unwind should happen and is tested for in
    /// Cranelift-generated code.
    const SENTINEL: Self::Abi;

    /// Converts this value into the ABI representation. Should never returned
    /// the `SENTINEL` value.
    fn into_abi(self) -> Self::Abi;
}

/// No return value from the host is represented as a `bool` in the ABI. Here
/// `true` means that execution succeeded while `false` is the sentinel used to
/// indicate an unwind.
unsafe impl HostResultHasUnwindSentinel for () {
    type Abi = bool;
    const SENTINEL: bool = false;
    fn into_abi(self) -> bool {
        true
    }
}

/// A 32-bit return value can be inflated to a 64-bit return value in the ABI.
/// In this manner a successful result is a zero-extended 32-bit value and the
/// failure sentinel is `u64::MAX` or -1 as a signed integer.
unsafe impl HostResultHasUnwindSentinel for u32 {
    type Abi = u64;
    const SENTINEL: u64 = u64::MAX;
    fn into_abi(self) -> u64 {
        self.into()
    }
}

/// If there is not actual successful result (e.g. an empty enum) then the ABI
/// can be `()`, or nothing, because there's no successful result and it's
/// always a failure.
unsafe impl HostResultHasUnwindSentinel for core::convert::Infallible {
    type Abi = ();
    const SENTINEL: () = ();
    fn into_abi(self) {
        match self {}
    }
}

/// Stores trace message with backtrace.
#[derive(Debug)]
pub struct Trap {
    /// Original reason from where this trap originated.
    pub reason: TrapReason,
    /// Wasm backtrace of the trap, if any.
    pub backtrace: Option<Backtrace>,
    /// The Wasm Coredump, if any.
    pub coredumpstack: Option<CoreDumpStack>,
}

/// Enumeration of different methods of raising a trap.
#[derive(Debug)]
pub enum TrapReason {
    /// A user-raised trap through `raise_user_trap`.
    User(Error),

    /// A trap raised from Cranelift-generated code.
    Jit {
        /// The program counter where this trap originated.
        ///
        /// This is later used with side tables from compilation to translate
        /// the trapping address to a trap code.
        pc: usize,

        /// If the trap was a memory-related trap such as SIGSEGV then this
        /// field will contain the address of the inaccessible data.
        ///
        /// Note that wasm loads/stores are not guaranteed to fill in this
        /// information. Dynamically-bounds-checked memories, for example, will
        /// not access an invalid address but may instead load from NULL or may
        /// explicitly jump to a `ud2` instruction. This is only available for
        /// fault-based traps which are one of the main ways, but not the only
        /// way, to run wasm.
        faulting_addr: Option<usize>,

        /// The trap code associated with this trap.
        trap: wasmtime_environ::Trap,
    },

    /// A trap raised from a wasm libcall
    Wasm(wasmtime_environ::Trap),
}

impl From<Error> for TrapReason {
    fn from(err: Error) -> Self {
        TrapReason::User(err)
    }
}

impl From<wasmtime_environ::Trap> for TrapReason {
    fn from(code: wasmtime_environ::Trap) -> Self {
        TrapReason::Wasm(code)
    }
}

/// Catches any wasm traps that happen within the execution of `closure`,
/// returning them as a `Result`.
///
/// # Unsafety
///
/// This function is unsafe because during the execution of `closure` it may be
/// longjmp'd over and none of its destructors on the stack may be run.
pub unsafe fn catch_traps<T, F>(
    store: &mut StoreContextMut<'_, T>,
    mut closure: F,
) -> Result<(), Box<Trap>>
where
    F: FnMut(*mut VMContext, Option<InterpreterRef<'_>>) -> bool,
{
    let caller = store.0.default_caller();
    let result = CallThreadState::new(store.0, caller).with(|cx| match store.0.interpreter() {
        // In interpreted mode directly invoke the host closure since we won't
        // be using host-based `setjmp`/`longjmp` as that's not going to save
        // the context we want.
        Some(r) => {
            cx.jmp_buf
                .set(CallThreadState::JMP_BUF_INTERPRETER_SENTINEL);
            closure(caller, Some(r))
        }

        // In native mode, however, defer to C to do the `setjmp` since Rust
        // doesn't understand `setjmp`.
        //
        // Note that here we pass a function pointer to C to catch longjmp
        // within, here it's `call_closure`, and that passes `None` for the
        // interpreter since this branch is only ever taken if the interpreter
        // isn't present.
        None => traphandlers::wasmtime_setjmp(
            cx.jmp_buf.as_ptr(),
            {
                extern "C" fn call_closure<F>(payload: *mut u8, caller: *mut VMContext) -> bool
                where
                    F: FnMut(*mut VMContext, Option<InterpreterRef<'_>>) -> bool,
                {
                    unsafe { (*(payload as *mut F))(caller, None) }
                }

                call_closure::<F>
            },
            &mut closure as *mut F as *mut u8,
            caller,
        ),
    });

    return match result {
        Ok(x) => Ok(x),
        Err((UnwindReason::Trap(reason), backtrace, coredumpstack)) => Err(Box::new(Trap {
            reason,
            backtrace,
            coredumpstack,
        })),
        #[cfg(all(feature = "std", panic = "unwind"))]
        Err((UnwindReason::Panic(panic), _, _)) => std::panic::resume_unwind(panic),
    };
}

// Module to hide visibility of the `CallThreadState::prev` field and force
// usage of its accessor methods.
mod call_thread_state {
    use super::*;
    use crate::runtime::vm::Unwind;

    /// Temporary state stored on the stack which is registered in the `tls` module
    /// below for calls into wasm.
    pub struct CallThreadState {
        pub(super) unwind: Cell<Option<(UnwindReason, Option<Backtrace>, Option<CoreDumpStack>)>>,
        pub(super) jmp_buf: Cell<*const u8>,
        #[cfg(all(feature = "signals-based-traps", not(miri)))]
        pub(super) signal_handler: Option<*const SignalHandler>,
        pub(super) capture_backtrace: bool,
        #[cfg(feature = "coredump")]
        pub(super) capture_coredump: bool,

        pub(crate) limits: *const VMRuntimeLimits,
        pub(crate) unwinder: &'static dyn Unwind,

        pub(super) prev: Cell<tls::Ptr>,
        #[cfg(all(feature = "signals-based-traps", unix, not(miri)))]
        pub(crate) async_guard_range: Range<*mut u8>,

        // The values of `VMRuntimeLimits::last_wasm_{exit_{pc,fp},entry_sp}`
        // for the *previous* `CallThreadState` for this same store/limits. Our
        // *current* last wasm PC/FP/SP are saved in `self.limits`. We save a
        // copy of the old registers here because the `VMRuntimeLimits`
        // typically doesn't change across nested calls into Wasm (i.e. they are
        // typically calls back into the same store and `self.limits ==
        // self.prev.limits`) and we must to maintain the list of
        // contiguous-Wasm-frames stack regions for backtracing purposes.
        old_last_wasm_exit_fp: Cell<usize>,
        old_last_wasm_exit_pc: Cell<usize>,
        old_last_wasm_entry_fp: Cell<usize>,
    }

    impl Drop for CallThreadState {
        fn drop(&mut self) {
            // Unwind information should not be present as it should have
            // already been processed.
            debug_assert!(self.unwind.replace(None).is_none());

            unsafe {
                *(*self.limits).last_wasm_exit_fp.get() = self.old_last_wasm_exit_fp.get();
                *(*self.limits).last_wasm_exit_pc.get() = self.old_last_wasm_exit_pc.get();
                *(*self.limits).last_wasm_entry_fp.get() = self.old_last_wasm_entry_fp.get();
            }
        }
    }

    impl CallThreadState {
        pub const JMP_BUF_INTERPRETER_SENTINEL: *mut u8 = 1 as *mut u8;

        #[inline]
        pub(super) fn new(store: &mut StoreOpaque, caller: *mut VMContext) -> CallThreadState {
            let limits = unsafe { *Instance::from_vmctx(caller, |i| i.runtime_limits()) };

            // Don't try to plumb #[cfg] everywhere for this field, just pretend
            // we're using it on miri/windows to silence compiler warnings.
            let _: Range<_> = store.async_guard_range();

            CallThreadState {
                unwind: Cell::new(None),
                unwinder: store.unwinder(),
                jmp_buf: Cell::new(ptr::null()),
                #[cfg(all(feature = "signals-based-traps", not(miri)))]
                signal_handler: store.signal_handler(),
                capture_backtrace: store.engine().config().wasm_backtrace,
                #[cfg(feature = "coredump")]
                capture_coredump: store.engine().config().coredump_on_trap,
                limits,
                #[cfg(all(feature = "signals-based-traps", unix, not(miri)))]
                async_guard_range: store.async_guard_range(),
                prev: Cell::new(ptr::null()),
                old_last_wasm_exit_fp: Cell::new(unsafe { *(*limits).last_wasm_exit_fp.get() }),
                old_last_wasm_exit_pc: Cell::new(unsafe { *(*limits).last_wasm_exit_pc.get() }),
                old_last_wasm_entry_fp: Cell::new(unsafe { *(*limits).last_wasm_entry_fp.get() }),
            }
        }

        /// Get the saved FP upon exit from Wasm for the previous `CallThreadState`.
        pub fn old_last_wasm_exit_fp(&self) -> usize {
            self.old_last_wasm_exit_fp.get()
        }

        /// Get the saved PC upon exit from Wasm for the previous `CallThreadState`.
        pub fn old_last_wasm_exit_pc(&self) -> usize {
            self.old_last_wasm_exit_pc.get()
        }

        /// Get the saved FP upon entry into Wasm for the previous `CallThreadState`.
        pub fn old_last_wasm_entry_fp(&self) -> usize {
            self.old_last_wasm_entry_fp.get()
        }

        /// Get the previous `CallThreadState`.
        pub fn prev(&self) -> tls::Ptr {
            self.prev.get()
        }

        #[inline]
        pub(crate) unsafe fn push(&self) {
            assert!(self.prev.get().is_null());
            self.prev.set(tls::raw::replace(self));
        }

        #[inline]
        pub(crate) unsafe fn pop(&self) {
            let prev = self.prev.replace(ptr::null());
            let head = tls::raw::replace(prev);
            assert!(core::ptr::eq(head, self));
        }
    }
}
pub use call_thread_state::*;

pub enum UnwindReason {
    #[cfg(all(feature = "std", panic = "unwind"))]
    Panic(Box<dyn std::any::Any + Send>),
    Trap(TrapReason),
}

impl CallThreadState {
    #[inline]
    fn with(
        mut self,
        closure: impl FnOnce(&CallThreadState) -> bool,
    ) -> Result<(), (UnwindReason, Option<Backtrace>, Option<CoreDumpStack>)> {
        let succeeded = tls::set(&mut self, |me| closure(me));
        if succeeded {
            Ok(())
        } else {
            Err(self.read_unwind())
        }
    }

    #[cold]
    fn read_unwind(&self) -> (UnwindReason, Option<Backtrace>, Option<CoreDumpStack>) {
        self.unwind.replace(None).unwrap()
    }

    /// Records the unwind information provided within this `CallThreadState`,
    /// optionally capturing a backtrace at this time.
    ///
    /// This function is used to stash metadata for why an unwind is about to
    /// happen. The actual unwind is expected to happen after this function is
    /// called using, for example, the `unwind` function below.
    ///
    /// Note that this is a relatively low-level function and will panic if
    /// mis-used.
    ///
    /// # Panics
    ///
    /// Panics if unwind information has already been recorded as that should
    /// have been processed first.
    fn record_unwind(&self, reason: UnwindReason) {
        if cfg!(debug_assertions) {
            let prev = self.unwind.replace(None);
            assert!(prev.is_none());
        }
        let (backtrace, coredump) = match &reason {
            // Panics don't need backtraces. There is nowhere to attach the
            // hypothetical backtrace to and it doesn't really make sense to try
            // in the first place since this is a Rust problem rather than a
            // Wasm problem.
            #[cfg(all(feature = "std", panic = "unwind"))]
            UnwindReason::Panic(_) => (None, None),
            // And if we are just propagating an existing trap that already has
            // a backtrace attached to it, then there is no need to capture a
            // new backtrace either.
            UnwindReason::Trap(TrapReason::User(err))
                if err.downcast_ref::<WasmBacktrace>().is_some() =>
            {
                (None, None)
            }
            UnwindReason::Trap(_) => (
                self.capture_backtrace(self.limits, None),
                self.capture_coredump(self.limits, None),
            ),
        };
        self.unwind.set(Some((reason, backtrace, coredump)));
    }

    /// Helper function to perform an actual unwinding operation.
    ///
    /// This must be preceded by a `record_unwind` operation above to be
    /// processed correctly on the other side.
    ///
    /// # Unsafety
    ///
    /// This function is not safe if the corresponding setjmp wasn't already
    /// called. Additionally this isn't safe as it will skip all Rust
    /// destructors on the stack, if there are any.
    unsafe fn unwind(&self) -> ! {
        debug_assert!(!self.jmp_buf.get().is_null());
        debug_assert!(self.jmp_buf.get() != CallThreadState::JMP_BUF_INTERPRETER_SENTINEL);
        traphandlers::wasmtime_longjmp(self.jmp_buf.get());
    }

    fn capture_backtrace(
        &self,
        limits: *const VMRuntimeLimits,
        trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Option<Backtrace> {
        if !self.capture_backtrace {
            return None;
        }

        Some(unsafe { Backtrace::new_with_trap_state(limits, self.unwinder, self, trap_pc_and_fp) })
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Self> + 'a {
        let mut state = Some(self);
        core::iter::from_fn(move || {
            let this = state?;
            state = unsafe { this.prev().as_ref() };
            Some(this)
        })
    }

    /// Trap handler using our thread-local state.
    ///
    /// * `regs` - some special program registers at the time that the trap
    ///   happened, for example `pc`.
    /// * `faulting_addr` - the system-provided address that the a fault, if
    ///   any, happened at. This is used when debug-asserting that all segfaults
    ///   are known to live within a `Store<T>` in a valid range.
    /// * `call_handler` - a closure used to invoke the platform-specific
    ///   signal handler for each instance, if available.
    ///
    /// Attempts to handle the trap if it's a wasm trap. Returns a `TrapTest`
    /// which indicates what this could be, such as:
    ///
    /// * `TrapTest::NotWasm` - not a wasm fault, this should get forwarded to
    ///   the next platform-specific fault handler.
    /// * `TrapTest::HandledByEmbedder` - the embedder `call_handler` handled
    ///   this signal, nothing else to do.
    /// * `TrapTest::Trap` - this is a wasm trap an the stack needs to be
    ///   unwound now.
    pub(crate) fn test_if_trap(
        &self,
        regs: TrapRegisters,
        faulting_addr: Option<usize>,
        call_handler: impl Fn(&SignalHandler) -> bool,
    ) -> TrapTest {
        // If we haven't even started to handle traps yet, bail out.
        if self.jmp_buf.get().is_null() {
            return TrapTest::NotWasm;
        }

        // First up see if any instance registered has a custom trap handler,
        // in which case run them all. If anything handles the trap then we
        // return that the trap was handled.
        let _ = &call_handler;
        #[cfg(all(feature = "signals-based-traps", not(miri)))]
        if let Some(handler) = self.signal_handler {
            if unsafe { call_handler(&*handler) } {
                return TrapTest::HandledByEmbedder;
            }
        }

        // If this fault wasn't in wasm code, then it's not our problem
        let Some((code, text_offset)) = lookup_code(regs.pc) else {
            return TrapTest::NotWasm;
        };

        // If the fault was at a location that was not marked as potentially
        // trapping, then that's a bug in Cranelift/Winch/etc. Don't try to
        // catch the trap and pretend this isn't wasm so the program likely
        // aborts.
        let Some(trap) = code.lookup_trap_code(text_offset) else {
            return TrapTest::NotWasm;
        };

        // If all that passed then this is indeed a wasm trap, so return the
        // `jmp_buf` passed to `wasmtime_longjmp` to resume.
        self.set_jit_trap(regs, faulting_addr, trap);
        TrapTest::Trap {
            jmp_buf: self.take_jmp_buf(),
        }
    }

    pub(crate) fn take_jmp_buf(&self) -> *const u8 {
        self.jmp_buf.replace(ptr::null())
    }

    pub(crate) fn set_jit_trap(
        &self,
        TrapRegisters { pc, fp, .. }: TrapRegisters,
        faulting_addr: Option<usize>,
        trap: wasmtime_environ::Trap,
    ) {
        let backtrace = self.capture_backtrace(self.limits, Some((pc, fp)));
        let coredump = self.capture_coredump(self.limits, Some((pc, fp)));
        self.unwind.set(Some((
            UnwindReason::Trap(TrapReason::Jit {
                pc,
                faulting_addr,
                trap,
            }),
            backtrace,
            coredump,
        )))
    }
}

// A private inner module for managing the TLS state that we require across
// calls in wasm. The WebAssembly code is called from C++ and then a trap may
// happen which requires us to read some contextual state to figure out what to
// do with the trap. This `tls` module is used to persist that information from
// the caller to the trap site.
pub(crate) mod tls {
    use super::CallThreadState;
    use core::mem;
    use core::ops::Range;

    pub use raw::Ptr;

    // An even *more* inner module for dealing with TLS. This actually has the
    // thread local variable and has functions to access the variable.
    //
    // Note that this is specially done to fully encapsulate that the accessors
    // for tls may or may not be inlined. Wasmtime's async support employs stack
    // switching which can resume execution on different OS threads. This means
    // that borrows of our TLS pointer must never live across accesses because
    // otherwise the access may be split across two threads and cause unsafety.
    //
    // This also means that extra care is taken by the runtime to save/restore
    // these TLS values when the runtime may have crossed threads.
    //
    // Note, though, that if async support is disabled at compile time then
    // these functions are free to be inlined.
    pub(super) mod raw {
        use super::CallThreadState;
        use sptr::Strict;

        pub type Ptr = *const CallThreadState;

        const _: () = {
            assert!(core::mem::align_of::<CallThreadState>() > 1);
        };

        fn tls_get() -> (Ptr, bool) {
            let mut initialized = false;
            let p = Strict::map_addr(crate::runtime::vm::sys::tls_get(), |a| {
                initialized = (a & 1) != 0;
                a & !1
            });
            (p.cast(), initialized)
        }

        fn tls_set(ptr: Ptr, initialized: bool) {
            let encoded = Strict::map_addr(ptr, |a| a | usize::from(initialized));
            crate::runtime::vm::sys::tls_set(encoded.cast_mut().cast::<u8>());
        }

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn replace(val: Ptr) -> Ptr {
            // When a new value is configured that means that we may be
            // entering WebAssembly so check to see if this thread has
            // performed per-thread initialization for traps.
            let (prev, initialized) = tls_get();
            if !initialized {
                super::super::lazy_per_thread_init();
            }
            tls_set(val, true);
            prev
        }

        /// Eagerly initialize thread-local runtime functionality. This will be performed
        /// lazily by the runtime if users do not perform it eagerly.
        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn initialize() {
            let (state, initialized) = tls_get();
            if initialized {
                return;
            }
            super::super::lazy_per_thread_init();
            tls_set(state, true);
        }

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn get() -> Ptr {
            tls_get().0
        }
    }

    pub use raw::initialize as tls_eager_initialize;

    /// Opaque state used to persist the state of the `CallThreadState`
    /// activations associated with a fiber stack that's used as part of an
    /// async wasm call.
    pub struct AsyncWasmCallState {
        // The head of a linked list of activations that are currently present
        // on an async call's fiber stack. This pointer points to the oldest
        // activation frame where the `prev` links internally link to younger
        // activation frames.
        //
        // When pushed onto a thread this linked list is traversed to get pushed
        // onto the current thread at the time.
        state: raw::Ptr,
    }

    impl AsyncWasmCallState {
        /// Creates new state that initially starts as null.
        pub fn new() -> AsyncWasmCallState {
            AsyncWasmCallState {
                state: core::ptr::null_mut(),
            }
        }

        /// Pushes the saved state of this wasm's call onto the current thread's
        /// state.
        ///
        /// This will iterate over the linked list of states stored within
        /// `self` and push them sequentially onto the current thread's
        /// activation list.
        ///
        /// The returned `PreviousAsyncWasmCallState` captures the state of this
        /// thread just before this operation, and it must have its `restore`
        /// method called to restore the state when the async wasm is suspended
        /// from.
        ///
        /// # Unsafety
        ///
        /// Must be carefully coordinated with
        /// `PreviousAsyncWasmCallState::restore` and fiber switches to ensure
        /// that this doesn't push stale data and the data is popped
        /// appropriately.
        pub unsafe fn push(self) -> PreviousAsyncWasmCallState {
            // Our `state` pointer is a linked list of oldest-to-youngest so by
            // pushing in order of the list we restore the youngest-to-oldest
            // list as stored in the state of this current thread.
            let ret = PreviousAsyncWasmCallState { state: raw::get() };
            let mut ptr = self.state;
            while let Some(state) = ptr.as_ref() {
                ptr = state.prev.replace(core::ptr::null_mut());
                state.push();
            }
            ret
        }

        /// Performs a runtime check that this state is indeed null.
        pub fn assert_null(&self) {
            assert!(self.state.is_null());
        }

        /// Asserts that the current CallThreadState pointer, if present, is not
        /// in the `range` specified.
        ///
        /// This is used when exiting a future in Wasmtime to assert that the
        /// current CallThreadState pointer does not point within the stack
        /// we're leaving (e.g.  allocated for a fiber).
        pub fn assert_current_state_not_in_range(range: Range<usize>) {
            let p = raw::get() as usize;
            assert!(p < range.start || range.end < p);
        }
    }

    /// Opaque state used to help control TLS state across stack switches for
    /// async support.
    pub struct PreviousAsyncWasmCallState {
        // The head of a linked list, similar to the TLS state. Note though that
        // this list is stored in reverse order to assist with `push` and `pop`
        // below.
        //
        // After a `push` call this stores the previous head for the current
        // thread so we know when to stop popping during a `pop`.
        state: raw::Ptr,
    }

    impl PreviousAsyncWasmCallState {
        /// Pops a fiber's linked list of activations and stores them in
        /// `AsyncWasmCallState`.
        ///
        /// This will pop the top activation of this current thread continuously
        /// until it reaches whatever the current activation was when `push` was
        /// originally called.
        ///
        /// # Unsafety
        ///
        /// Must be paired with a `push` and only performed at a time when a
        /// fiber is being suspended.
        pub unsafe fn restore(self) -> AsyncWasmCallState {
            let thread_head = self.state;
            mem::forget(self);
            let mut ret = AsyncWasmCallState::new();
            loop {
                // If the current TLS state is as we originally found it, then
                // this loop is finished.
                let ptr = raw::get();
                if ptr == thread_head {
                    break ret;
                }

                // Pop this activation from the current thread's TLS state, and
                // then afterwards push it onto our own linked list within this
                // `AsyncWasmCallState`. Note that the linked list in `AsyncWasmCallState` is stored
                // in reverse order so a subsequent `push` later on pushes
                // everything in the right order.
                (*ptr).pop();
                if let Some(state) = ret.state.as_ref() {
                    (*ptr).prev.set(state);
                }
                ret.state = ptr;
            }
        }
    }

    impl Drop for PreviousAsyncWasmCallState {
        fn drop(&mut self) {
            panic!("must be consumed with `restore`");
        }
    }

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `state`, unless
    /// this is recursively called again.
    #[inline]
    pub fn set<R>(state: &mut CallThreadState, closure: impl FnOnce(&CallThreadState) -> R) -> R {
        struct Reset<'a> {
            state: &'a CallThreadState,
        }

        impl Drop for Reset<'_> {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    self.state.pop();
                }
            }
        }

        unsafe {
            state.push();
            let reset = Reset { state };
            closure(reset.state)
        }
    }

    /// Returns the last pointer configured with `set` above, if any.
    pub fn with<R>(closure: impl FnOnce(Option<&CallThreadState>) -> R) -> R {
        let p = raw::get();
        unsafe { closure(if p.is_null() { None } else { Some(&*p) }) }
    }
}
