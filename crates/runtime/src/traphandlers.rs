//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

mod backtrace;

use crate::{VMContext, VMRuntimeLimits};
use anyhow::Error;
use std::any::Any;
use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::Once;

pub use self::backtrace::Backtrace;
pub use self::tls::{tls_eager_initialize, TlsRestore};

#[link(name = "wasmtime-helpers")]
extern "C" {
    #[allow(improper_ctypes)]
    fn wasmtime_setjmp(
        jmp_buf: *mut *const u8,
        callback: extern "C" fn(*mut u8, *mut VMContext),
        payload: *mut u8,
        callee: *mut VMContext,
    ) -> i32;
    fn wasmtime_longjmp(jmp_buf: *const u8) -> !;
}

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "macos", not(feature = "posix-signals-on-macos")))] {
        mod macos;
        use macos as sys;
    } else if #[cfg(unix)] {
        mod unix;
        use unix as sys;
    } else if #[cfg(target_os = "windows")] {
        mod windows;
        use windows as sys;
    }
}

pub use sys::SignalHandler;

/// Globally-set callback to determine whether a program counter is actually a
/// wasm trap.
///
/// This is initialized during `init_traps` below. The definition lives within
/// `wasmtime` currently.
static mut IS_WASM_PC: fn(usize) -> bool = |_| false;

/// This function is required to be called before any WebAssembly is entered.
/// This will configure global state such as signal handlers to prepare the
/// process to receive wasm traps.
///
/// This function must not only be called globally once before entering
/// WebAssembly but it must also be called once-per-thread that enters
/// WebAssembly. Currently in wasmtime's integration this function is called on
/// creation of a `Engine`.
///
/// The `is_wasm_pc` argument is used when a trap happens to determine if a
/// program counter is the pc of an actual wasm trap or not. This is then used
/// to disambiguate faults that happen due to wasm and faults that happen due to
/// bugs in Rust or elsewhere.
pub fn init_traps(is_wasm_pc: fn(usize) -> bool) {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        IS_WASM_PC = is_wasm_pc;
        sys::platform_init();
    });
}

/// Raises a trap immediately.
///
/// This function performs as-if a wasm trap was just executed. This trap
/// payload is then returned from `catch_traps` below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn raise_trap(reason: TrapReason) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::Trap(reason)))
}

/// Raises a user-defined trap immediately.
///
/// This function performs as-if a wasm trap was just executed, only the trap
/// has a dynamic payload associated with it which is user-provided. This trap
/// payload is then returned from `catch_traps` below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn raise_user_trap(error: Error, needs_backtrace: bool) -> ! {
    raise_trap(TrapReason::User {
        error,
        needs_backtrace,
    })
}

/// Raises a trap from inside library code immediately.
///
/// This function performs as-if a wasm trap was just executed. This trap
/// payload is then returned from `catch_traps` below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn raise_lib_trap(trap: wasmtime_environ::Trap) -> ! {
    raise_trap(TrapReason::Wasm(trap))
}

/// Carries a Rust panic across wasm code and resumes the panic on the other
/// side.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn resume_panic(payload: Box<dyn Any + Send>) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::Panic(payload)))
}

/// Stores trace message with backtrace.
#[derive(Debug)]
pub struct Trap {
    /// Original reason from where this trap originated.
    pub reason: TrapReason,
    /// Wasm backtrace of the trap, if any.
    pub backtrace: Option<Backtrace>,
}

/// Enumeration of different methods of raising a trap.
#[derive(Debug)]
pub enum TrapReason {
    /// A user-raised trap through `raise_user_trap`.
    User {
        /// The actual user trap error.
        error: Error,
        /// Whether we need to capture a backtrace for this error or not.
        needs_backtrace: bool,
    },

    /// A trap raised from Cranelift-generated code with the pc listed of where
    /// the trap came from.
    Jit(usize),

    /// A trap raised from a wasm libcall
    Wasm(wasmtime_environ::Trap),
}

