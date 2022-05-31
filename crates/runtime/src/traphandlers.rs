//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::VMContext;
use anyhow::Error;
use std::any::Any;
use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::Once;
use wasmtime_environ::TrapCode;

pub use self::tls::{tls_eager_initialize, TlsRestore};
pub use backtrace::Backtrace;

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
pub unsafe fn raise_user_trap(data: Error) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::UserTrap(data)))
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
pub unsafe fn raise_lib_trap(trap: Trap) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::LibTrap(trap)))
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
pub enum Trap {
    /// A user-raised trap through `raise_user_trap`.
    User {
        /// The user-provided error
        error: Error,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Option<Backtrace>,
    },

    /// A trap raised from jit code
    Jit {
        /// The program counter in JIT code where this trap happened.
        pc: usize,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Option<Backtrace>,
    },

    /// A trap raised from a wasm libcall
    Wasm {
        /// Code of the trap.
        trap_code: TrapCode,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Option<Backtrace>,
    },

    /// A trap indicating that the runtime was unable to allocate sufficient memory.
    OOM {
        /// Native stack backtrace at the time the OOM occurred
        backtrace: Option<Backtrace>,
    },
}

impl Trap {
    /// Construct a new Wasm trap with the given trap code.
    ///
    /// Internally saves a backtrace when passed across a setjmp boundary, if the
    /// engine is configured to save backtraces.
    pub fn wasm(trap_code: TrapCode) -> Self {
        Trap::Wasm {
            trap_code,
            backtrace: None,
        }
    }

    /// Construct a new Wasm trap from a user Error.
    ///
    /// Internally saves a backtrace when passed across a setjmp boundary, if the
    /// engine is configured to save backtraces.
    pub fn user(error: Error) -> Self {
        Trap::User {
            error,
            backtrace: None,
        }
    }
    /// Construct a new OOM trap.
    ///
    /// Internally saves a backtrace when passed across a setjmp boundary, if the
    /// engine is configured to save backtraces.
    pub fn oom() -> Self {
        Trap::OOM { backtrace: None }
    }

    fn insert_backtrace(&mut self, bt: Backtrace) {
        match self {
            Trap::User { backtrace, .. } => *backtrace = Some(bt),
            Trap::Jit { backtrace, .. } => *backtrace = Some(bt),
            Trap::Wasm { backtrace, .. } => *backtrace = Some(bt),
            Trap::OOM { backtrace, .. } => *backtrace = Some(bt),
        }
    }
}

/// Catches any wasm traps that happen within the execution of `closure`,
/// returning them as a `Result`.
///
/// Highly unsafe since `closure` won't have any dtors run.
pub unsafe fn catch_traps<'a, F>(
    signal_handler: Option<*const SignalHandler<'static>>,
    capture_backtrace: bool,
    callee: *mut VMContext,
    mut closure: F,
) -> Result<(), Box<Trap>>
where
    F: FnMut(*mut VMContext),
{
    return CallThreadState::new(signal_handler, capture_backtrace).with(|cx| {
        wasmtime_setjmp(
            cx.jmp_buf.as_ptr(),
            call_closure::<F>,
            &mut closure as *mut F as *mut u8,
            callee,
        )
    });

    extern "C" fn call_closure<F>(payload: *mut u8, callee: *mut VMContext)
    where
        F: FnMut(*mut VMContext),
    {
        unsafe { (*(payload as *mut F))(callee) }
    }
}

/// Temporary state stored on the stack which is registered in the `tls` module
/// below for calls into wasm.
pub struct CallThreadState {
    unwind: UnsafeCell<MaybeUninit<(UnwindReason, Option<Backtrace>)>>,
    jmp_buf: Cell<*const u8>,
    handling_trap: Cell<bool>,
    signal_handler: Option<*const SignalHandler<'static>>,
    prev: Cell<tls::Ptr>,
    capture_backtrace: bool,
}

enum UnwindReason {
    Panic(Box<dyn Any + Send>),
    UserTrap(Error),
    LibTrap(Trap),
    JitTrap { pc: usize }, // Removed a backtrace here
}

impl CallThreadState {
    #[inline]
    fn new(
        signal_handler: Option<*const SignalHandler<'static>>,
        capture_backtrace: bool,
    ) -> CallThreadState {
        CallThreadState {
            unwind: UnsafeCell::new(MaybeUninit::uninit()),
            jmp_buf: Cell::new(ptr::null()),
            handling_trap: Cell::new(false),
            signal_handler,
            prev: Cell::new(ptr::null()),
            capture_backtrace,
        }
    }

    fn with(self, closure: impl FnOnce(&CallThreadState) -> i32) -> Result<(), Box<Trap>> {
        let ret = tls::set(&self, || closure(&self))?;
        if ret != 0 {
            Ok(())
        } else {
            Err(unsafe { self.read_trap() })
        }
    }