impl TrapReason {
    /// Create a new `TrapReason::User` that does not have a backtrace yet.
    pub fn user_without_backtrace(error: Error) -> Self {
        TrapReason::User {
            error,
            needs_backtrace: true,
        }
    }

    /// Create a new `TrapReason::User` that already has a backtrace.
    pub fn user_with_backtrace(error: Error) -> Self {
        TrapReason::User {
            error,
            needs_backtrace: false,
        }
    }

    /// Is this a JIT trap?
    pub fn is_jit(&self) -> bool {
        matches!(self, TrapReason::Jit(_))
    }
}

impl From<Error> for TrapReason {
    fn from(err: Error) -> Self {
        TrapReason::user_without_backtrace(err)
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
/// Highly unsafe since `closure` won't have any dtors run.
pub unsafe fn catch_traps<'a, F>(
    signal_handler: Option<*const SignalHandler<'static>>,
    capture_backtrace: bool,
    caller: *mut VMContext,
    mut closure: F,
) -> Result<(), Box<Trap>>
where
    F: FnMut(*mut VMContext),
{
    let limits = (*caller).instance_mut().runtime_limits();

    let result = CallThreadState::new(signal_handler, capture_backtrace, *limits).with(|cx| {
        wasmtime_setjmp(
            cx.jmp_buf.as_ptr(),
            call_closure::<F>,
            &mut closure as *mut F as *mut u8,
            caller,
        )
    });

    return match result {
        Ok(x) => Ok(x),
        Err((UnwindReason::Trap(reason), backtrace)) => Err(Box::new(Trap { reason, backtrace })),
        Err((UnwindReason::Panic(panic), _)) => std::panic::resume_unwind(panic),
    };

    extern "C" fn call_closure<F>(payload: *mut u8, caller: *mut VMContext)
    where
        F: FnMut(*mut VMContext),
    {
        unsafe { (*(payload as *mut F))(caller) }
    }
}

// Module to hide visibility of the `CallThreadState::prev` field and force
// usage of its accessor methods.
mod call_thread_state {
    use super::*;
    use std::mem;

    /// Temporary state stored on the stack which is registered in the `tls` module
    /// below for calls into wasm.
    pub struct CallThreadState {
        pub(super) unwind: UnsafeCell<MaybeUninit<(UnwindReason, Option<Backtrace>)>>,
        pub(super) jmp_buf: Cell<*const u8>,
        pub(super) signal_handler: Option<*const SignalHandler<'static>>,
        pub(super) capture_backtrace: bool,

        pub(crate) limits: *const VMRuntimeLimits,

        prev: Cell<tls::Ptr>,

        // The values of `VMRuntimeLimits::last_wasm_{exit_{pc,fp},entry_sp}` for
        // the *previous* `CallThreadState`. Our *current* last wasm PC/FP/SP are
        // saved in `self.limits`. We save a copy of the old registers here because
        // the `VMRuntimeLimits` typically doesn't change across nested calls into
        // Wasm (i.e. they are typically calls back into the same store and
        // `self.limits == self.prev.limits`) and we must to maintain the list of
        // contiguous-Wasm-frames stack regions for backtracing purposes.
        old_last_wasm_exit_fp: Cell<usize>,
        old_last_wasm_exit_pc: Cell<usize>,
        old_last_wasm_entry_sp: Cell<usize>,
    }

    impl CallThreadState {
        #[inline]
        pub(super) fn new(
            signal_handler: Option<*const SignalHandler<'static>>,
            capture_backtrace: bool,
            limits: *const VMRuntimeLimits,
        ) -> CallThreadState {
            CallThreadState {
                unwind: UnsafeCell::new(MaybeUninit::uninit()),
                jmp_buf: Cell::new(ptr::null()),
                signal_handler,
                capture_backtrace,
                limits,
                prev: Cell::new(ptr::null()),
                old_last_wasm_exit_fp: Cell::new(0),
                old_last_wasm_exit_pc: Cell::new(0),
                old_last_wasm_entry_sp: Cell::new(0),
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

        /// Get the saved SP upon entry into Wasm for the previous `CallThreadState`.
        pub fn old_last_wasm_entry_sp(&self) -> usize {
            self.old_last_wasm_entry_sp.get()
        }

        /// Get the previous `CallThreadState`.
        pub fn prev(&self) -> tls::Ptr {
            self.prev.get()
        }

        /// Connect the link to the previous `CallThreadState`.
        ///
        /// Synchronizes the last wasm FP, PC, and SP on `self` and the old
        /// `self.prev` for the given new `prev`, and returns the old
        /// `self.prev`.
        pub unsafe fn set_prev(&self, prev: tls::Ptr) -> tls::Ptr {
            let old_prev = self.prev.get();

            // Restore the old `prev`'s saved registers in its
            // `VMRuntimeLimits`. This is necessary for when we are async
            // suspending the top `CallThreadState` and doing `set_prev(null)`
            // on it, and so any stack walking we do subsequently will start at
            // the old `prev` and look at its `VMRuntimeLimits` to get the
            // initial saved registers.
            if let Some(old_prev) = old_prev.as_ref() {
                *(*old_prev.limits).last_wasm_exit_fp.get() = self.old_last_wasm_exit_fp();
                *(*old_prev.limits).last_wasm_exit_pc.get() = self.old_last_wasm_exit_pc();
                *(*old_prev.limits).last_wasm_entry_sp.get() = self.old_last_wasm_entry_sp();
            }

            self.prev.set(prev);

            let mut old_last_wasm_exit_fp = 0;
            let mut old_last_wasm_exit_pc = 0;
            let mut old_last_wasm_entry_sp = 0;
            if let Some(prev) = prev.as_ref() {
                // We are entering a new `CallThreadState` or resuming a
                // previously suspended one. This means we will push new Wasm
                // frames that save the new Wasm FP/SP/PC registers into
                // `VMRuntimeLimits`, we need to first save the old Wasm
                // FP/SP/PC registers into this new `CallThreadState` to
                // maintain our list of contiguous Wasm frame regions that we
                // use when capturing stack traces.
                //
                // NB: the Wasm<--->host trampolines saved the Wasm FP/SP/PC
                // registers in the active-at-that-time store's
                // `VMRuntimeLimits`. For the most recent FP/PC/SP that is the
                // `state.prev.limits` (since we haven't entered this
                // `CallThreadState` yet). And that can be a different
                // `VMRuntimeLimits` instance from the currently active
                // `state.limits`, which will be used by the upcoming call into
                // Wasm! Consider the case where we have multiple, nested calls
                // across stores (with host code in between, by necessity, since
                // only things in the same store can be linked directly
                // together):
                //
                //     | ...             |
                //     | Host            |  |
                //     +-----------------+  | stack
                //     | Wasm in store A |  | grows
                //     +-----------------+  | down
                //     | Host            |  |
                //     +-----------------+  |
                //     | Wasm in store B |  V
                //     +-----------------+
                //
                // In this scenario `state.limits != state.prev.limits`,
                // i.e. `B.limits != A.limits`! Therefore we must take care to
                // read the old FP/SP/PC from `state.prev.limits`, rather than
                // `state.limits`, and store those saved registers into the
                // current `state`.
                //
                // See also the comment above the
                // `CallThreadState::old_last_wasm_*` fields.
                old_last_wasm_exit_fp =
                    mem::replace(&mut *(*prev.limits).last_wasm_exit_fp.get(), 0);
                old_last_wasm_exit_pc =
                    mem::replace(&mut *(*prev.limits).last_wasm_exit_pc.get(), 0);
                old_last_wasm_entry_sp =
                    mem::replace(&mut *(*prev.limits).last_wasm_entry_sp.get(), 0);
            }

            self.old_last_wasm_exit_fp.set(old_last_wasm_exit_fp);
            self.old_last_wasm_exit_pc.set(old_last_wasm_exit_pc);
            self.old_last_wasm_entry_sp.set(old_last_wasm_entry_sp);

            old_prev
        }
    }
}
pub use call_thread_state::*;

enum UnwindReason {
    Panic(Box<dyn Any + Send>),
    Trap(TrapReason),
}

impl CallThreadState {
    fn with(
        mut self,
        closure: impl FnOnce(&CallThreadState) -> i32,
    ) -> Result<(), (UnwindReason, Option<Backtrace>)> {
        let ret = tls::set(&mut self, |me| closure(me));
        if ret != 0 {
            Ok(())
        } else {
            Err(unsafe { self.read_unwind() })
        }
    }