    #[cold]
    unsafe fn read_trap(&self) -> Box<Trap> {
        Box::new(match (*self.unwind.get()).as_ptr().read() {
            (UnwindReason::UserTrap(error), backtrace) => Trap::User { error, backtrace },
            (UnwindReason::LibTrap(mut trap), backtrace) => {
                if let Some(backtrace) = backtrace {
                    trap.insert_backtrace(backtrace);
                }
                trap
            }
            (UnwindReason::JitTrap { pc }, backtrace) => Trap::Jit { pc, backtrace },
            (UnwindReason::Panic(panic), _) => std::panic::resume_unwind(panic),
        })
    }

    fn unwind_with(&self, reason: UnwindReason) -> ! {
        let backtrace = if self.capture_backtrace {
            Some(Backtrace::new_unresolved())
        } else {
            None
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
    fn jmp_buf_if_trap(
        &self,
        pc: *const u8,
        call_handler: impl Fn(&SignalHandler) -> bool,
    ) -> *const u8 {
        // If we hit a fault while handling a previous trap, that's quite bad,
        // so bail out and let the system handle this recursive segfault.
        //
        // Otherwise flag ourselves as handling a trap, do the trap handling,
        // and reset our trap handling flag.
        if self.handling_trap.replace(true) {
            return ptr::null();
        }
        let _reset = ResetCell(&self.handling_trap, false);

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
        self.jmp_buf.get()
    }

    fn capture_backtrace(&self, pc: *const u8) {
        let backtrace = if self.capture_backtrace {
            Some(Backtrace::new_unresolved())
        } else {
            None
        };
        unsafe {
            (*self.unwind.get())
                .as_mut_ptr()
                .write((UnwindReason::JitTrap { pc: pc as usize }, backtrace));
        }
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
    use crate::Trap;
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
        use crate::Trap;
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
        pub fn replace(val: Ptr) -> Result<Ptr, Box<Trap>> {
            PTR.with(|p| {
                // When a new value is configured that means that we may be
                // entering WebAssembly so check to see if this thread has
                // performed per-thread initialization for traps.
                let (prev, initialized) = p.get();
                if !initialized {
                    super::super::sys::lazy_per_thread_init()?;
                }
                p.set((val, true));
                Ok(prev)
            })
        }

        /// Eagerly initialize thread-local runtime functionality. This will be performed
        /// lazily by the runtime if users do not perform it eagerly.
        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn initialize() -> Result<(), Box<Trap>> {
            PTR.with(|p| {
                let (state, initialized) = p.get();
                if initialized {
                    return Ok(());
                }
                super::super::sys::lazy_per_thread_init()?;
                p.set((state, true));
                Ok(())
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
    pub struct TlsRestore(raw::Ptr);

    impl TlsRestore {
        /// Takes the TLS state that is currently configured and returns a
        /// token that is used to replace it later.
        ///
        /// This is not a safe operation since it's intended to only be used
        /// with stack switching found with fibers and async wasmtime.
        pub unsafe fn take() -> Result<TlsRestore, Box<Trap>> {
            // Our tls pointer must be set at this time, and it must not be
            // null. We need to restore the previous pointer since we're
            // removing ourselves from the call-stack, and in the process we
            // null out our own previous field for safety in case it's
            // accidentally used later.
            let raw = raw::get();
            if !raw.is_null() {
                let prev = (*raw).prev.replace(ptr::null());
                raw::replace(prev)?;
            }
            // Null case: we aren't in a wasm context, so theres no tls
            // to save for restoration.
            Ok(TlsRestore(raw))
        }

        /// Restores a previous tls state back into this thread's TLS.
        ///
        /// This is unsafe because it's intended to only be used within the
        /// context of stack switching within wasmtime.
        pub unsafe fn replace(self) -> Result<(), Box<super::Trap>> {
            // Null case: we aren't in a wasm context, so theres no tls
            // to restore.
            if self.0.is_null() {
                return Ok(());
            }
            // We need to configure our previous TLS pointer to whatever is in
            // TLS at this time, and then we set the current state to ourselves.
            let prev = raw::get();
            assert!((*self.0).prev.get().is_null());
            (*self.0).prev.set(prev);
            raw::replace(self.0)?;
            Ok(())
        }
    }

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `ptr`, unless this
    /// is recursively called again.
    #[inline]
    pub fn set<R>(state: &CallThreadState, closure: impl FnOnce() -> R) -> Result<R, Box<Trap>> {
        struct Reset<'a>(&'a CallThreadState);

        impl Drop for Reset<'_> {
            #[inline]
            fn drop(&mut self) {
                raw::replace(self.0.prev.replace(ptr::null()))
                    .expect("tls should be previously initialized");
            }
        }

        let prev = raw::replace(state)?;
        state.prev.set(prev);
        let _reset = Reset(state);
        Ok(closure())
    }

    /// Returns the last pointer configured with `set` above. Panics if `set`
    /// has not been previously called.
    pub fn with<R>(closure: impl FnOnce(Option<&CallThreadState>) -> R) -> R {
        let p = raw::get();
        unsafe { closure(if p.is_null() { None } else { Some(&*p) }) }
    }
}