    #[cold]
    unsafe fn read_unwind(&self) -> (UnwindReason, Option<Backtrace>) {
        (*self.unwind.get()).as_ptr().read()
    }

    fn unwind_with(&self, reason: UnwindReason) -> ! {
        let backtrace = match reason {
            // Panics don't need backtraces. There is nowhere to attach the
            // hypothetical backtrace to and it doesn't really make sense to try
            // in the first place since this is a Rust problem rather than a
            // Wasm problem.
            UnwindReason::Panic(_)
            // And if we are just propagating an existing trap that already has
            // a backtrace attached to it, then there is no need to capture a
            // new backtrace either.
            | UnwindReason::Trap(TrapReason::User {
                needs_backtrace: false,
                ..
            }) => None,
            UnwindReason::Trap(_) => self.capture_backtrace(None),
        };
        unsafe {
            (*self.unwind.get()).as_mut_ptr().write((reason, backtrace));
            wasmtime_longjmp(self.jmp_buf.get());
        }
    }

    /// Trap handler using our thread-local state.
    ///
    /// * `pc` - the program counter the trap happened at
    /// * `call_handler` - a closure used to invoke the platform-specific
    ///   signal handler for each instance, if available.
    ///
    /// Attempts to handle the trap if it's a wasm trap. Returns a few
    /// different things:
    ///
    /// * null - the trap didn't look like a wasm trap and should continue as a
    ///   trap
    /// * 1 as a pointer - the trap was handled by a custom trap handler on an
    ///   instance, and the trap handler should quickly return.
    /// * a different pointer - a jmp_buf buffer to longjmp to, meaning that
    ///   the wasm trap was succesfully handled.
    #[cfg_attr(target_os = "macos", allow(dead_code))] // macOS is more raw and doesn't use this
    fn take_jmp_buf_if_trap(
        &self,
        pc: *const u8,
        call_handler: impl Fn(&SignalHandler) -> bool,
    ) -> *const u8 {
        // If we haven't even started to handle traps yet, bail out.
        if self.jmp_buf.get().is_null() {
            return ptr::null();
        }

        // First up see if any instance registered has a custom trap handler,
        // in which case run them all. If anything handles the trap then we
        // return that the trap was handled.
        if let Some(handler) = self.signal_handler {
            if unsafe { call_handler(&*handler) } {
                return 1 as *const _;
            }
        }

        // If this fault wasn't in wasm code, then it's not our problem
        if unsafe { !IS_WASM_PC(pc as usize) } {
            return ptr::null();
        }

        // If all that passed then this is indeed a wasm trap, so return the
        // `jmp_buf` passed to `wasmtime_longjmp` to resume.
        self.jmp_buf.replace(ptr::null())
    }

    fn set_jit_trap(&self, pc: *const u8, fp: usize) {
        let backtrace = self.capture_backtrace(Some((pc as usize, fp)));
        unsafe {
            (*self.unwind.get())
                .as_mut_ptr()
                .write((UnwindReason::Trap(TrapReason::Jit(pc as usize)), backtrace));
        }
    }

    fn capture_backtrace(&self, pc_and_fp: Option<(usize, usize)>) -> Option<Backtrace> {
        if !self.capture_backtrace {
            return None;
        }

        Some(unsafe { Backtrace::new_with_trap_state(self, pc_and_fp) })
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = &Self> + 'a {
        let mut state = Some(self);
        std::iter::from_fn(move || {
            let this = state?;
            state = unsafe { this.prev().as_ref() };
            Some(this)
        })
    }
}

struct ResetCell<'a, T: Copy>(&'a Cell<T>, T);

impl<T: Copy> Drop for ResetCell<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

// A private inner module for managing the TLS state that we require across
// calls in wasm. The WebAssembly code is called from C++ and then a trap may
// happen which requires us to read some contextual state to figure out what to
// do with the trap. This `tls` module is used to persist that information from
// the caller to the trap site.
mod tls {
    use super::CallThreadState;
    use std::ptr;

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
    mod raw {
        use super::CallThreadState;
        use std::cell::Cell;
        use std::ptr;

        pub type Ptr = *const CallThreadState;

        // The first entry here is the `Ptr` which is what's used as part of the
        // public interface of this module. The second entry is a boolean which
        // allows the runtime to perform per-thread initialization if necessary
        // for handling traps (e.g. setting up ports on macOS and sigaltstack on
        // Unix).
        thread_local!(static PTR: Cell<(Ptr, bool)> = const { Cell::new((ptr::null(), false)) });

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn replace(val: Ptr) -> Ptr {
            PTR.with(|p| {
                // When a new value is configured that means that we may be
                // entering WebAssembly so check to see if this thread has
                // performed per-thread initialization for traps.
                let (prev, initialized) = p.get();
                if !initialized {
                    super::super::sys::lazy_per_thread_init();
                }
                p.set((val, true));
                prev
            })
        }

        /// Eagerly initialize thread-local runtime functionality. This will be performed
        /// lazily by the runtime if users do not perform it eagerly.
        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn initialize() {
            PTR.with(|p| {
                let (state, initialized) = p.get();
                if initialized {
                    return;
                }
                super::super::sys::lazy_per_thread_init();
                p.set((state, true));
            })
        }

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn get() -> Ptr {
            PTR.with(|p| p.get().0)
        }
    }

    pub use raw::initialize as tls_eager_initialize;

    /// Opaque state used to help control TLS state across stack switches for
    /// async support.
    pub struct TlsRestore {
        state: raw::Ptr,
    }

    impl TlsRestore {
        /// Takes the TLS state that is currently configured and returns a
        /// token that is used to replace it later.
        ///
        /// This is not a safe operation since it's intended to only be used
        /// with stack switching found with fibers and async wasmtime.
        pub unsafe fn take() -> TlsRestore {
            // Our tls pointer must be set at this time, and it must not be
            // null. We need to restore the previous pointer since we're
            // removing ourselves from the call-stack, and in the process we
            // null out our own previous field for safety in case it's
            // accidentally used later.
            let state = raw::get();
            if let Some(state) = state.as_ref() {
                let prev_state = state.set_prev(ptr::null());
                raw::replace(prev_state);
            } else {
                // Null case: we aren't in a wasm context, so theres no tls to
                // save for restoration.
            }

            TlsRestore { state }
        }

        /// Restores a previous tls state back into this thread's TLS.
        ///
        /// This is unsafe because it's intended to only be used within the
        /// context of stack switching within wasmtime.
        pub unsafe fn replace(self) {
            // Null case: we aren't in a wasm context, so theres no tls
            // to restore.
            if self.state.is_null() {
                return;
            }

            // We need to configure our previous TLS pointer to whatever is in
            // TLS at this time, and then we set the current state to ourselves.
            let prev = raw::get();
            assert!((*self.state).prev().is_null());
            (*self.state).set_prev(prev);
            raw::replace(self.state);
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
                    let prev = self.state.set_prev(ptr::null());
                    let old_state = raw::replace(prev);
                    debug_assert!(std::ptr::eq(old_state, self.state));
                }
            }
        }

        let prev = raw::replace(state);

        unsafe {
            state.set_prev(prev);

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
